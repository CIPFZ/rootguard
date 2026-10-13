//! 对局场景（SPEC 3.4）：拥有全部对局实体；FixedUpdate 调用 `battle_tick`（SPEC 6，ADR-002）。

use rootguard_core::components::DewPickup;
use rootguard_core::events::UiAction;
use rootguard_core::registry::DefenderRegistry;
use rootguard_core::resources::{BattleState, Phase, UiInbox};
use rootguard_core::{DefenderId, Grid, LevelId, battle_tick, end_battle, start_battle};
use trellia::egui;
use trellia::prelude::*;

use crate::scenes::{TitleScene, button_at, take_scene_actions};
use crate::script::{Replay, Script};
use crate::{hud, render};

/// 卡槽快捷键（SPEC 11.4：数字 1 ～ 8）。
const CARD_KEYS: [Key; 8] = [
    Key::Digit1,
    Key::Digit2,
    Key::Digit3,
    Key::Digit4,
    Key::Digit5,
    Key::Digit6,
    Key::Digit7,
    Key::Digit8,
];

/// 对局场景。
pub struct BattleScene {
    level: LevelId,
    cards: Vec<DefenderId>,
    seed: Option<u64>,
    outcome: Option<Phase>,
    autoplay: Option<Replay>,
}

impl BattleScene {
    /// 以关卡、所选卡牌（`Forced` 关忽略）与可选种子覆盖创建。
    #[must_use]
    pub fn new(level: LevelId, cards: Vec<DefenderId>, seed: Option<u64>) -> Self {
        Self {
            level,
            cards,
            seed,
            outcome: None,
            autoplay: None,
        }
    }

    /// 自动回放脚本解（调试用，`--autoplay`）：在 FixedUpdate 中按 `level_tick` 精确注入 `UiAction`，
    /// 与 headless 回放逐 tick 一致。
    #[must_use]
    pub fn with_autoplay(script: Script) -> Self {
        let mut scene = Self::new(script.level, script.cards.clone(), None);
        scene.autoplay = Some(Replay::new(script));
        scene
    }

    fn restart(&self, world: &mut World) {
        world
            .resources
            .expect_mut::<SceneCommands>()
            .replace(BattleScene::new(self.level, self.cards.clone(), self.seed));
        // 重开后不再自动回放
    }

    /// 鼠标与快捷键 → `UiAction`（SPEC 11.1 / 11.4）。只读模拟状态。
    fn translate_input(world: &mut World) -> Vec<UiAction> {
        let input = world.resources.expect::<Input>().clone();
        let Some(state) = world.resources.get::<BattleState>() else {
            return Vec::new();
        };
        let selected = state.selected_card.is_some() || state.shovel_mode;
        let shovel = state.shovel_mode;
        let mut out = Vec::new();
        for (i, key) in CARD_KEYS.iter().enumerate() {
            if input.key_just_pressed(*key) {
                out.push(UiAction::SelectCard(i));
            }
        }
        if input.key_just_pressed(Key::Escape) || input.mouse_just_pressed(MouseButton::Right) {
            if selected {
                out.push(UiAction::DeselectCard);
            } else if input.key_just_pressed(Key::Escape) {
                out.push(UiAction::PauseToggle);
            }
        }
        if input.mouse_just_pressed(MouseButton::Left)
            && let Some(pos) = input.mouse_pos
        {
            let dew_hit = world
                .query::<(&Transform2D, &DewPickup)>()
                .iter()
                .any(|(t, d)| d.amount > 0 && (t.pos - pos).length() <= Grid::DEW_COLLECT_RADIUS);
            if let Some(i) = hud::card_at(world, pos) {
                out.push(if state.selected_card == Some(i) {
                    UiAction::DeselectCard
                } else {
                    UiAction::SelectCard(i)
                });
            } else if dew_hit {
                out.push(UiAction::CollectDewAt(pos));
            } else if let Some((row, col)) = Grid::cell_at(pos) {
                if shovel {
                    out.push(UiAction::ShovelAt { row, col });
                } else if state.selected_card.is_some() {
                    out.push(UiAction::PlaceAt { row, col });
                }
            }
        }
        out
    }
}

impl Scene for BattleScene {
    fn name(&self) -> &'static str {
        "Battle"
    }

    fn on_enter(&mut self, world: &mut World) {
        if self.cards.is_empty() {
            // 未经选卡（命令行直达）：默认卡组为注册表顺序的全部防御者，由 start_battle 截取前 card_slots 张
            self.cards = world
                .resources
                .get::<DefenderRegistry>()
                .map(|d| d.iter().map(|(id, _)| id).collect())
                .unwrap_or_default();
        }
        if let Err(e) = start_battle(world, self.level, &self.cards) {
            log::warn!("无法开始关卡 {}: {e}", self.level);
            world
                .resources
                .expect_mut::<SceneCommands>()
                .replace(TitleScene);
            return;
        }
        if let Some(seed) = self.seed {
            world.resources.insert(GameRng::from_seed(seed));
        }
        world.resources.insert(render::RenderState::default());
    }

    fn on_exit(&mut self, world: &mut World) {
        end_battle(world);
        world.resources.remove::<BattleState>();
    }

    fn fixed_update(&mut self, world: &mut World) {
        if self.outcome.is_some() {
            return;
        }
        if let Some(replay) = self.autoplay.as_mut()
            && let Some(state) = world.resources.get::<BattleState>()
            && state.phase == Phase::Playing
        {
            let due = replay.due(state.level_tick);
            world.resources.expect_mut::<UiInbox>().0.extend(due);
        }
        battle_tick(world);
        let phase = world.resources.get::<BattleState>().map(|s| s.phase);
        if matches!(phase, Some(Phase::Won | Phase::Lost)) {
            self.outcome = phase;
        }
    }

    fn update(&mut self, world: &mut World) {
        render::track_hurt(world);
        for action in take_scene_actions(world) {
            match action {
                UiAction::Restart => self.restart(world),
                UiAction::BackToTitle => world
                    .resources
                    .expect_mut::<SceneCommands>()
                    .replace(TitleScene),
                _ => {}
            }
        }
        if self.outcome.is_some() {
            let input = world.resources.expect::<Input>();
            if input.key_just_pressed(Key::Enter) {
                world.events.send(UiAction::Restart);
            } else if input.key_just_pressed(Key::Escape) {
                world.events.send(UiAction::BackToTitle);
            }
            return;
        }
        for action in Self::translate_input(world) {
            world.events.send(action);
        }
    }

    fn draw(&mut self, world: &mut World) {
        render::draw_battle(world);
        let text = match self.outcome {
            Some(Phase::Won) => Some("VICTORY"),
            Some(Phase::Lost) => Some("DEFEAT"),
            _ => None,
        };
        if let Some(title) = text {
            render::overlay(
                world.resources.expect_mut::<DrawList>(),
                title,
                "Enter: play again    Esc: title",
            );
        }
    }

    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {
        if self.outcome.is_some() {
            return;
        }
        let shovel_on = world
            .resources
            .get::<BattleState>()
            .is_some_and(|s| s.shovel_mode);
        let label = if shovel_on { "Shovel*" } else { "Shovel" };
        if button_at(world, ctx, "hud_shovel", hud::SHOVEL_BUTTON, label) {
            world.events.send(UiAction::ToggleShovel);
        }
        if button_at(world, ctx, "hud_pause", hud::PAUSE_BUTTON, "Pause") {
            world.events.send(UiAction::PauseToggle);
        }
    }
}
