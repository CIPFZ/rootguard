//! SPEC 16.2 `every_level_has_scripted_solution`：每关 `tests/scripts/<id>.ron` 回放必胜。
//!
//! 运行：`cargo test -p rootguard-app --test scripted_levels`
//!
//! 录制（重新生成脚本，需人工审阅 diff）：
//! `ROOTGUARD_RECORD=1 cargo test -p rootguard-app --test scripted_levels record_scripts -- --ignored --nocapture`

mod common;

use std::path::{Path, PathBuf};

use common::bot::{Bot, BotConfig};
use common::{did, hoses_used, lid, next_id, runner_with, send, state};
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
    let tag = format!("script-{}-{}", script.level, next_id());
    let mut runner = runner_with(scene, &tag);
    runner.frame();
    runner
}

/// 回放结果：(最终阶段, 结束时的 level_tick, 已触发的 Hose 数)。
fn replay(script: &Script) -> (Option<Phase>, u64, usize) {
    let mut runner = runner_for(script);
    let mut cursor = Replay::new(script.clone());
    loop {
        let Some(s) = state(&runner) else {
            return (None, 0, 0);
        };
        if matches!(s.phase, Phase::Won | Phase::Lost) || s.level_tick > script.max_ticks {
            return (Some(s.phase), s.level_tick, hoses_used(&runner));
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

/// 进入第 `index` 关时已解锁的卡牌：初始卡牌 + 之前各关的 `unlock_on_win`。
fn unlocked_before(index: usize) -> Vec<String> {
    let app = build_app_with(rootguard_app::TitleScene);
    let world = app.world();
    let mut out: Vec<String> = world
        .resources
        .expect::<rootguard_core::data::Rules>()
        .starting_cards
        .iter()
        .map(|d| d.as_str().to_owned())
        .collect();
    for level in world.resources.expect::<LevelRegistry>().iter().take(index) {
        out.extend(level.unlock_on_win.iter().map(|d| d.as_str().to_owned()));
    }
    out
}

/// 读取关卡 `id` 的脚本解。
fn load_script(id: &str) -> Script {
    let path = scripts_dir().join(format!("{id}.ron"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("关卡 {id} 缺少脚本解 {}: {e}", path.display()));
    let script = Script::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(script.level.as_str(), id, "{}", path.display());
    script
}

#[test]
fn every_level_has_scripted_solution() {
    let ids = level_ids();
    assert!(!ids.is_empty());
    for (index, id) in ids.into_iter().enumerate() {
        let script = load_script(&id);
        let unlocked = unlocked_before(index);
        for card in &script.cards {
            assert!(
                unlocked.iter().any(|u| u == card.as_str()),
                "关卡 {id} 的脚本使用了尚未解锁的卡牌 {card}"
            );
        }
        let (phase, tick, hoses) = replay(&script);
        assert_eq!(
            phase,
            Some(Phase::Won),
            "关卡 {id} 脚本回放未胜利（tick {tick}）"
        );
        println!("{id}: 胜利于 level_tick {tick}，Hose {hoses}");
    }
}

/// 每关的录制参数：选卡（须为当时已解锁的卡牌）与机器人配置。参数由离线参数搜索得到。
fn record_plan(id: &str) -> (Vec<&'static str>, BotConfig) {
    let base = BotConfig::default();
    let tuned = |focus: f32, min_shooters: usize, producers_per_lane: usize| BotConfig {
        producers_per_lane,
        econ_until: 14_000,
        wall_min_power: 1.0,
        focus,
        min_shooters,
        mine_min_hp: 15,
        ..BotConfig::default()
    };
    match id {
        "1-1" | "1-2" | "1-3" => (vec![], base),
        "1-4" => (
            vec!["pebble_pod", "dewcup", "barkwall", "frostpod"],
            PLAN_1_4.cfg(tuned),
        ),
        "1-5" => (
            vec!["pebble_pod", "dewcup", "barkwall", "frostpod", "twinpod"],
            PLAN_1_5.cfg(tuned),
        ),
        "1-6" => (
            vec![
                "pebble_pod",
                "dewcup",
                "barkwall",
                "frostpod",
                "twinpod",
                "thornmat",
            ],
            tuned(25.0, 1, 2),
        ),
        "1-7" => (
            vec![
                "twinpod",
                "dewcup",
                "barkwall",
                "frostpod",
                "pebble_pod",
                "thornmat",
                "tuberblast",
            ],
            BotConfig {
                mine_min_hp: 30,
                ..tuned(40.0, 1, 2)
            },
        ),
        "1-8" => (
            vec![
                "twinpod",
                "dewcup",
                "barkwall",
                "frostpod",
                "thornmat",
                "tuberblast",
                "emberbulb",
            ],
            tuned(40.0, 1, 2),
        ),
        "1-9" => (
            vec![
                "twinpod",
                "dewcup",
                "barkwall",
                "frostpod",
                "pebble_pod",
                "thornmat",
                "tuberblast",
                "emberbulb",
            ],
            tuned(60.0, 2, 1),
        ),
        _ => (vec![], base),
    }
}

/// 一组搜索得到的参数：(focus, min_shooters, producers_per_lane)。
struct Plan(f32, usize, usize);

impl Plan {
    fn cfg(&self, f: impl Fn(f32, usize, usize) -> BotConfig) -> BotConfig {
        f(self.0, self.1, self.2)
    }
}

const PLAN_1_4: Plan = Plan(25.0, 1, 1);
const PLAN_1_5: Plan = Plan(40.0, 2, 1);

/// 录制结果：脚本、最终阶段、结束时的 level_tick、已触发的 Hose 数。
struct Recorded {
    script: Script,
    phase: Option<Phase>,
    tick: u64,
    hoses: usize,
}

/// 用机器人录制一关。
fn record(id: &str, cards: &[&str], cfg: BotConfig, max_ticks: u64) -> Recorded {
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
            return Recorded {
                script,
                phase: None,
                tick: 0,
                hoses: 0,
            };
        };
        if matches!(s.phase, Phase::Won | Phase::Lost) || s.level_tick > max_ticks {
            return Recorded {
                script,
                phase: Some(s.phase),
                tick: s.level_tick,
                hoses: hoses_used(&runner),
            };
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

/// 录制的最长 tick。
const RECORD_MAX_TICKS: u64 = 60_000;

#[test]
#[ignore = "录制工具：ROOTGUARD_RECORD=1 时重写 tests/scripts/*.ron"]
fn record_scripts() {
    let only = std::env::var("ROOTGUARD_LEVEL").ok();
    for id in level_ids() {
        if only.as_deref().is_some_and(|o| o != id) {
            continue;
        }
        let (cards, cfg) = record_plan(&id);
        let r = record(&id, &cards, cfg, RECORD_MAX_TICKS);
        println!(
            "{id}: 录制结果 {:?} 于 level_tick {}，Hose {}，{} 步",
            r.phase,
            r.tick,
            r.hoses,
            r.script.steps.len()
        );
        // SPEC 10.3：Hose 超过 1 次的录制不合格，不写入
        if r.phase == Some(Phase::Won)
            && r.hoses <= 1
            && std::env::var_os("ROOTGUARD_RECORD").is_some()
        {
            let script = Script {
                max_ticks: r.tick + 600,
                ..r.script
            };
            let (check, _, _) = replay(&script);
            assert_eq!(check, Some(Phase::Won), "{id}: 录制后回放不一致");
            let text = script.to_ron().expect("序列化");
            std::fs::create_dir_all(scripts_dir()).expect("创建目录");
            std::fs::write(scripts_dir().join(format!("{id}.ron")), text).expect("写入脚本");
        }
    }
}

/// 参数搜索的候选卡组（须为进入该关时已解锁的卡牌，且不超过卡槽数）。
fn search_decks(id: &str) -> Vec<Vec<&'static str>> {
    match id {
        "1-7" => vec![
            vec![
                "twinpod",
                "dewcup",
                "barkwall",
                "frostpod",
                "pebble_pod",
                "thornmat",
                "tuberblast",
            ],
            vec![
                "twinpod",
                "dewcup",
                "barkwall",
                "frostpod",
                "thornmat",
                "tuberblast",
            ],
            vec![
                "twinpod",
                "dewcup",
                "barkwall",
                "frostpod",
                "pebble_pod",
                "tuberblast",
            ],
        ],
        "1-8" => vec![
            vec![
                "twinpod",
                "dewcup",
                "barkwall",
                "frostpod",
                "thornmat",
                "tuberblast",
                "emberbulb",
            ],
            vec![
                "twinpod",
                "dewcup",
                "barkwall",
                "frostpod",
                "pebble_pod",
                "tuberblast",
                "emberbulb",
            ],
            vec![
                "twinpod",
                "dewcup",
                "barkwall",
                "frostpod",
                "pebble_pod",
                "thornmat",
                "emberbulb",
            ],
        ],
        "1-9" => vec![
            vec![
                "twinpod",
                "dewcup",
                "barkwall",
                "frostpod",
                "pebble_pod",
                "thornmat",
                "tuberblast",
                "emberbulb",
            ],
            vec![
                "twinpod",
                "dewcup",
                "barkwall",
                "frostpod",
                "thornmat",
                "tuberblast",
                "emberbulb",
                "gourdshell",
            ],
            vec![
                "twinpod",
                "dewcup",
                "barkwall",
                "frostpod",
                "pebble_pod",
                "tuberblast",
                "emberbulb",
                "gourdshell",
            ],
        ],
        "1-10" => vec![
            vec![
                "twinpod",
                "sunlantern",
                "dewcup",
                "barkwall",
                "frostpod",
                "tuberblast",
                "emberbulb",
                "snapmaw",
            ],
            vec![
                "twinpod",
                "dewcup",
                "barkwall",
                "frostpod",
                "thornmat",
                "tuberblast",
                "emberbulb",
                "snapmaw",
            ],
            vec![
                "twinpod",
                "sunlantern",
                "dewcup",
                "barkwall",
                "frostpod",
                "thornmat",
                "tuberblast",
                "emberbulb",
            ],
        ],
        _ => vec![record_plan(id).0],
    }
}

/// 参数搜索的机器人参数网格（AGENT 3.4 建议 ≥ 2 套卡组 × ≥ 8 组参数）。
fn search_grid() -> Vec<BotConfig> {
    let mut out = Vec::new();
    for focus in [12.0, 25.0, 40.0, 60.0] {
        for min_shooters in [1, 2] {
            for producers_per_lane in [1, 2] {
                for econ_until in [8_000, 14_000] {
                    for wall_first in [false, true] {
                        for save_for_shooters in [false, true] {
                            for wall_min_power in [1.0, 2.0] {
                                for emergency_x in [560.0, 760.0] {
                                    out.push(BotConfig {
                                        producers_per_lane,
                                        econ_until,
                                        emergency_x,
                                        wall_min_power,
                                        wall_first,
                                        focus,
                                        min_shooters,
                                        mine_min_hp: 15,
                                        save_for_shooters,
                                        ..BotConfig::default()
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

/// 参数搜索（AGENT 3.4）：`ROOTGUARD_LEVEL=<id>` 时对 `search_decks × search_grid` 逐一录制，
/// 按“胜利且 Hose ≤ 1 → Hose 少 → 用时短”打印前若干名；不写文件，选中的参数手工填入 `record_plan`。
///
/// `ROOTGUARD_LEVEL=1-9 cargo test --release -p rootguard-app --test scripted_levels search_plans -- --ignored --nocapture`
#[test]
#[ignore = "参数搜索工具：需要 ROOTGUARD_LEVEL"]
fn search_plans() {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let Ok(id) = std::env::var("ROOTGUARD_LEVEL") else {
        println!("未设置 ROOTGUARD_LEVEL，跳过");
        return;
    };
    let decks = search_decks(&id);
    let grid = search_grid();
    let jobs: Vec<(usize, usize)> = (0..decks.len())
        .flat_map(|d| (0..grid.len()).map(move |g| (d, g)))
        .collect();
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(d, g)) = jobs.get(i) else { break };
                    let r = record(&id, &decks[d], grid[g].clone(), RECORD_MAX_TICKS);
                    results
                        .lock()
                        .expect("结果锁")
                        .push((d, g, r.phase, r.tick, r.hoses));
                }
            });
        }
    });
    let mut results = results.into_inner().expect("结果锁");
    let won = |p: &Option<Phase>| *p == Some(Phase::Won);
    results.sort_by_key(|&(d, g, phase, tick, hoses)| {
        let ok = won(&phase) && hoses <= 1;
        (
            !ok,
            !won(&phase),
            hoses,
            if won(&phase) { tick } else { u64::MAX - tick },
            d,
            g,
        )
    });
    let pass = results.iter().filter(|r| won(&r.2) && r.4 <= 1).count();
    println!(
        "{id}: {} 套卡组 × {} 组参数，合格（胜利且 Hose ≤ 1）{pass} 个，胜利 {} 个",
        decks.len(),
        grid.len(),
        results.iter().filter(|r| won(&r.2)).count()
    );
    for &(d, g, phase, tick, hoses) in results.iter().take(12) {
        let c = &grid[g];
        println!(
            "  卡组 {d} 参数 {g:>3}（focus {} min_shooters {} producers {} econ_until {} wall_first {} save {} wall_min_power {} emergency_x {}）：{phase:?} tick {tick} Hose {hoses}",
            c.focus,
            c.min_shooters,
            c.producers_per_lane,
            c.econ_until,
            c.wall_first,
            c.save_for_shooters,
            c.wall_min_power,
            c.emergency_x
        );
    }
}
