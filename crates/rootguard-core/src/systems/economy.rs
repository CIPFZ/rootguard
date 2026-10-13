//! SPEC 6 第 4 步：经济（SPEC 9）。

use trellia_core::{Entity, GameRng, Timer, Transform2D, Vec2, World};

use crate::battle::EconomyState;
use crate::components::{BattleTag, Cell, DewPickup, Health, Producer};
use crate::data::Rules;
use crate::events::{DewCollected, DewSource, DewSpawned};
use crate::grid::Grid;
use crate::registry::{DefenderRegistry, LevelRegistry};
use crate::resources::{BattleState, CardTray, LaneState, PendingOps};

/// SPEC 6 第 4 步：天空 Dew、Producer 生产、Dew 下落与过期、点击收集、卡牌冷却推进。
pub fn economy_system(world: &mut World) {
    let rules = world.resources.expect::<Rules>().clone();
    sky_dew(world, &rules);
    produce(world, &rules);
    fall_and_expire(world, &rules);
    collect(world, &rules);
    for card in &mut world.resources.expect_mut::<CardTray>().cards {
        card.cooldown.tick();
    }
    refresh_cards(world);
}

/// 刷新全部卡牌的 `ready = cooldown.finished() && dew ≥ cost`（SPEC 9）。
pub fn refresh_cards(world: &mut World) {
    let Some(dew) = world.resources.get::<BattleState>().map(|s| s.dew) else {
        return;
    };
    let Some(defenders) = world.resources.remove::<DefenderRegistry>() else {
        return;
    };
    if let Some(tray) = world.resources.get_mut::<CardTray>() {
        for card in &mut tray.cards {
            let cost = defenders.get(card.defender).map_or(i32::MAX, |d| d.cost);
            card.ready = card.cooldown.finished() && dew >= cost;
        }
    }
    world.resources.insert(defenders);
}

fn spawn_dew(
    world: &mut World,
    pos: Vec2,
    land_y: f32,
    amount: i32,
    landed: bool,
    rules: &Rules,
) -> Entity {
    world.spawn((
        BattleTag,
        Transform2D::new(pos),
        DewPickup {
            amount,
            land_y,
            expire: Timer::once(rules.economy.dew_expire_ticks),
            landed,
        },
    ))
}

fn sky_dew(world: &mut World, rules: &Rules) {
    let level = world.resources.expect::<BattleState>().level;
    let sky = world
        .resources
        .expect::<LevelRegistry>()
        .get(level)
        .is_some_and(|l| l.sky_dew);
    if !sky
        || !world
            .resources
            .expect_mut::<EconomyState>()
            .sky_timer
            .tick()
    {
        return;
    }
    let rows = world.resources.expect::<LaneState>().active_rows();
    let eco = rules.economy;
    let rng = world.resources.expect_mut::<GameRng>();
    let Some(&row) = rng.choose(&rows) else {
        return;
    };
    let col = rng.gen_range(0..Grid::COLS);
    let next = eco.sky_interval_ticks + rng.gen_range(0..eco.sky_interval_jitter_ticks.max(1));
    world.resources.expect_mut::<EconomyState>().sky_timer = Timer::once(next);
    let target = Grid::cell_center(row, col);
    let entity = spawn_dew(
        world,
        Vec2::new(target.x, Grid::SKY_DEW_START_Y),
        target.y,
        eco.sky_amount,
        false,
        rules,
    );
    world.events.send(DewSpawned {
        entity,
        source: DewSource::Sky,
    });
}

fn produce(world: &mut World, rules: &Rules) {
    let mut ready: Vec<(Entity, Cell, i32)> = Vec::new();
    for (e, producer, cell, h) in world
        .entities
        .query_mut::<(Entity, &mut Producer, &Cell, &Health)>()
    {
        if !h.alive() || !producer.timer.tick() {
            continue;
        }
        producer.produced += 1;
        let bonus = producer.bonus_every > 0 && producer.produced % producer.bonus_every == 0;
        let amount = producer.amount + if bonus { producer.bonus_amount } else { 0 };
        ready.push((e, *cell, amount));
    }
    ready.sort_by_key(|(e, _, _)| e.to_bits());
    let r = Grid::PRODUCER_SCATTER_PX;
    for (source, cell, amount) in ready {
        let rng = world.resources.expect_mut::<GameRng>();
        let offset = Vec2::new(rng.gen_range(-r..=r) as f32, rng.gen_range(-r..=r) as f32);
        let pos = Grid::cell_center(cell.row, cell.col) + offset;
        let entity = spawn_dew(world, pos, pos.y, amount, true, rules);
        world.events.send(DewSpawned {
            entity,
            source: DewSource::Producer(source),
        });
    }
}

fn fall_and_expire(world: &mut World, rules: &Rules) {
    let speed = rules.economy.sky_fall_speed;
    for (e, t, dew) in world
        .entities
        .query_mut::<(Entity, &mut Transform2D, &mut DewPickup)>()
    {
        if !dew.landed {
            t.pos.y += speed;
            if t.pos.y >= dew.land_y {
                t.pos.y = dew.land_y;
                dew.landed = true;
            }
        } else if dew.expire.tick() {
            dew.amount = 0;
            world.commands.despawn(e);
        }
    }
}

fn collect(world: &mut World, rules: &Rules) {
    let clicks = std::mem::take(&mut world.resources.expect_mut::<PendingOps>().collect);
    let radius_sq = Grid::DEW_COLLECT_RADIUS * Grid::DEW_COLLECT_RADIUS;
    for click in clicks {
        let hit = world
            .query::<(Entity, &Transform2D, &DewPickup)>()
            .iter()
            .filter(|(_, _, d)| d.amount > 0)
            .map(|(e, t, d)| (e, (t.pos - click).length_squared(), d.amount, t.pos))
            .filter(|(_, dist, _, _)| *dist <= radius_sq)
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.to_bits().cmp(&b.0.to_bits())));
        let Some((e, _, amount, pos)) = hit else {
            continue;
        };
        if let Some(mut d) = world.get_mut::<DewPickup>(e) {
            d.amount = 0;
        }
        world.commands.despawn(e);
        let state = world.resources.expect_mut::<BattleState>();
        state.dew = (state.dew + amount).min(rules.economy.dew_cap);
        world.events.send(DewCollected { amount, pos });
    }
}
