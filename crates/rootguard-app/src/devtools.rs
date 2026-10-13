//! Rootguard 调试窗口（SPEC 11.5，feature `devtools`）：与引擎调试面板一起由 F1 开关。
//!
//! 提供 Dew +100、跳到下一波、立即胜利 / 失败，并列出每个敌人的生命与状态；格子边界与敌人
//! 碰撞区间由 `render` 在 `DebugDraw` 开启时绘制。调试操作直接修改模拟资源，只用于开发（ADR-005）。

use rootguard_core::components::{Armor, EnemyRef, Health, Lane, StatusEffects, WalkState, Walker};
use rootguard_core::data::Rules;
use rootguard_core::resources::{BattleState, Phase, WaveState};
use trellia::egui;
use trellia::prelude::*;

/// 调试窗口开关键（与引擎面板相同，SPEC 11.4）。
const TOGGLE_KEY: Key = Key::F1;
/// “Dew +100” 的增量。
const DEW_BONUS: i32 = 100;

/// 资源：窗口是否打开。
#[derive(Default)]
struct DebugWindowState {
    open: bool,
}

fn toggle(world: &mut World) {
    if world
        .resources
        .expect::<Input>()
        .key_just_pressed(TOGGLE_KEY)
        && let Some(state) = world.resources.get_mut::<DebugWindowState>()
    {
        state.open = !state.open;
    }
}

fn state_label(w: &Walker) -> &'static str {
    match w.state {
        WalkState::Walking => "walk",
        WalkState::Eating(_) => "eat",
        WalkState::Vaulting { .. } => "vault",
        WalkState::Swallowed => "swallowed",
    }
}

fn window(world: &mut World, ctx: &egui::Context) {
    if !world
        .resources
        .get::<DebugWindowState>()
        .is_some_and(|s| s.open)
    {
        return;
    }
    let Some(battle) = world.resources.get::<BattleState>().cloned() else {
        return;
    };
    let mut rows: Vec<(u64, String)> = world
        .query::<(
            Entity,
            &EnemyRef,
            &Health,
            &Lane,
            &Transform2D,
            &Walker,
            &StatusEffects,
            Option<&Armor>,
        )>()
        .iter()
        .map(|(e, r, h, l, t, w, s, a)| {
            (
                e.to_bits().get(),
                format!(
                    "{:<14} L{} x{:>6.1} hp{:>3}{} {}{}",
                    r.id.as_str(),
                    l.row,
                    t.pos.x,
                    h.hp,
                    a.map_or(String::new(), |a| format!("+{}", a.hp)),
                    state_label(w),
                    if s.chilled() { " chill" } else { "" }
                ),
            )
        })
        .collect();
    rows.sort_by_key(|(id, _)| *id);
    let mut dew = false;
    let mut next_wave = false;
    let mut outcome = None;
    egui::Window::new("Rootguard Debug")
        .default_pos([900.0, 90.0])
        .show(ctx, |ui| {
            ui.label(format!(
                "level {}  tick {}  dew {}  phase {:?}",
                battle.level, battle.level_tick, battle.dew, battle.phase
            ));
            ui.horizontal(|ui| {
                dew = ui.button(format!("Dew +{DEW_BONUS}")).clicked();
                next_wave = ui.button("Next wave").clicked();
                if ui.button("Win").clicked() {
                    outcome = Some(Phase::Won);
                }
                if ui.button("Lose").clicked() {
                    outcome = Some(Phase::Lost);
                }
            });
            ui.separator();
            ui.label(format!("enemies: {}", rows.len()));
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for (_, row) in &rows {
                        ui.monospace(row);
                    }
                });
        });
    let cap = world
        .resources
        .get::<Rules>()
        .map_or(i32::MAX, |r| r.economy.dew_cap);
    if let Some(state) = world.resources.get_mut::<BattleState>() {
        if dew {
            state.dew = (state.dew + DEW_BONUS).min(cap);
        }
        if let Some(phase) = outcome
            && state.phase == Phase::Playing
        {
            state.phase = phase;
        }
    }
    if next_wave && let Some(waves) = world.resources.get_mut::<WaveState>() {
        waves.wave_timer.remaining = waves.wave_timer.remaining.min(1);
    }
}

/// SPEC 11.5 调试扩展插件（须在 `ScenePlugin` 之后添加，使窗口画在场景 UI 之上）。
pub struct RootguardDevtoolsPlugin;

impl Plugin for RootguardDevtoolsPlugin {
    fn name(&self) -> &'static str {
        "RootguardDevtoolsPlugin"
    }

    fn build(&self, app: &mut App) {
        app.insert_resource(DebugWindowState::default())
            .add_system(Stage::Update, toggle);
        trellia::scene::add_ui_hook(app, window);
    }
}
