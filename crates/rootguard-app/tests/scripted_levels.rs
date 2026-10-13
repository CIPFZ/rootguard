//! SPEC 16.2 `every_level_has_scripted_solution`：每关 `tests/scripts/<id>.ron` 回放必胜。
//!
//! 运行：`cargo test -p rootguard-app --test scripted_levels`
//!
//! 录制（重新生成脚本，需人工审阅 diff）：
//! `ROOTGUARD_RECORD=1 cargo test -p rootguard-app --test scripted_levels record_scripts -- --ignored --nocapture`

mod common;

use std::path::{Path, PathBuf};

use common::bot::{Bot, BotConfig};
use common::{did, lid, send, state};
use rootguard_app::script::{Replay, Script, ScriptAction, Step};
use rootguard_app::{BattleScene, build_app_with};
use rootguard_core::registry::LevelRegistry;
use rootguard_core::resources::Phase;
use trellia::prelude::*;

fn scripts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/scripts")
}

fn runner_for(script: &Script) -> HeadlessRunner {
    let scene = BattleScene::new(script.level, script.cards.clone(), None);
    let mut runner = HeadlessRunner::new(build_app_with(scene));
    runner.frame();
    runner
}

/// 回放结果：(最终阶段, 结束时的 level_tick)。
fn replay(script: &Script) -> (Option<Phase>, u64) {
    let mut runner = runner_for(script);
    let mut cursor = Replay::new(script.clone());
    loop {
        let Some(s) = state(&runner) else {
            return (None, 0);
        };
        if matches!(s.phase, Phase::Won | Phase::Lost) || s.level_tick > script.max_ticks {
            return (Some(s.phase), s.level_tick);
        }
        if s.phase == Phase::Playing {
            for action in cursor.due(s.level_tick) {
                send(&mut runner, action);
            }
        }
        runner.frame();
    }
}

fn level_ids() -> Vec<String> {
    let app = build_app_with(rootguard_app::TitleScene);
    app.world()
        .resources
        .expect::<LevelRegistry>()
        .iter()
        .map(|l| l.id.as_str().to_owned())
        .collect()
}

#[test]
fn every_level_has_scripted_solution() {
    let ids = level_ids();
    assert!(!ids.is_empty());
    for id in ids {
        let path = scripts_dir().join(format!("{id}.ron"));
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("关卡 {id} 缺少脚本解 {}: {e}", path.display()));
        let script = Script::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(script.level.as_str(), id, "{}", path.display());
        let (phase, tick) = replay(&script);
        assert_eq!(
            phase,
            Some(Phase::Won),
            "关卡 {id} 脚本回放未胜利（tick {tick}）"
        );
        println!("{id}: 胜利于 level_tick {tick}");
    }
}

/// 每关的录制参数：选卡与机器人配置。
fn record_plan(id: &str) -> (Vec<&'static str>, BotConfig) {
    let cfg = BotConfig::default();
    match id {
        "1-1" | "1-2" | "1-3" => (vec![], cfg),
        _ => (vec![], cfg),
    }
}

/// 用机器人录制一关，返回脚本与结果。
fn record(
    id: &str,
    cards: &[&str],
    cfg: BotConfig,
    max_ticks: u64,
) -> (Script, Option<Phase>, u64) {
    let mut script = Script {
        level: lid(id),
        cards: cards.iter().map(|c| did(c)).collect(),
        max_ticks,
        steps: Vec::new(),
    };
    let mut runner = runner_for(&script);
    let mut bot = Bot::new(cfg);
    loop {
        let Some(s) = state(&runner) else {
            return (script, None, 0);
        };
        if matches!(s.phase, Phase::Won | Phase::Lost) || s.level_tick > max_ticks {
            return (script, Some(s.phase), s.level_tick);
        }
        let actions = bot.decide(runner.app().world());
        if !actions.is_empty() {
            script.steps.push(Step {
                tick: s.level_tick,
                actions: actions.iter().filter_map(ScriptAction::from_ui).collect(),
            });
            for a in actions {
                send(&mut runner, a);
            }
        }
        runner.frame();
    }
}

#[test]
#[ignore = "录制工具：ROOTGUARD_RECORD=1 时重写 tests/scripts/*.ron"]
fn record_scripts() {
    let only = std::env::var("ROOTGUARD_LEVEL").ok();
    for id in level_ids() {
        if only.as_deref().is_some_and(|o| o != id) {
            continue;
        }
        let (cards, cfg) = record_plan(&id);
        let (script, phase, tick) = record(&id, &cards, cfg, 60_000);
        println!(
            "{id}: 录制结果 {phase:?} 于 level_tick {tick}，{} 步",
            script.steps.len()
        );
        if phase == Some(Phase::Won) && std::env::var_os("ROOTGUARD_RECORD").is_some() {
            let script = Script {
                max_ticks: tick + 600,
                ..script
            };
            let (check, _) = replay(&script);
            assert_eq!(check, Some(Phase::Won), "{id}: 录制后回放不一致");
            let text = script.to_ron().expect("序列化");
            std::fs::create_dir_all(scripts_dir()).expect("创建目录");
            std::fs::write(scripts_dir().join(format!("{id}.ron")), text).expect("写入脚本");
        }
    }
}
