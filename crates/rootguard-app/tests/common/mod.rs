//! 集成测试公共夹具：以 `HeadlessRunner` 驱动完整 App，只通过 `UiAction` 操作（SPEC 11.1 / 16.2）。

#![allow(dead_code)]

use rootguard_app::save::SaveLocation;
use rootguard_app::{BattleScene, build_app_full};
use rootguard_core::components::{Cell, DefenderRef, DewPickup, Health, OverlayOf};
use rootguard_core::events::UiAction;
use rootguard_core::resources::{BattleState, CardTray, LaneState, Phase};
use rootguard_core::{DefenderId, LevelId};
use trellia::prelude::*;

pub fn lid(s: &str) -> LevelId {
    LevelId::new(s).expect("关卡 id")
}

pub fn did(s: &str) -> DefenderId {
    DefenderId::new(s).expect("防御者 id")
}

/// 测试专用的空存档目录（进程号 + 标签，避免并行测试互相影响，且不触碰用户目录）。
pub fn temp_save(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rootguard-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// 以指定初始场景与独立存档目录创建 headless 运行器。
pub fn runner_with(scene: impl Scene, save_tag: &str) -> HeadlessRunner {
    HeadlessRunner::new(build_app_full(
        scene,
        SaveLocation::Dir(temp_save(save_tag)),
    ))
}

/// 直接进入对局的 headless 运行器（每帧恰好 1 tick）。
pub fn battle_runner(level: &str, cards: &[&str]) -> HeadlessRunner {
    let cards = cards.iter().map(|c| did(c)).collect();
    let tag = format!("battle-{level}-{}", next_id());
    let mut runner = runner_with(BattleScene::new(lid(level), cards, None), &tag);
    runner.frame();
    runner
}

/// 每次调用递增的计数（同一进程内区分临时目录）。
pub fn next_id() -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// 顶层场景名。
pub fn top(runner: &HeadlessRunner) -> Option<&'static str> {
    runner
        .app()
        .world()
        .resources
        .get::<SceneStack>()
        .and_then(SceneStack::top_name)
}

pub fn state(runner: &HeadlessRunner) -> Option<BattleState> {
    runner.app().world().resources.get::<BattleState>().cloned()
}

pub fn phase(runner: &HeadlessRunner) -> Option<Phase> {
    state(runner).map(|s| s.phase)
}

pub fn send(runner: &mut HeadlessRunner, action: UiAction) {
    runner.app_mut().world_mut().events.send(action);
}

/// 当前全部可收集 Dew 的位置。
pub fn dew_positions(world: &World) -> Vec<Vec2> {
    let mut v: Vec<(Entity, Vec2)> = world
        .query::<(Entity, &Transform2D, &DewPickup)>()
        .iter()
        .filter(|(_, _, d)| d.amount > 0)
        .map(|(e, t, _)| (e, t.pos))
        .collect();
    v.sort_by_key(|(e, _)| e.to_bits());
    v.into_iter().map(|(_, p)| p).collect()
}

/// 格子是否已有存活的地面防御者。
pub fn occupied(world: &World, row: u8, col: u8) -> bool {
    world
        .query::<(&Cell, &Health, &DefenderRef)>()
        .without::<&OverlayOf>()
        .iter()
        .any(|(c, h, _)| c.row == row && c.col == col && h.alive())
}

/// 第 `slot` 张卡是否就绪。
pub fn card_ready(world: &World, slot: usize) -> bool {
    world
        .resources
        .get::<CardTray>()
        .and_then(|t| t.cards.get(slot))
        .is_some_and(|c| c.ready)
}

/// 已触发的 Hose Burst 数（SPEC 10.3）。
///
/// `HoseTriggered` 由 `SfxPlugin` 在当帧取走（ADR-004 第 4 条），帧结束后测试读不到事件；
/// `lane_defense_system` 发出该事件当且仅当把某条激活车道的 `hose_available` 置为 false，
/// 且每条车道至多一次，因此“激活车道中已用掉的 Hose 数”与事件数一一对应。
pub fn hoses_used(runner: &HeadlessRunner) -> usize {
    runner
        .app()
        .world()
        .resources
        .get::<LaneState>()
        .map_or(0, |l| {
            l.active_rows()
                .into_iter()
                .filter(|&r| !l.hose_available[usize::from(r)])
                .count()
        })
}
pub mod bot;
