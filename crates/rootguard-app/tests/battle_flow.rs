//! SPEC 16.2 的 G2 验收：1-1 脚本胜利、1-1 挂机失败、占位渲染覆盖全部实体。

mod common;

use common::{battle_runner, card_ready, dew_positions, occupied, send, state, top};
use rootguard_core::components::BattleTag;
use rootguard_core::events::UiAction;
use rootguard_core::resources::{LaneState, Phase};
use trellia::prelude::*;

#[test]
fn level_1_1_scripted_win() {
    let mut runner = battle_runner("1-1", &[]);
    let mut won_at = None;
    for _ in 0..20_000 {
        let Some(s) = state(&runner) else { break };
        if s.phase == Phase::Won {
            won_at = Some(s.level_tick);
            break;
        }
        assert_ne!(s.phase, Phase::Lost, "1-1 脚本不应失败");
        let world = runner.app().world();
        let dews = dew_positions(world);
        // 第 60 tick 放 pebble_pod 于 (2,0)；之后每次 ready 且 Dew 够时依次放在 (2,1..4)
        let target = if s.level_tick == 60 {
            Some(0)
        } else if s.level_tick > 60 && card_ready(world, 0) {
            (1..=4).find(|&c| !occupied(world, 2, c))
        } else {
            None
        };
        for pos in dews {
            send(&mut runner, UiAction::CollectDewAt(pos));
        }
        if let Some(col) = target {
            send(&mut runner, UiAction::SelectCard(0));
            send(&mut runner, UiAction::PlaceAt { row: 2, col });
        }
        runner.frame();
    }
    let won_at = won_at.expect("20000 tick 内 BattleEnded(Won)");
    assert!(won_at <= 20_000);
    println!("1-1 脚本胜利于 level_tick {won_at}");
}

#[test]
fn level_1_1_idle_lose() {
    // 事件由 SfxPlugin 在当帧消费，这里通过状态断言：先用掉 Hose，再越线失败
    let mut runner = battle_runner("1-1", &[]);
    let mut hose_used_at = None;
    for _ in 0..40_000 {
        runner.frame();
        let world = runner.app().world();
        let hose_left = world
            .resources
            .get::<LaneState>()
            .is_some_and(|l| l.hose_available[2]);
        let s = state(&runner).expect("对局");
        if !hose_left && hose_used_at.is_none() {
            hose_used_at = Some(s.level_tick);
        }
        if s.phase == Phase::Lost {
            let used = hose_used_at.expect("失败前先触发 Hose");
            assert!(used < s.level_tick, "Hose 先于失败");
            assert_eq!(top(&runner), Some("Defeat"));
            return;
        }
        assert_ne!(s.phase, Phase::Won);
    }
    panic!("40000 帧内未失败");
}

#[test]
fn placeholder_mode_draws_all_entities() {
    let mut runner = battle_runner("1-1", &[]);
    let mut checked = 0;
    for _ in 0..6000 {
        let s = state(&runner).expect("对局中");
        if s.level_tick == 30 || s.level_tick.is_multiple_of(700) {
            send(&mut runner, UiAction::SelectCard(0));
            let col = (0..9).find(|&c| !occupied(runner.app().world(), 2, c));
            if let Some(col) = col {
                send(&mut runner, UiAction::PlaceAt { row: 2, col });
            }
        }
        runner.frame();
        let entities = runner.app().world().query::<&BattleTag>().iter().count();
        let shapes = runner
            .last_draw_list()
            .iter()
            .filter(|c| matches!(c.kind, DrawKind::Rect { .. } | DrawKind::Circle { .. }))
            .count();
        assert!(shapes >= entities, "形状数 {shapes} < 实体数 {entities}");
        if entities > 3 {
            checked += 1;
        }
    }
    assert!(checked > 100, "应在有多个实体的帧上检查（{checked}）");
}
