//! SPEC 6 第 5 / 14 步：波次调度与胜负判定（SPEC 10.1）。

use trellia_core::{GameRng, Timer, World};

use crate::battle::spawn_enemy;
use crate::components::{Armor, EnemyRef, Health, SpawnedInWave};
use crate::data::{LaneSpec, Rules};
use crate::events::{BattleEnded, Outcome, WaveStarted};
use crate::grid::Grid;
use crate::registry::{EnemyRegistry, LevelRegistry};
use crate::resources::{BattleState, LaneState, Phase, WaveState};

/// 第 `wave` 波生成的、仍存活的敌人的剩余总 hp（主生命 + 护甲）。
fn wave_hp_left(world: &World, wave: u16) -> i32 {
    world
        .query::<(&EnemyRef, &Health, &SpawnedInWave, Option<&Armor>)>()
        .iter()
        .filter(|(_, h, w, _)| h.alive() && w.0 == wave)
        .map(|(_, h, _, a)| h.hp + a.map_or(0, |a| a.hp.max(0)))
        .sum()
}

/// SPEC 6 第 5 步：波次计时与生成、提前开始（剩余 hp ≤ 40%，最低间隔 300 tick）、flag 波横幅。
pub fn wave_system(world: &mut World) {
    let rules = world.resources.expect::<Rules>().waves;
    let state = world.resources.expect::<BattleState>();
    let (level_id, now) = (state.level, state.level_tick);
    let Some(level) = world
        .resources
        .expect::<LevelRegistry>()
        .get(level_id)
        .cloned()
    else {
        return;
    };
    let mut ws = *world.resources.expect::<WaveState>();

    if let Some(banner) = ws.banner.as_mut()
        && banner.tick()
    {
        ws.banner = None;
    }

    if ws.next_wave >= level.waves.len() {
        ws.all_spawned = true;
        world.resources.insert(ws);
        return;
    }

    // 提前开始：上一波剩余 hp ≤ early_start_percent% 时，下一波在最低间隔满足后立即生成
    if let (Some(last), Some(prev)) = (ws.last_spawn_tick, ws.next_wave.checked_sub(1)) {
        let left = wave_hp_left(world, u16::try_from(prev).unwrap_or(u16::MAX));
        ws.wave_hp_left = left;
        if ws.wave_hp_total > 0 && left * 100 <= ws.wave_hp_total * rules.early_start_percent {
            let since = now.saturating_sub(last);
            let gap_left = u64::from(rules.min_gap_ticks).saturating_sub(since);
            let remaining = u32::try_from(gap_left)
                .unwrap_or(u32::MAX)
                .saturating_add(1);
            if ws.wave_timer.remaining > remaining {
                ws.wave_timer.remaining = remaining;
            }
        }
    }

    let next = &level.waves[ws.next_wave];
    if next.flag && ws.banner.is_none() && ws.wave_timer.remaining <= rules.flag_banner_ticks {
        ws.banner = Some(Timer::once(ws.wave_timer.remaining));
    }

    if !ws.wave_timer.tick() && ws.wave_timer.remaining > 0 {
        world.resources.insert(ws);
        return;
    }

    // 生成本波
    let index = ws.next_wave;
    let wave_tag = u16::try_from(index).unwrap_or(u16::MAX);
    let rows = world.resources.expect::<LaneState>().active_rows();
    let mut total = 0;
    for spawn in &next.spawns {
        let Some(def_hp) = world
            .resources
            .expect::<EnemyRegistry>()
            .get(spawn.enemy)
            .map(crate::data::EnemyDef::total_hp)
        else {
            continue;
        };
        let rng = world.resources.expect_mut::<GameRng>();
        let start = if rows.is_empty() {
            0
        } else {
            rng.gen_range(0..rows.len())
        };
        for j in 0..spawn.count as usize {
            let rng = world.resources.expect_mut::<GameRng>();
            let row = match spawn.lane {
                LaneSpec::Random => rng.choose(&rows).copied(),
                LaneSpec::Spread => rows.get((start + j) % rows.len().max(1)).copied(),
                LaneSpec::Fixed(r) => Some(r),
            };
            let Some(row) = row else {
                continue;
            };
            let x = Grid::SPAWN_X + rng.gen_range(0..Grid::SPAWN_JITTER_PX) as f32;
            if spawn_enemy(world, spawn.enemy, row, x, Some(wave_tag)).is_some() {
                total += def_hp;
            }
        }
    }
    world.events.send(WaveStarted {
        index,
        flag: next.flag,
    });
    ws.banner = None;
    ws.last_spawn_tick = Some(now);
    ws.wave_hp_total = total;
    ws.wave_hp_left = total;
    ws.next_wave += 1;
    match level.waves.get(ws.next_wave) {
        Some(w) => ws.wave_timer = Timer::once(w.delay_ticks),
        None => ws.all_spawned = true,
    }
    world.resources.insert(ws);
}

/// SPEC 6 第 14 步：全部波次生成完毕且场上无存活敌人 → `Won`，发 `BattleEnded`。
pub fn outcome_system(world: &mut World) {
    let state = world.resources.expect::<BattleState>();
    if state.phase != Phase::Playing || !world.resources.expect::<WaveState>().all_spawned {
        return;
    }
    let any_alive = world
        .query::<(&EnemyRef, &Health)>()
        .iter()
        .any(|(_, h)| h.alive());
    if any_alive {
        return;
    }
    world.resources.expect_mut::<BattleState>().phase = Phase::Won;
    world.events.send(BattleEnded {
        outcome: Outcome::Won,
    });
}
