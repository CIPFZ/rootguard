//! FixedUpdate 系统（SPEC 第 6 章）。
//!
//! 由 `BattleScene::fixed_update` 调用 [`battle_tick`]，按 SPEC 6 的固定顺序执行全部系统
//! （ADR-002：选择"场景调用"而非"插件注册"）。只有 `phase == Playing` 时执行第 2 ～ 14 步。

mod combat;
mod damage;
mod economy;
mod input;
mod waves;

pub use combat::{
    bite_system, defender_behavior_system, projectile_system, status_system, walker_system,
};
pub use damage::{damage_system, death_system, lane_defense_system};
pub use economy::{economy_system, refresh_cards};
pub use input::{placement_system, ui_action_system};
pub use waves::{outcome_system, wave_system};

use trellia_core::{Entity, World};

use crate::components::{EnemyRef, Health, Lane, Walker};
use crate::resources::{BattleState, Phase};

/// SPEC 6 第 2 ～ 14 步（第 1 步 `intro_system` 单独执行）。
pub const PLAYING_SYSTEMS: [fn(&mut World); 13] = [
    ui_action_system,
    placement_system,
    economy_system,
    wave_system,
    status_system,
    walker_system,
    defender_behavior_system,
    projectile_system,
    bite_system,
    damage_system,
    death_system,
    lane_defense_system,
    outcome_system,
];

/// 推进一个对局 tick（SPEC 6）。无 `BattleState` 时什么也不做。
pub fn battle_tick(world: &mut World) {
    if !world.resources.contains::<BattleState>() {
        return;
    }
    intro_system(world);
    let state = world.resources.expect_mut::<BattleState>();
    if state.phase != Phase::Playing {
        return;
    }
    state.level_tick += 1;
    for system in PLAYING_SYSTEMS {
        system(world);
    }
}

/// SPEC 6 第 1 步：`Intro` 倒计时结束后进入 `Playing`。
pub fn intro_system(world: &mut World) {
    let Some(state) = world.resources.get_mut::<BattleState>() else {
        return;
    };
    if state.phase == Phase::Intro && (state.intro.finished() || state.intro.tick()) {
        state.phase = Phase::Playing;
    }
}

/// 一个存活敌人的快照（系统内部计算用）。
#[derive(Clone, Copy, Debug)]
pub(crate) struct EnemyView {
    pub entity: Entity,
    pub row: u8,
    pub x: f32,
}

/// 全部存活且未被吞食的敌人，按 `(x, Entity)` 升序（SPEC 2.5 同 tick 多目标的排序规则）。
pub(crate) fn alive_enemies(world: &World) -> Vec<EnemyView> {
    let mut out: Vec<EnemyView> = world
        .query::<(
            Entity,
            &EnemyRef,
            &Health,
            &Lane,
            &trellia_core::Transform2D,
            &Walker,
        )>()
        .iter()
        .filter(|(_, _, h, _, _, w)| {
            h.alive() && !matches!(w.state, crate::components::WalkState::Swallowed)
        })
        .map(|(entity, _, _, lane, t, _)| EnemyView {
            entity,
            row: lane.row,
            x: t.pos.x,
        })
        .collect();
    sort_targets(&mut out);
    out
}

/// 按 `(x, Entity id)` 升序排序（SPEC 2.5）。
pub(crate) fn sort_targets(list: &mut [EnemyView]) {
    list.sort_by(|a, b| {
        a.x.total_cmp(&b.x)
            .then(a.entity.to_bits().cmp(&b.entity.to_bits()))
    });
}
