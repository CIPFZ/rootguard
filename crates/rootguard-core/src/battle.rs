//! 对局的开始 / 结束与实体构造（SPEC 3.4 / 5 / 7 / 8）。

use trellia_core::hecs::EntityBuilder;
use trellia_core::{Entity, GameRng, Timer, Transform2D, Vec2, World};

use crate::components::{
    BattleTag, Biter, Blast, Carrier, Cell, ChompState, Chomper, DefenderRef, EnemyRef, Gust,
    Health, Heavy, Lane, Mine, NotBlocking, OverlayOf, Placeholder, Producer, ProjectileBody,
    ProjectileRef, Shooter, SpawnedInWave, Spikes, Sprinter, StatusEffects, Vaulter, WalkState,
    Walker,
};
use crate::data::{Behavior, CardRule, DefenderDef, EnemyDef, ProjectileDef, Rules, Trait};
use crate::events::EnemySpawned;
use crate::grid::Grid;
use crate::ids::{DefenderId, LevelId};
use crate::registry::{DefenderRegistry, EnemyRegistry, LevelRegistry};
use crate::resources::{
    BattleState, CardSlot, CardTray, DamageQueue, DeathLog, LaneState, PendingOps, Phase, WaveState,
};

/// 防御者标签：敌人不停下啃食带此标签的防御者（SPEC 7.2 Spikes）。
pub const TAG_SPIKY: &str = "spiky";

/// 经济计时状态（SPEC 9 天空 Dew）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EconomyState {
    /// 距下一次天空 Dew 的计时器。
    pub sky_timer: Timer,
}

/// 开始对局失败。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BattleError {
    /// 关卡不存在。
    #[error("关卡不存在: {0}")]
    UnknownLevel(LevelId),
    /// 卡牌不存在。
    #[error("卡牌不存在: {0}")]
    UnknownCard(DefenderId),
    /// 缺少注册表或规则资源（未添加 `ContentPlugin`）。
    #[error("缺少内容资源（未添加 ContentPlugin）")]
    MissingContent,
}

/// 冷却已完成的单次计时器（SPEC 9：关卡开始时所有卡牌冷却完成）。
#[must_use]
pub fn finished_timer(duration: u32) -> Timer {
    Timer {
        remaining: 0,
        duration,
        repeating: false,
    }
}

/// 开始对局：以关卡种子重置 `GameRng`，插入 `BattleState` / `CardTray` / `WaveState` /
/// `LaneState` 等资源（SPEC 5.2）。`Forced` 关卡忽略 `cards`；`Choose` 关卡取前 `card_slots` 张。
///
/// # Errors
///
/// 关卡或卡牌不存在、内容资源缺失时返回 [`BattleError`]。
pub fn start_battle(
    world: &mut World,
    level_id: LevelId,
    cards: &[DefenderId],
) -> Result<(), BattleError> {
    let res = &world.resources;
    let (Some(levels), Some(defenders), Some(rules)) = (
        res.get::<LevelRegistry>(),
        res.get::<DefenderRegistry>(),
        res.get::<Rules>(),
    ) else {
        return Err(BattleError::MissingContent);
    };
    let level = levels
        .get(level_id)
        .ok_or(BattleError::UnknownLevel(level_id))?
        .clone();
    let rules = *rules;
    let chosen: Vec<DefenderId> = match &level.cards {
        CardRule::Forced(list) => list.clone(),
        CardRule::Choose => cards
            .iter()
            .copied()
            .take(usize::from(level.card_slots))
            .collect(),
    };
    let mut tray = CardTray::default();
    for id in chosen {
        let def = defenders.get(id).ok_or(BattleError::UnknownCard(id))?;
        tray.cards.push(CardSlot {
            defender: id,
            cooldown: finished_timer(def.cooldown_ticks),
            ready: false,
        });
    }

    end_battle(world);
    let first_delay = level.waves.first().map_or(0, |w| w.delay_ticks);
    world.resources.insert(GameRng::from_seed(level.seed));
    world.resources.insert(BattleState {
        level: level_id,
        level_tick: 0,
        dew: level.start_dew,
        phase: Phase::Intro,
        selected_card: None,
        shovel_mode: false,
        intro: Timer::once(rules.intro_ticks),
    });
    world.resources.insert(tray);
    world.resources.insert(WaveState {
        next_wave: 0,
        wave_timer: Timer::once(first_delay),
        wave_hp_total: 0,
        wave_hp_left: 0,
        all_spawned: level.waves.is_empty(),
        banner: None,
        last_spawn_tick: None,
        total_waves: level.waves.len(),
    });
    world.resources.insert(LaneState::new(&level.lanes));
    world.resources.insert(EconomyState {
        sky_timer: Timer::once(rules.economy.sky_first_ticks),
    });
    world.resources.insert(DamageQueue::default());
    world.resources.insert(PendingOps::default());
    world.resources.insert(DeathLog::default());
    crate::systems::refresh_cards(world);
    Ok(())
}

/// 结束对局：立即销毁全部 `BattleTag` 实体（SPEC 3.4 `BattleScene::on_exit`）。
pub fn end_battle(world: &mut World) {
    let doomed: Vec<Entity> = world
        .query::<(Entity, &BattleTag)>()
        .iter()
        .map(|(e, _)| e)
        .collect();
    for e in doomed {
        let _ = world.despawn(e);
    }
}

/// 按定义构造防御者的组件（SPEC 7.2）。`host` 为覆盖层的宿主。
#[must_use]
pub fn defender_bundle(def: &DefenderDef, cell: Cell, host: Option<Entity>) -> EntityBuilder {
    let mut b = EntityBuilder::new();
    b.add(BattleTag)
        .add(cell)
        .add(Lane { row: cell.row })
        .add(Transform2D::new(Grid::cell_center(cell.row, cell.col)))
        .add(DefenderRef { id: def.id })
        .add(Health::full(def.hp))
        .add(Placeholder {
            color: def.placeholder.color,
            label: def.placeholder.label,
            size: Grid::DEFENDER_SIZE,
        });
    if def.has_tag(TAG_SPIKY) {
        b.add(NotBlocking);
    }
    if let Some(host) = host {
        b.add(OverlayOf { host });
    }
    for behavior in &def.behaviors {
        match *behavior {
            Behavior::Shooter {
                interval_ticks,
                volley,
                volley_gap_ticks,
                projectile,
            } => {
                b.add(Shooter {
                    timer: finished_timer(interval_ticks),
                    volley,
                    volley_left: 0,
                    volley_timer: Timer::once(volley_gap_ticks),
                    projectile,
                });
            }
            Behavior::Producer {
                interval_ticks,
                amount,
                bonus_every,
                bonus_amount,
            } => {
                b.add(Producer {
                    timer: Timer::repeating(interval_ticks),
                    amount,
                    bonus_every,
                    bonus_amount,
                    produced: 0,
                });
            }
            Behavior::Spikes {
                interval_ticks,
                damage,
            } => {
                b.add(Spikes {
                    timer: Timer::repeating(interval_ticks),
                    damage,
                });
            }
            Behavior::Mine { arm_ticks, damage } => {
                b.add(Mine {
                    arm: Timer::once(arm_ticks),
                    armed: arm_ticks == 0,
                    damage,
                });
            }
            Behavior::Blast {
                fuse_ticks,
                damage,
                radius_cells,
            } => {
                b.add(Blast {
                    fuse: Timer::once(fuse_ticks),
                    damage,
                    radius_cells,
                });
            }
            Behavior::Chomper {
                chew_ticks,
                reach_cells,
            } => {
                b.add(Chomper {
                    state: ChompState::Ready,
                    chew: Timer::once(chew_ticks),
                    reach_cells,
                });
            }
            Behavior::Gust {
                fuse_ticks,
                push_px,
            } => {
                b.add(Gust {
                    fuse: Timer::once(fuse_ticks),
                    push_px,
                });
            }
            Behavior::Wall | Behavior::Overlay { .. } => {}
        }
    }
    b
}

/// 按定义构造敌人的组件（SPEC 8）。
#[must_use]
pub fn enemy_bundle(def: &EnemyDef, row: u8, x: f32, wave: Option<u16>) -> EntityBuilder {
    let mut b = EntityBuilder::new();
    b.add(BattleTag)
        .add(Lane { row })
        .add(Transform2D::new(Vec2::new(x, Grid::row_center_y(row))))
        .add(EnemyRef { id: def.id })
        .add(Health::full(def.hp))
        .add(StatusEffects::default())
        .add(Walker {
            base_speed: def.speed,
            state: WalkState::Walking,
        })
        .add(Biter {
            damage: def.bite_damage,
            interval: def.bite_interval_ticks,
            elapsed: 0,
        })
        .add(Placeholder {
            color: def.placeholder.color,
            label: def.placeholder.label,
            size: Grid::ENEMY_SIZE,
        });
    if let Some(armor) = def.armor {
        let (kind, hp) = armor.split();
        b.add(crate::components::Armor { kind, hp, max: hp });
    }
    if let Some(wave) = wave {
        b.add(SpawnedInWave(wave));
    }
    for t in &def.traits {
        match *t {
            Trait::Vault { ticks } => {
                b.add(Vaulter { used: false, ticks });
            }
            Trait::Sprint {
                threshold_percent,
                factor,
            } => {
                b.add(Sprinter {
                    threshold_percent,
                    factor,
                    triggered: false,
                });
            }
            Trait::Carrier { child } => {
                b.add(Carrier { child });
            }
            Trait::Heavy => {
                b.add(Heavy);
            }
        }
    }
    b
}

/// 立即生成敌人并发 `EnemySpawned`；id 不存在时返回 `None`。
pub fn spawn_enemy(
    world: &mut World,
    id: crate::ids::EnemyId,
    row: u8,
    x: f32,
    wave: Option<u16>,
) -> Option<Entity> {
    let def = world.resources.get::<EnemyRegistry>()?.get(id)?.clone();
    let entity = world.spawn(enemy_bundle(&def, row, x, wave).build());
    world.events.send(EnemySpawned {
        entity,
        id,
        lane: row,
    });
    Some(entity)
}

/// 构造弹丸的组件（SPEC 4.2 / 7.2）。
#[must_use]
pub fn projectile_bundle(def: &ProjectileDef, row: u8, pos: Vec2) -> EntityBuilder {
    let mut b = EntityBuilder::new();
    b.add(BattleTag)
        .add(Lane { row })
        .add(Transform2D::new(pos))
        .add(ProjectileRef { id: def.id })
        .add(ProjectileBody {
            damage: def.damage,
            speed: def.speed,
            pierce: def.pierce,
            origin_x: pos.x,
            hit: Vec::new(),
        })
        .add(Placeholder {
            color: def.placeholder.color,
            label: crate::ids::ShortStr::default(),
            size: Vec2::splat(def.placeholder.radius * 2.0),
        });
    b
}
