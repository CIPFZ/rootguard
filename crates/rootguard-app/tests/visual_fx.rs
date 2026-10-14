//! G5 视觉效果（SPEC 9 / 11.2 / 17 G5）：Dew 收集后飞向 HUD、过期的 Dew 不飞、横幅入场动画。
//! 这些效果只存在于呈现层（`RenderState`），模拟不受影响（快照一致性由 `tests/skin.rs` 与 golden 覆盖）。

mod common;

use common::{battle_runner, dew_positions, state};
use rootguard_app::render::RenderState;
use rootguard_core::components::DewPickup;
use rootguard_core::events::UiAction;
use rootguard_core::resources::Phase;
use trellia::prelude::*;

fn render_state(runner: &HeadlessRunner) -> &RenderState {
    runner
        .app()
        .world()
        .resources
        .get::<RenderState>()
        .expect("对局中应有呈现层状态")
}

#[test]
fn collected_dew_flies_to_hud() {
    let mut runner = battle_runner("1-1", &[]);
    // 等第一颗天空 Dew 出现
    let pos = loop {
        runner.frame();
        if let Some(p) = dew_positions(runner.app().world()).first() {
            break *p;
        }
        assert!(
            state(&runner).is_some_and(|s| s.level_tick < 3000),
            "3000 tick 内应出现天空 Dew"
        );
    };
    let before = state(&runner).expect("对局").dew;
    common::send(&mut runner, UiAction::CollectDewAt(pos));
    runner.frame();
    runner.frame();
    assert_eq!(state(&runner).expect("对局").dew, before + 25);
    assert_eq!(
        render_state(&runner).flying_dew(),
        1,
        "收集后应有一颗飞行中的 Dew"
    );
    for _ in 0..60 {
        runner.frame();
    }
    assert_eq!(render_state(&runner).flying_dew(), 0, "约 0.5 秒后飞行结束");
}

#[test]
fn expired_dew_does_not_fly() {
    let mut runner = battle_runner("1-1", &[]);
    let mut first: Option<Entity> = None;
    for _ in 0..4000 {
        runner.frame();
        assert_eq!(
            render_state(&runner).flying_dew(),
            0,
            "未收集的 Dew 不应飞向 HUD"
        );
        let world = runner.app().world();
        match first {
            None => {
                first = world
                    .query::<(Entity, &DewPickup)>()
                    .iter()
                    .map(|(e, _)| e)
                    .next()
            }
            Some(e) if world.get::<DewPickup>(e).is_none() => return,
            Some(_) => {}
        }
    }
    panic!("4000 帧内第一颗 Dew 应出现并过期");
}

#[test]
fn banner_slides_in() {
    let mut runner = battle_runner("1-1", &[]);
    assert_eq!(state(&runner).map(|s| s.phase), Some(Phase::Intro));
    runner.frame();
    let first = render_state(&runner)
        .banner_progress()
        .expect("Intro 期间显示“准备”横幅");
    assert!(first < 1.0, "横幅刚出现时应处于入场中（{first}）");
    for _ in 0..30 {
        runner.frame();
    }
    let later = render_state(&runner).banner_progress().expect("横幅");
    assert!(later > first, "入场进度应增加");
    assert!(
        (later - 1.0).abs() < f32::EPSILON,
        "0.5 秒后入场完成（{later}）"
    );
}
