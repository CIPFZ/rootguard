//! SPEC 6 第 6 ～ 10 步：状态、行走、防御者行为、弹丸、啃食。

use trellia_core::{Entity, Transform2D, Vec2, World};

use crate::battle::projectile_bundle;
use crate::components::{
    Biter, Blast, Cell, ChompState, Chomper, EnemyRef, Gust, Health, Heavy, Lane, Mine,
    NotBlocking, OverlayOf, ProjectileBody, ProjectileRef, Shielded, Shooter, Spikes, Sprinter,
    StatusEffects, Vaulter, WalkState, Walker,
};
use crate::data::{OnHit, Rules};
use crate::events::{DamageSource, ProjectileFired, ProjectileHit, RemoveCause};
use crate::grid::Grid;
use crate::registry::ProjectileRegistry;
use crate::resources::DamageQueue;
use crate::systems::input::remove_defender;
use crate::systems::{EnemyView, alive_enemies};

/// SPEC 6 第 6 步：`StatusEffects` 计时递减。
pub fn status_system(world: &mut World) {
    for status in world.entities.query_mut::<&mut StatusEffects>() {
        status.chill_ticks = status.chill_ticks.saturating_sub(1);
        status.stun_ticks = status.stun_ticks.saturating_sub(1);
    }
}

/// 一个存活防御者的快照（行走判定用）。
#[derive(Clone, Copy, Debug)]
struct DefenderView {
    entity: Entity,
    cell: Cell,
    chomper_ready: bool,
}

/// 可被敌人"到达"并啃食的存活地面防御者（排除覆盖层与 `NotBlocking`）。
fn blocking_defenders(world: &World) -> Vec<DefenderView> {
    world
        .query::<(Entity, &Cell, &Health, Option<&Chomper>)>()
        .without::<&OverlayOf>()
        .without::<&NotBlocking>()
        .iter()
        .filter(|(_, _, h, _)| h.alive())
        .map(|(entity, cell, _, chomper)| DefenderView {
            entity,
            cell: *cell,
            chomper_ready: chomper.is_some_and(|c| c.state == ChompState::Ready),
        })
        .collect()
}

/// SPEC 6 第 7 步：敌人状态机（行走 / 啃食 / 跨越 / 被吞）、"到达防御者"判定、Sprinter 触发。
pub fn walker_system(world: &mut World) {
    let chill = world.resources.expect::<Rules>().status.chill_speed_factor;
    let defenders = blocking_defenders(world);
    for (t, walker, biter, lane, health, status, vaulter, sprinter) in world.entities.query_mut::<(
        &mut Transform2D,
        &mut Walker,
        &mut Biter,
        &Lane,
        &Health,
        &StatusEffects,
        Option<&mut Vaulter>,
        Option<&mut Sprinter>,
    )>() {
        if !health.alive() {
            continue;
        }
        if let Some(s) = sprinter
            && !s.triggered
            && health.hp * 100 <= health.max * s.threshold_percent
        {
            s.triggered = true;
            walker.base_speed *= s.factor;
        }
        let speed = if status.chilled() {
            walker.base_speed * chill
        } else {
            walker.base_speed
        };
        let x = t.pos.x;
        match walker.state {
            WalkState::Swallowed => continue,
            WalkState::Vaulting {
                from,
                to,
                t: elapsed,
            } => {
                let total = vaulter.map_or(1, |v| v.ticks.max(1));
                let elapsed = elapsed + 1;
                if elapsed >= total {
                    t.pos.x = to;
                    walker.state = WalkState::Walking;
                } else {
                    t.pos.x = from + (to - from) * (elapsed as f32 / total as f32);
                    walker.state = WalkState::Vaulting {
                        from,
                        to,
                        t: elapsed,
                    };
                }
                continue;
            }
            WalkState::Eating(target) => {
                let still = defenders.iter().any(|d| {
                    d.entity == target
                        && d.cell.row == lane.row
                        && Grid::enemy_reaches_col(x, d.cell.col)
                });
                if still {
                    continue;
                }
                walker.state = WalkState::Walking;
                biter.elapsed = 0;
            }
            WalkState::Walking => {}
        }
        // 到达多个防御者时取最靠右者（最先接触）
        let reached = defenders
            .iter()
            .filter(|d| d.cell.row == lane.row && Grid::enemy_reaches_col(x, d.cell.col))
            .max_by_key(|d| d.cell.col);
        match reached {
            Some(d) => match vaulter {
                // 首次遇到防御者时跨越到其左侧格；就绪的吞食者优先于跨越（SPEC 8.1）
                Some(v) if !v.used && !d.chomper_ready => {
                    v.used = true;
                    let to = Grid::cell_center(d.cell.row, d.cell.col).x - Grid::CELL_SIZE.x;
                    walker.state = WalkState::Vaulting { from: x, to, t: 0 };
                }
                _ => {
                    walker.state = WalkState::Eating(d.entity);
                    biter.elapsed = 0;
                }
            },
            None => t.pos.x -= speed,
        }
    }
}

/// SPEC 6 第 8 步：Shooter / Spikes / Mine / Blast / Chomper / Gust 的计时与触发。
pub fn defender_behavior_system(world: &mut World) {
    let enemies = alive_enemies(world);
    shooters(world, &enemies);
    spikes(world, &enemies);
    mines(world, &enemies);
    blasts(world, &enemies);
    chompers(world);
    gusts(world);
}

fn in_row(enemies: &[EnemyView], row: u8) -> impl Iterator<Item = &EnemyView> {
    enemies.iter().filter(move |e| e.row == row)
}

fn shooters(world: &mut World, enemies: &[EnemyView]) {
    let mut shots: Vec<(Entity, Cell, crate::ids::ProjectileId)> = Vec::new();
    for (e, shooter, cell, h) in world
        .entities
        .query_mut::<(Entity, &mut Shooter, &Cell, &Health)>()
    {
        if !h.alive() {
            continue;
        }
        let right = Grid::cell_right(cell.col);
        let has_target = in_row(enemies, cell.row)
            .any(|en| en.x > right - Grid::SHOOTER_BACK_TOLERANCE && en.x <= Grid::SHOOTER_MAX_X);
        if !shooter.timer.finished() {
            shooter.timer.tick();
        }
        if shooter.volley_left > 0 && shooter.volley_timer.tick() {
            shooter.volley_left -= 1;
            shooter.volley_timer.reset();
            shots.push((e, *cell, shooter.projectile));
        }
        if shooter.timer.finished() && has_target {
            shooter.timer.reset();
            let extra = shooter.volley.saturating_sub(1);
            let burst = if shooter.volley_timer.duration == 0 {
                extra + 1
            } else {
                shooter.volley_left = extra;
                shooter.volley_timer.reset();
                1
            };
            for _ in 0..burst {
                shots.push((e, *cell, shooter.projectile));
            }
        }
    }
    shots.sort_by_key(|(e, _, _)| e.to_bits());
    for (from, cell, id) in shots {
        let Some(def) = world
            .resources
            .expect::<ProjectileRegistry>()
            .get(id)
            .cloned()
        else {
            continue;
        };
        let pos = Grid::cell_center(cell.row, cell.col) + Grid::MUZZLE_OFFSET;
        let entity = world.spawn(projectile_bundle(&def, cell.row, pos).build());
        world.events.send(ProjectileFired { entity, from, id });
    }
}

fn spikes(world: &mut World, enemies: &[EnemyView]) {
    let mut hits: Vec<(Entity, i32)> = Vec::new();
    for (spikes, cell, h) in world.entities.query_mut::<(&mut Spikes, &Cell, &Health)>() {
        if !h.alive() || !spikes.timer.tick() {
            continue;
        }
        for en in in_row(enemies, cell.row) {
            if Grid::enemy_overlaps_cols(en.x, cell.col, 0, 0) {
                hits.push((en.entity, spikes.damage));
            }
        }
    }
    let queue = world.resources.expect_mut::<DamageQueue>();
    for (target, amount) in hits {
        queue.push(target, amount, DamageSource::Spikes, false);
    }
}

fn mines(world: &mut World, enemies: &[EnemyView]) {
    let mut exploded: Vec<(Entity, Vec<Entity>, i32)> = Vec::new();
    for (e, mine, cell, h) in world
        .entities
        .query_mut::<(Entity, &mut Mine, &Cell, &Health)>()
    {
        if !h.alive() {
            continue;
        }
        if !mine.armed {
            mine.armed = mine.arm.tick() || mine.arm.finished();
            if !mine.armed {
                continue;
            }
        }
        let victims: Vec<Entity> = in_row(enemies, cell.row)
            .filter(|en| Grid::enemy_overlaps_cols(en.x, cell.col, 0, 0))
            .map(|en| en.entity)
            .collect();
        if !victims.is_empty() {
            exploded.push((e, victims, mine.damage));
        }
    }
    exploded.sort_by_key(|(e, _, _)| e.to_bits());
    for (mine, victims, damage) in exploded {
        let queue = world.resources.expect_mut::<DamageQueue>();
        for v in victims {
            queue.push(v, damage, DamageSource::Mine, true);
        }
        remove_defender(world, mine, RemoveCause::Consumed);
    }
}

fn blasts(world: &mut World, enemies: &[EnemyView]) {
    let mut fired: Vec<(Entity, Cell, i32, u8)> = Vec::new();
    for (e, blast, cell, h) in world
        .entities
        .query_mut::<(Entity, &mut Blast, &Cell, &Health)>()
    {
        if h.alive() && blast.fuse.tick() {
            fired.push((e, *cell, blast.damage, blast.radius_cells));
        }
    }
    fired.sort_by_key(|(e, _, _, _)| e.to_bits());
    for (blast, cell, damage, r) in fired {
        let queue = world.resources.expect_mut::<DamageQueue>();
        for en in enemies {
            if en.row.abs_diff(cell.row) <= r && Grid::enemy_overlaps_cols(en.x, cell.col, r, r) {
                queue.push(en.entity, damage, DamageSource::Blast, true);
            }
        }
        remove_defender(world, blast, RemoveCause::Consumed);
    }
}

fn chompers(world: &mut World) {
    // 吞食目标须非 Heavy；同 tick 被多个吞食者选中时只吞一次
    let candidates: Vec<EnemyView> = {
        let mut list: Vec<EnemyView> = world
            .query::<(Entity, &EnemyRef, &Health, &Lane, &Transform2D, &Walker)>()
            .without::<&Heavy>()
            .iter()
            .filter(|(_, _, h, _, _, w)| h.alive() && w.state != WalkState::Swallowed)
            .map(|(entity, _, _, lane, t, _)| EnemyView {
                entity,
                row: lane.row,
                x: t.pos.x,
            })
            .collect();
        crate::systems::sort_targets(&mut list);
        list
    };
    let mut chomps: Vec<(Entity, Entity)> = Vec::new();
    let mut taken: Vec<Entity> = Vec::new();
    let mut order: Vec<(Entity, Cell)> = world
        .query::<(Entity, &Chomper, &Cell, &Health)>()
        .iter()
        .filter(|(_, _, _, h)| h.alive())
        .map(|(e, _, c, _)| (e, *c))
        .collect();
    order.sort_by_key(|(e, _)| e.to_bits());
    for (e, cell) in order {
        let Some(mut chomper) = world.get_mut::<Chomper>(e) else {
            continue;
        };
        if chomper.state == ChompState::Chewing {
            if chomper.chew.tick() || chomper.chew.finished() {
                chomper.state = ChompState::Ready;
            }
            continue;
        }
        let reach = chomper.reach_cells;
        let target = candidates.iter().find(|en| {
            en.row == cell.row
                && !taken.contains(&en.entity)
                && Grid::enemy_overlaps_cols(en.x, cell.col, 0, reach)
        });
        if let Some(target) = target {
            chomper.state = ChompState::Chewing;
            chomper.chew.reset();
            taken.push(target.entity);
            chomps.push((e, target.entity));
        }
    }
    for (_, target) in chomps {
        if let Some(mut w) = world.get_mut::<Walker>(target) {
            w.state = WalkState::Swallowed;
        }
        world
            .resources
            .expect_mut::<DamageQueue>()
            .push(target, 0, DamageSource::Chomp, true);
    }
}

fn gusts(world: &mut World) {
    let heavy_factor = world.resources.expect::<Rules>().heavy_push_factor;
    let mut fired: Vec<(Entity, u8, f32)> = Vec::new();
    for (e, gust, cell, h) in world
        .entities
        .query_mut::<(Entity, &mut Gust, &Cell, &Health)>()
    {
        if h.alive() && gust.fuse.tick() {
            fired.push((e, cell.row, gust.push_px));
        }
    }
    fired.sort_by_key(|(e, _, _)| e.to_bits());
    for (gust, row, push) in fired {
        for (t, walker, lane, h, heavy) in world.entities.query_mut::<(
            &mut Transform2D,
            &mut Walker,
            &Lane,
            &Health,
            Option<&Heavy>,
        )>() {
            if lane.row != row || !h.alive() || walker.state == WalkState::Swallowed {
                continue;
            }
            let dist = if heavy.is_some() {
                push * heavy_factor
            } else {
                push
            };
            let x = t.pos.x;
            let new_x = (x + dist).min(Grid::GUST_CLAMP_X).max(x);
            let delta = new_x - x;
            t.teleport(Vec2::new(new_x, t.pos.y));
            if let WalkState::Vaulting { from, to, t: el } = walker.state {
                walker.state = WalkState::Vaulting {
                    from: from + delta,
                    to: to + delta,
                    t: el,
                };
            }
        }
        remove_defender(world, gust, RemoveCause::Consumed);
    }
}

/// SPEC 6 第 9 步：弹丸位移、越界销毁、命中判定（SPEC 2.5），生成伤害请求与命中效果。
pub fn projectile_system(world: &mut World) {
    let enemies = alive_enemies(world);
    let mut hits: Vec<(Entity, Entity, i32, crate::ids::ProjectileId)> = Vec::new();
    for (e, t, body, lane, pref) in world.entities.query_mut::<(
        Entity,
        &mut Transform2D,
        &mut ProjectileBody,
        &Lane,
        &ProjectileRef,
    )>() {
        t.pos.x += body.speed;
        let x = t.pos.x;
        if x > Grid::PROJECTILE_DESPAWN_X {
            world.commands.despawn(e);
            continue;
        }
        let reach = Grid::ENEMY_HALF_WIDTH + Grid::PROJECTILE_HALF_WIDTH;
        let targets: Vec<Entity> = in_row(&enemies, lane.row)
            .filter(|en| {
                en.x >= body.origin_x && (en.x - x).abs() <= reach && !body.hit.contains(&en.entity)
            })
            .map(|en| en.entity)
            .collect();
        let Some(&target) = targets.first() else {
            continue;
        };
        hits.push((e, target, body.damage, pref.id));
        if body.pierce {
            body.hit.push(target);
        } else {
            world.commands.despawn(e);
        }
    }
    for (projectile, target, damage, id) in hits {
        let armored = world
            .get::<crate::components::Armor>(target)
            .is_some_and(|a| a.hp > 0);
        world.resources.expect_mut::<DamageQueue>().push(
            target,
            damage,
            DamageSource::Projectile,
            false,
        );
        let effects = world
            .resources
            .expect::<ProjectileRegistry>()
            .get(id)
            .map(|d| d.on_hit.clone())
            .unwrap_or_default();
        for effect in effects {
            match effect {
                OnHit::Chill { ticks } => {
                    if let Some(mut s) = world.get_mut::<StatusEffects>(target) {
                        s.chill_ticks = ticks;
                    }
                }
            }
        }
        world.events.send(ProjectileHit {
            entity: projectile,
            target,
            armored,
        });
    }
}

/// SPEC 6 第 10 步：啃食计时（Chill 时间隔按规则倍增），对防御者（优先覆盖层）产生伤害请求。
pub fn bite_system(world: &mut World) {
    let factor = world
        .resources
        .expect::<Rules>()
        .status
        .chill_bite_interval_factor;
    let mut bites: Vec<(Entity, i32)> = Vec::new();
    for (walker, biter, h, status) in world
        .entities
        .query_mut::<(&Walker, &mut Biter, &Health, &StatusEffects)>()
    {
        let WalkState::Eating(target) = walker.state else {
            continue;
        };
        if !h.alive() {
            continue;
        }
        biter.elapsed += 1;
        let interval = if status.chilled() {
            biter.interval * factor
        } else {
            biter.interval
        };
        if biter.elapsed >= interval {
            biter.elapsed = 0;
            bites.push((target, biter.damage));
        }
    }
    for (host, damage) in bites {
        let overlay = world
            .get::<Shielded>(host)
            .map(|s| s.overlay)
            .filter(|&o| world.get::<Health>(o).is_some_and(|h| h.alive()));
        world.resources.expect_mut::<DamageQueue>().push(
            overlay.unwrap_or(host),
            damage,
            DamageSource::Bite,
            false,
        );
    }
}
