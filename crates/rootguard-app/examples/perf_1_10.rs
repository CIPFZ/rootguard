//! 性能测量（SPEC 16.3）：release 下 headless 回放 1-10 脚本解，逐帧计时（模拟 + 呈现层 DrawList 生成，
//! 不含 GPU 与 egui 绘制），报告最大场面附近的帧耗时。
//!
//! `cargo run --release -p rootguard-app --example perf_1_10 [-- --skinned [--garden]] [--stress]`
//!
//! `--stress`：不回放脚本，而是把 45 格全部种满双籽荚并持续保持约 40 个鼓腹枯壳在场，
//! 达到 SPEC 16.3 假设的“≈ 40 敌人、45 防御者、100 弹丸”（直接改对局状态，仅用于测量）。
//!
//! 设计期工具，不是测试（测试禁止真实时间，AGENT 4.5）。结果记录在 `docs/progress.md`。

use std::time::Instant;

use rootguard_app::save::SaveLocation;
use rootguard_app::script::{Replay, Script};
use rootguard_app::{BattleScene, build_app_full};
use rootguard_core::Grid;
use rootguard_core::components::{BattleTag, Cell, EnemyRef, ProjectileBody};
use rootguard_core::events::{SkinMode, UiAction};
use rootguard_core::resources::{BattleState, CardTray, Phase};
use trellia::prelude::*;

/// 压力场景的目标敌人数与帧数。
const STRESS_ENEMIES: usize = 40;
const STRESS_FRAMES: u32 = 6000;

/// 压力场景：每帧补满 Dew 与冷却、在下一个空格放一株双籽荚，并补足敌人。
fn stress_step(runner: &mut HeadlessRunner, frame: u32) {
    let world = runner.app_mut().world_mut();
    let Some(state) = world.resources.get_mut::<BattleState>() else {
        return;
    };
    if state.phase != Phase::Playing {
        return;
    }
    state.dew = 9990;
    let slot = world.resources.get_mut::<CardTray>().and_then(|tray| {
        for c in &mut tray.cards {
            c.cooldown.remaining = 0;
        }
        tray.cards
            .iter()
            .position(|c| c.defender.as_str() == "twinpod")
    });
    let cell = u8::try_from(frame % 45).unwrap_or(0);
    if let Some(slot) = slot {
        world.events.send(UiAction::SelectCard(slot));
        world.events.send(UiAction::PlaceAt {
            row: cell / Grid::COLS,
            col: cell % Grid::COLS,
        });
    }
    let alive = world.query::<(&BattleTag, &EnemyRef)>().iter().count();
    if alive < STRESS_ENEMIES
        && let Ok(id) = rootguard_core::EnemyId::new("drum_husk")
    {
        let row = u8::try_from(frame % u32::from(Grid::ROWS)).unwrap_or(0);
        let _ = rootguard_core::battle::spawn_enemy(world, id, row, Grid::SPAWN_X, None);
    }
}

fn main() {
    let skinned = std::env::args().any(|a| a == "--skinned");
    let stress = std::env::args().any(|a| a == "--stress");
    // 美术阶段：花园皮肤（特效、两层生产者、片段事件帧都只在 garden 中，ADR-016）
    let garden = std::env::args().any(|a| a == "--garden");
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/scripts/1-10.ron");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("无法读取 {}", path.display());
        std::process::exit(1);
    };
    let script = match Script::parse(&text) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("脚本解析失败: {e}");
            std::process::exit(1);
        }
    };
    let save = std::env::temp_dir().join(format!("rootguard-perf-{}", std::process::id()));
    let scene = BattleScene::new(script.level, script.cards.clone(), None);
    let mut runner = HeadlessRunner::new(build_app_full(scene, SaveLocation::Dir(save.clone())));
    runner.frame();
    if skinned {
        runner
            .app_mut()
            .world_mut()
            .events
            .send(UiAction::SetSkin(SkinMode::Skinned));
        if garden {
            runner
                .app_mut()
                .world_mut()
                .events
                .send(UiAction::SetSkinName(
                    rootguard_core::events::SkinName::Garden,
                ));
        }
    }
    let mut replay = Replay::new(script);
    let mut frame = 0_u32;
    // (帧耗时 µs, 敌人数, 防御者数, 弹丸数, 实体数)
    let mut samples: Vec<(u128, usize, usize, usize, usize)> = Vec::new();
    loop {
        let world = runner.app().world();
        let Some(state) = world.resources.get::<BattleState>().cloned() else {
            break;
        };
        if matches!(state.phase, Phase::Won | Phase::Lost) || (stress && frame >= STRESS_FRAMES) {
            println!("结束：{:?} 于 level_tick {}", state.phase, state.level_tick);
            break;
        }
        frame += 1;
        if stress {
            stress_step(&mut runner, frame);
        } else if state.phase == Phase::Playing {
            for action in replay.due(state.level_tick) {
                runner.app_mut().world_mut().events.send(action);
            }
        }
        let start = Instant::now();
        runner.frame();
        let micros = start.elapsed().as_micros();
        let world = runner.app().world();
        let enemies = world.query::<(&BattleTag, &EnemyRef)>().iter().count();
        let defenders = world.query::<(&BattleTag, &Cell)>().iter().count();
        let projectiles = world
            .query::<(&BattleTag, &ProjectileBody)>()
            .iter()
            .count();
        samples.push((
            micros,
            enemies,
            defenders,
            projectiles,
            world.entity_count(),
        ));
    }
    let _ = std::fs::remove_dir_all(save);
    let Some(peak) = samples.iter().map(|s| s.1).max() else {
        eprintln!("没有采样");
        std::process::exit(1);
    };
    // 最大场面：敌人数 ≥ 峰值 80% 的帧
    let mut heavy: Vec<_> = samples.iter().filter(|s| s.1 * 5 >= peak * 4).collect();
    heavy.sort_by_key(|s| s.0);
    let pct = |p: usize| heavy[(heavy.len() - 1) * p / 100].0;
    let worst = heavy.last().copied().copied().unwrap_or_default();
    let mut all: Vec<u128> = samples.iter().map(|s| s.0).collect();
    all.sort_unstable();
    println!(
        "模式 {}{}；共 {} 帧，全程 p50 {} µs、p99 {} µs、最大 {} µs",
        match (skinned, garden) {
            (true, true) => "Skinned（garden）",
            (true, false) => "Skinned（default）",
            _ => "Placeholder",
        },
        if stress { "（压力场景）" } else { "" },
        all.len(),
        all[all.len() / 2],
        all[(all.len() - 1) * 99 / 100],
        all[all.len() - 1]
    );
    println!(
        "最大场面（敌人 ≥ {} 的 {} 帧）：p50 {} µs、p99 {} µs、最大 {} µs（该帧敌人 {}、防御者 {}、弹丸 {}、实体 {}）",
        (peak * 4).div_ceil(5),
        heavy.len(),
        pct(50),
        pct(99),
        worst.0,
        worst.1,
        worst.2,
        worst.3,
        worst.4
    );
    let max_proj = samples.iter().map(|s| s.3).max().unwrap_or(0);
    let max_def = samples.iter().map(|s| s.2).max().unwrap_or(0);
    println!("峰值：敌人 {peak}、防御者 {max_def}、弹丸 {max_proj}");
}
