//! SPEC 16.2 `determinism_golden_1_4`：1-4 脚本回放 30000 tick，快照与 `tests/golden/level_1_4.snap` 逐字节一致。
//!
//! 快照 = 引擎 `HeadlessRunner::snapshot()`（`Transform2D`）+ `rootguard_core::snapshot::write`（全部对局资源与组件）。
//! golden 只在人工核对后以 `ROOTGUARD_BLESS=1 cargo test -p rootguard-app --test golden` 生成；
//! 之后任何导致其变化的改动都必须先向负责人说明（AGENT.md 3.5 / 8）。

mod common;

use std::path::{Path, PathBuf};

use common::{next_id, runner_with, send, state};
use rootguard_app::BattleScene;
use rootguard_app::script::{Replay, Script};
use rootguard_core::resources::Phase;

const TICKS: u32 = 30_000;

fn golden_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/level_1_4.snap")
}

fn run() -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/scripts/1-4.ron");
    let script = Script::parse(&std::fs::read_to_string(path).expect("1-4 脚本")).expect("解析");
    let scene = BattleScene::new(script.level, script.cards.clone(), None);
    let mut runner = runner_with(scene, &format!("golden-{}", next_id()));
    let mut replay = Replay::new(script);
    for _ in 0..TICKS {
        if let Some(s) = state(&runner)
            && s.phase == Phase::Playing
        {
            for action in replay.due(s.level_tick) {
                send(&mut runner, action);
            }
        }
        runner.frame();
    }
    let mut bytes = runner.snapshot();
    bytes.extend(rootguard_core::snapshot::write(runner.app().world()));
    bytes
}

#[test]
fn determinism_golden_1_4() {
    let a = run();
    let b = run();
    assert_eq!(a, b, "两次回放快照必须逐字节相同");
    let path = golden_path();
    if std::env::var_os("ROOTGUARD_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().expect("父目录")).expect("创建目录");
        std::fs::write(&path, &a).expect("写入 golden");
        println!("已写入 {}（{} 字节）", path.display(), a.len());
        return;
    }
    let golden = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "缺少 golden {}: {e}（首次以 ROOTGUARD_BLESS=1 生成）",
            path.display()
        )
    });
    assert_eq!(a.len(), golden.len(), "快照长度与 golden 不同");
    assert!(a == golden, "快照与 golden 不一致");
}
