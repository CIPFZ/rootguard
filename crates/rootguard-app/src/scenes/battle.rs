//! 对局场景（SPEC 3.4）：拥有全部对局实体；FixedUpdate 调用 `battle_tick`（SPEC 6，ADR-002）。

use rootguard_core::components::DewPickup;
use rootguard_core::events::UiAction;
use rootguard_core::registry::{DefenderRegistry, LevelRegistry};
use rootguard_core::resources::{BattleState, Phase, UiInbox};
use rootguard_core::{DefenderId, Grid, LevelId, battle_tick, end_battle, start_battle};
use trellia::egui;
use trellia::prelude::*;

use crate::i18n::t;
use crate::scenes::{OutcomeScene, TitleScene, button_styled, send};
use crate::script::{Replay, Script};
use crate::sfx::{self, Track};
use crate::{hud, render, save};

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
    ended: bool,
    /// 上方有覆盖层（暂停 / 设置）时为 true：HUD 按钮与鼠标翻译停用。
    covered: bool,
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
            ended: false,
            covered: false,
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

    /// 鼠标与快捷键 → `UiAction`（SPEC 11.1 / 11.4）。只读模拟状态。
    fn translate_input(world: &World) -> Vec<UiAction> {
        let input = world.resources.expect::<Input>();
        let Some(state) = world.resources.get::<BattleState>() else {
            return Vec::new();
        };
        let selected = state.selected_card.is_some() || state.shovel_mode;
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
                if state.shovel_mode {
                    out.push(UiAction::ShovelAt { row, col });
                } else if state.selected_card.is_some() {
                    out.push(UiAction::PlaceAt { row, col });
                }
            }
        }
        out
    }

    /// 对局结束：胜利时记录进度并保存（SPEC 14），推入结算场景。
    fn finish(&mut self, world: &mut World, won: bool) {
        self.ended = true;
        let unlocks = if won {
            let list = world
                .resources
                .get::<LevelRegistry>()
                .and_then(|l| l.get(self.level))
                .map(|l| l.unlock_on_win.clone())
                .unwrap_or_default();
            save::record_win(world, self.level, &list)
        } else {
            Vec::new()
        };
        let has_next = world
            .resources
            .get::<LevelRegistry>()
            .and_then(|l| l.next(self.level))
            .is_some();
        world
            .resources
            .expect_mut::<SceneCommands>()
            .push(OutcomeScene::new(won, unlocks, has_next));
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
        // 命令行直达时会话为空：补上，使“重试 / 下一关”可用
        if let Some(session) = world.resources.get_mut::<crate::flow::Session>()
            && session.level.is_none()
        {
            session.level = Some(self.level);
        }
        world.resources.insert(render::RenderState::default());
        sfx::play_music(world, Track::Battle);
    }

    fn on_pause(&mut self, _world: &mut World) {
        self.covered = true;
    }

    fn on_resume(&mut self, _world: &mut World) {
        self.covered = false;
    }

    fn on_exit(&mut self, world: &mut World) {
        end_battle(world);
        world.resources.remove::<BattleState>();
    }

    fn fixed_update(&mut self, world: &mut World) {
        if self.ended {
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
        match world.resources.get::<BattleState>().map(|s| s.phase) {
            Some(Phase::Won) => self.finish(world, true),
            Some(Phase::Lost) => self.finish(world, false),
            _ => {}
        }
    }

    fn update(&mut self, world: &mut World) {
        render::track_visuals(world);
        if self.ended {
            return;
        }
        for action in Self::translate_input(world) {
            send(world, action);
        }
    }

    fn draw(&mut self, world: &mut World) {
        render::draw_battle(world);
    }

    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {
        if self.ended || self.covered {
            return;
        }
        let shovel_on = world
            .resources
            .get::<BattleState>()
            .is_some_and(|s| s.shovel_mode);
        // 皮肤 HUD：铲子按钮只显示园丁小铁铲（DrawList 中），egui 按钮无边框、无文字（ART-SPEC 8.3）
        let skinned = hud::skinned_hud(crate::skin::Painter::active(world).as_ref());
        let mut label = if skinned {
            String::new()
        } else {
            t(world, "battle.shovel")
        };
        if shovel_on && !skinned {
            label.push_str(" *");
        }
        if button_styled(
            world,
            ctx,
            "hud_shovel",
            hud::SHOVEL_BUTTON,
            &label,
            true,
            !skinned,
        ) {
            send(world, UiAction::ToggleShovel);
        }
        if button_styled(
            world,
            ctx,
            "hud_pause",
            hud::PAUSE_BUTTON,
            &t(world, "battle.pause"),
            true,
            !skinned,
        ) {
            send(world, UiAction::PauseToggle);
        }
    }
}
