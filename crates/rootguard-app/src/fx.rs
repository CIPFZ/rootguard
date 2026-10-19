//! 特效实体（ART-SPEC 7）：呈现层根据模拟事件生成、寿命以 tick 计、不进入模拟层（ADR-016 第 5 节）。
//!
//! [`capture`] 在 `Stage::Update` 中先于 `SfxPlugin` 运行（`sfx_system` 会取走大部分事件），只读事件与实体位置，
//! 把要播放的特效记入 [`crate::render::RenderState`]；[`draw`] 在 Render 阶段按“当前 tick − 开始 tick”取帧绘制。
//! 皮肤缺少某特效片段时不画（占位模式没有特效，SPEC 12.2 不要求）。弹丸种类在发射时记下，命中时据此选特效。

use std::collections::BTreeSet;

use std::collections::BTreeMap;

use rootguard_core::Grid;
use rootguard_core::components::{Cell, ChompState, Chomper, EnemyRef, Lane, ProjectileBody};
use rootguard_core::data::ArmorKind;
use rootguard_core::events::{
    DamageSource, DefenderPlaced, DefenderRemoved, DewSource, DewSpawned, EnemyArmorBroken,
    EnemyDamaged, EnemyDied, HoseTriggered, ProjectileFired, ProjectileHit, RemoveCause,
};
use trellia::prelude::*;

use crate::render::RenderState;
use crate::skin::{self, Painter, Pose};

/// 特效图层：角色之上、HUD 之下。
const FX_LAYER: Layer = Layer::FX;

/// 护甲破碎特效相对敌人脚底的高度（px）：Helm 在头顶、Shield 在胸前、Drum 在腹部（ART-SPEC 5.2 ～ 5.8）。
const HELM_Y: f32 = -112.0;
const SHIELD_Y: f32 = -54.0;
const DRUM_Y: f32 = -72.0;
/// 干屑 / 小芽合拢的中心相对敌人脚底（px）。
const CRUMBLE_Y: f32 = -48.0;
const SPROUT_FOLD_Y: f32 = -26.0;
/// 落地土屑相对格子脚底（px）。
const PLANT_DUST_Y: f32 = -10.0;
/// 炽球热浪的绘制尺寸（ART-SPEC 7：缩放到 300 × 345，即 3 × 3 格）。
const EMBER_WAVE_SIZE: Vec2 = Vec2::new(300.0, 345.0);
/// 荆棘垫扎到时敌人脚下小尘的尺寸（复用 `plant_dust`，ART-SPEC 4.6）。
const PRICK_DUST_SIZE: Vec2 = Vec2::new(56.0, 56.0);
/// 咬夹草打嗝气泡相对格子脚底的位置（嘴的右上方，ART-SPEC 4.10）。
const BURP_OFFSET: Vec2 = Vec2::new(30.0, -78.0);
/// `chew` 片段一个循环的长度：只有咀嚼计时器剩余不超过它时（最后一个循环）才冒泡。
const CHEW_CYCLE_TICKS: u32 = 60;
/// 片段事件帧：咬夹草 `chew` 末帧的打嗝（ART-SPEC v1.3 4.10）。
pub const BURP_EVENT: &str = "burp";
/// 铲除：向上弹起并淡出（ART-SPEC 4.13）的时长与上升速度。
pub const SHOVEL_TICKS: u64 = 12;
const SHOVEL_RISE_PX_PER_TICK: f32 = 2.5;

/// 一个正在播放的特效。
#[derive(Clone, Debug, PartialEq)]
pub struct Fx {
    /// 皮肤片段名（`<特效 id>_play`）。
    pub clip: String,
    /// 中心位置（逻辑像素）。
    pub pos: Vec2,
    /// 开始的 `fixed_tick`。
    pub start: u64,
    /// 绘制尺寸；`None` 为帧原尺寸（ART-SPEC 7 的“缩放到 / 沿车道拉伸”用）。
    pub size: Option<Vec2>,
    /// 铲除的“残影”：`clip` 是防御者自己的 idle 片段，取第 0 帧上升并淡出 [`SHOVEL_TICKS`]。
    pub ghost: bool,
}

impl Fx {
    fn at(fx: &str, pos: Vec2, start: u64) -> Self {
        Self {
            clip: clip_of(fx),
            pos,
            start,
            size: None,
            ghost: false,
        }
    }

    fn sized(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }
}

/// 特效片段名。
#[must_use]
pub fn clip_of(fx: &str) -> String {
    format!("{fx}_play")
}

/// 护甲破碎特效（ART-SPEC 7 `armor_break_*`）：按敌人与护甲种类。
#[must_use]
pub fn armor_break_fx(enemy: &str, kind: ArmorKind) -> (&'static str, f32) {
    match (enemy, kind) {
        ("clay_husk", _) => ("armor_break_pot", HELM_Y),
        (_, ArmorKind::Helm) => ("armor_break_twig", HELM_Y),
        (_, ArmorKind::Shield) => ("armor_break_plank", SHIELD_Y),
        (_, ArmorKind::Drum) => ("armor_break_gourd", DRUM_Y),
    }
}

/// 格子底部（与防御者的脚底一致）。
fn cell_foot(cell: Cell) -> Vec2 {
    skin::foot(Grid::cell_center(cell.row, cell.col))
}

/// `Update`（先于 `sfx_system`）：把本帧的模拟事件转成特效（ART-SPEC 7 与第 4 ～ 6 章的特效表）。
/// 只在对局中（有 `RenderState`）工作。
pub fn capture(world: &mut World) {
    if world.resources.get::<RenderState>().is_none() {
        return;
    }
    let tick = world.resources.expect::<Time>().fixed_tick;
    let mut spawned: Vec<(String, Fx)> = Vec::new();
    let mut push = |key: String, fx: Fx| spawned.push((key, fx));
    let enemy_pos = |e: Entity| {
        let mut q = world
            .entities
            .query_one::<(&EnemyRef, &Transform2D, &Lane)>(e);
        q.get()
            .ok()
            .map(|(r, t, _)| (r.id.as_str().to_owned(), t.pos))
    };
    // 护甲破碎（第 5 节）
    for e in world.events.get::<EnemyArmorBroken>().iter() {
        if let Some((id, pos)) = enemy_pos(e.entity) {
            let (fx, dy) = armor_break_fx(&id, e.kind);
            push(
                format!("armor:{}", e.entity.to_bits().get()),
                Fx::at(fx, skin::foot(pos) + Vec2::new(0.0, dy), tick),
            );
        }
    }
    // 死亡位置（同 tick 死亡的命中目标已被移除）与被吞的敌人（不散成干屑）
    let died: BTreeMap<u64, (u8, f32)> = world
        .events
        .get::<EnemyDied>()
        .iter()
        .map(|d| (d.entity.to_bits().get(), (d.lane, d.x)))
        .collect();
    let swallowed: BTreeSet<u64> = world
        .events
        .get::<EnemyDamaged>()
        .iter()
        .filter(|d| d.source == DamageSource::Chomp)
        .map(|d| d.entity.to_bits().get())
        .collect();
    // 发射：口部小尘 / 白气（4.1 / 4.4），记住弹丸种类供命中时使用
    let mut fired: Vec<(u64, String)> = Vec::new();
    for f in world.events.get::<ProjectileFired>().iter() {
        let id = f.id.as_str().to_owned();
        if let Some(cell) = world.get::<Cell>(f.from).map(|c| *c) {
            let fx = if id == "frost_pebble" {
                "muzzle_frost"
            } else {
                "muzzle_dust"
            };
            let pos = Grid::cell_center(cell.row, cell.col) + Grid::MUZZLE_OFFSET;
            push(
                format!("fire:{}", f.entity.to_bits().get()),
                Fx::at(fx, pos, tick),
            );
        }
        fired.push((f.entity.to_bits().get(), id));
    }
    // 命中：石子碎裂 / 冰晶四散（6 章）
    let mut hits: Vec<(u64, Fx)> = Vec::new();
    {
        let kinds = world
            .resources
            .get::<RenderState>()
            .map(|s| s.projectile_kinds.clone())
            .unwrap_or_default();
        let fired_now: BTreeMap<u64, String> = fired.iter().cloned().collect();
        for h in world.events.get::<ProjectileHit>().iter() {
            let key = h.entity.to_bits().get();
            let frost = kinds
                .get(&key)
                .or_else(|| fired_now.get(&key))
                .is_some_and(|k| k == "frost_pebble");
            let target = h.target.to_bits().get();
            let pos = enemy_pos(h.target).map(|(_, p)| p).or_else(|| {
                died.get(&target)
                    .map(|&(lane, x)| Vec2::new(x, Grid::row_center_y(lane)))
            });
            if let Some(p) = pos {
                let at = Vec2::new(p.x - Grid::ENEMY_HALF_WIDTH, p.y + Grid::MUZZLE_OFFSET.y);
                let fx = if frost { "hit_frost" } else { "hit_puff" };
                hits.push((key, Fx::at(fx, at, tick)));
            }
        }
    }
    for (key, fx) in hits.iter().cloned() {
        push(format!("hit:{key}"), fx);
    }
    // 死亡：干屑 / 小芽合拢（5 章）
    for d in world.events.get::<EnemyDied>().iter() {
        let key = d.entity.to_bits().get();
        if swallowed.contains(&key) {
            continue;
        }
        let foot = skin::foot(Vec2::new(d.x, Grid::row_center_y(d.lane)));
        let (fx, dy) = if d.id.as_str() == "sproutling" {
            ("sprout_fold", SPROUT_FOLD_Y)
        } else {
            ("crumble", CRUMBLE_Y)
        };
        push(
            format!("died:{key}"),
            Fx::at(fx, foot + Vec2::new(0.0, dy), tick),
        );
    }
    // 荆棘垫扎到：被扎的敌人脚下小尘（4.6）
    for d in world.events.get::<EnemyDamaged>().iter() {
        if d.source == DamageSource::Spikes
            && let Some((_, pos)) = enemy_pos(d.entity)
        {
            push(
                format!("prick:{}", d.entity.to_bits().get()),
                Fx::at("plant_dust", skin::foot(pos), tick).sized(PRICK_DUST_SIZE),
            );
        }
    }
    // 放置落地土屑（4.13）
    for p in world.events.get::<DefenderPlaced>().iter() {
        push(
            format!("plant:{}", p.entity.to_bits().get()),
            Fx::at(
                "plant_dust",
                cell_foot(p.cell) + Vec2::new(0.0, PLANT_DUST_Y),
                tick,
            ),
        );
    }
    // 防御者离场：被吃光落叶（4.13）、铲除残影（4.13）、消耗型的爆炸 / 风环（4.7 / 4.8 / 4.11）
    for r in world.events.get::<DefenderRemoved>().iter() {
        let key = format!("removed:{}", r.entity.to_bits().get());
        let center = Grid::cell_center(r.cell.row, r.cell.col);
        let fx = match (r.cause, r.id.as_str()) {
            (RemoveCause::Killed, _) => Fx::at("wilt", center, tick),
            (RemoveCause::Shovel, id) => Fx {
                clip: format!("{id}_idle"),
                pos: cell_foot(r.cell),
                start: tick,
                size: None,
                ghost: true,
            },
            (RemoveCause::Consumed, "tuberblast") => Fx::at("tuber_burst", center, tick),
            (RemoveCause::Consumed, "emberbulb") => {
                Fx::at("ember_wave", center, tick).sized(EMBER_WAVE_SIZE)
            }
            (RemoveCause::Consumed, "gustbloom") => {
                // 风环从风铃花沿车道扫到右端（4.11）
                let left = Grid::cell_left(r.cell.col);
                let right = Grid::GUST_CLAMP_X;
                Fx::at("gust_ring", Vec2::new((left + right) / 2.0, center.y), tick)
                    .sized(Vec2::new(right - left, Grid::CELL_SIZE.y))
            }
            _ => continue,
        };
        push(key, fx);
    }
    // 露水产出星光（4.2）
    for d in world.events.get::<DewSpawned>().iter() {
        if matches!(d.source, DewSource::Producer(_))
            && let Some(t) = world.get::<Transform2D>(d.entity)
        {
            push(
                format!("dew:{}", d.entity.to_bits().get()),
                Fx::at("dew_pop", t.pos, tick),
            );
        }
    }
    // 咬夹草咀嚼结束打嗝：chew 片段末帧事件，只在最后一个循环冒一个气泡（ART-SPEC v1.3 4.10）
    for a in world.events.get::<AnimationEvent>().iter() {
        if a.event != BURP_EVENT {
            continue;
        }
        let mut q = world.entities.query_one::<(&Chomper, &Cell)>(a.entity);
        if let Ok((c, cell)) = q.get()
            && c.state == ChompState::Chewing
            && c.chew.remaining <= CHEW_CYCLE_TICKS
        {
            push(
                format!("burp:{}", a.entity.to_bits().get()),
                Fx::at("burp_bubble", cell_foot(*cell) + BURP_OFFSET, tick),
            );
        }
    }
    // Hose：整条车道冲水（LORE 2.5，ART-SPEC 7 “车道全长”）
    for h in world.events.get::<HoseTriggered>().iter() {
        let y = Grid::row_center_y(h.lane);
        push(
            format!("hose:{}", h.lane),
            Fx::at(
                "hose_burst",
                Vec2::new((Grid::HOSE_X + Grid::RIGHT_X) / 2.0, y),
                tick,
            )
            .sized(Vec2::new(Grid::RIGHT_X - Grid::HOSE_X, Grid::CELL_SIZE.y)),
        );
    }
    let alive_projectiles: BTreeSet<u64> = world
        .query::<(Entity, &ProjectileBody)>()
        .iter()
        .map(|(e, _)| e.to_bits().get())
        .collect();
    let Some(state) = world.resources.get_mut::<RenderState>() else {
        return;
    };
    // 未被 sfx_system 取走的事件在下一帧仍可读：上一帧已生成过的键不再生成
    let keys: BTreeSet<String> = spawned.iter().map(|(k, _)| k.clone()).collect();
    for (key, fx) in spawned {
        if !state.fx_keys.contains(&key) {
            state.fx.push(fx);
        }
    }
    state.fx_keys = keys;
    state.projectile_kinds.extend(fired);
    state
        .projectile_kinds
        .retain(|k, _| alive_projectiles.contains(k));
}

/// 删除已播放完的特效（`Update`，片段长度取所选皮肤；皮肤没有该片段时立即删除）。
pub fn prune(world: &mut World) {
    let tick = world.resources.expect::<Time>().fixed_tick;
    let lens: Vec<Option<u64>> = {
        let Some(state) = world.resources.get::<RenderState>() else {
            return;
        };
        let view = skin::SkinView::selected(world);
        let clips = world.resources.get::<Assets<AnimationClip>>();
        state
            .fx
            .iter()
            .map(|f| {
                if f.ghost {
                    return Some(SHOVEL_TICKS);
                }
                let h = view.as_ref()?.clip(&f.clip)?;
                clips?.get(h).map(skin::clip_ticks)
            })
            .collect()
    };
    if let Some(state) = world.resources.get_mut::<RenderState>() {
        let mut i = 0;
        state.fx.retain(|f| {
            let keep = lens[i].is_some_and(|len| tick.saturating_sub(f.start) < len);
            i += 1;
            keep
        });
    }
}

/// Render：绘制全部特效。
pub fn draw(world: &World, list: &mut DrawList, painter: Option<&Painter<'_>>, tick: u64) {
    let (Some(sk), Some(state)) = (painter, world.resources.get::<RenderState>()) else {
        return;
    };
    for f in &state.fx {
        let elapsed = tick.saturating_sub(f.start);
        if f.ghost {
            // 铲除：取 idle 第 0 帧向上弹起并淡出（ART-SPEC 4.13）
            let k = elapsed as f32 / SHOVEL_TICKS as f32;
            let pose = Pose {
                clip: &f.clip,
                elapsed: 0,
                flash: false,
                tint: Color::WHITE.with_alpha((1.0 - k).clamp(0.0, 1.0)),
            };
            let pos = f.pos - Vec2::new(0.0, SHOVEL_RISE_PX_PER_TICK * elapsed as f32);
            sk.draw(list, Layer::WORLD, pos, f.pos.y, pose);
            continue;
        }
        let pose = Pose {
            clip: &f.clip,
            elapsed,
            flash: false,
            tint: Color::WHITE,
        };
        match f.size {
            Some(size) => sk.draw_sized(list, FX_LAYER, f.pos, f.pos.y, pose, size),
            None => sk.draw(list, FX_LAYER, f.pos, f.pos.y, pose),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armor_break_fx_by_enemy() {
        assert_eq!(
            armor_break_fx("twig_husk", ArmorKind::Helm).0,
            "armor_break_twig"
        );
        assert_eq!(
            armor_break_fx("clay_husk", ArmorKind::Helm).0,
            "armor_break_pot"
        );
        assert_eq!(
            armor_break_fx("shield_husk", ArmorKind::Shield).0,
            "armor_break_plank"
        );
        assert_eq!(
            armor_break_fx("drum_husk", ArmorKind::Drum).0,
            "armor_break_gourd"
        );
    }
}
