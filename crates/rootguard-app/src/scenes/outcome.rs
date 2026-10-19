//! 结算覆盖层（SPEC 3.4 / 11.3）：胜利显示解锁的新卡；下一关 / 重试 / 返回。

use rootguard_core::DefenderId;
use rootguard_core::events::UiAction;
use trellia::egui;
use trellia::prelude::*;

use crate::i18n::t;
use crate::scenes::{draw_wood_buttons, menu_button, menu_slot, send};
use crate::sfx::{self, Track};
use crate::{LOGICAL_SIZE, render};

/// 结算场景。
pub struct OutcomeScene {
    won: bool,
    unlocked: Vec<DefenderId>,
    has_next: bool,
    ending: bool,
}

/// 结局静帧（ART-SPEC 8.4 / LORE 结局）：背层、前层与两只角色的位置。
pub const ENDING_BACK: &str = "story_ending_back";
const ENDING_FRONT: &str = "story_ending_front";
/// 前层（门口的枯土埂）左上角。
const ENDING_FRONT_POS: Vec2 = Vec2::new(950.0, 400.0);
/// 日灯笼：脚底位置与放大倍数（光束从它的灯笼照向裂缝）。
const ENDING_LANTERN: (Vec2, f32) = (Vec2::new(330.0, 640.0), 1.6);
/// 小芽怪：躲在土埂后只探出头。
const ENDING_SPROUT: (Vec2, f32) = (Vec2::new(1080.0, 432.0), 1.6);
/// 日灯笼的灯笼叠层：最亮的一帧（第 4 次产出的金色，ART-SPEC 4.12），身体为 idle（两层绘制，ADR-016 第 2 节）。
const ENDING_LANTERN_LIGHT: (&str, u64) = ("sunlantern_produce_gold", 1400);
/// 结局一句的位置（底部木牌）。
const ENDING_LINE_POS: Vec2 = Vec2::new(640.0, 676.0);

impl OutcomeScene {
    /// 创建；`unlocked` 为本次新解锁的卡牌。
    #[must_use]
    pub fn new(won: bool, unlocked: Vec<DefenderId>, has_next: bool) -> Self {
        Self {
            won,
            unlocked,
            has_next,
            ending: false,
        }
    }

    /// 最后一关胜利：显示结局静帧与 `ending.line`（ART-SPEC 8.4）。
    #[must_use]
    pub fn with_ending(mut self, ending: bool) -> Self {
        self.ending = ending && self.won;
        self
    }

    /// 结局静帧（皮肤模式且皮肤有结局画面时）；返回是否画了。
    fn draw_ending(world: &mut World, line: &str) -> bool {
        let Some(mut list) = world.resources.remove::<DrawList>() else {
            return false;
        };
        let drew = crate::skin::Painter::active(world).is_some_and(|sk| {
            let full = Rect::from_min_size(Vec2::ZERO, LOGICAL_SIZE);
            if !sk.draw_in(
                &mut list,
                Layer::OVERLAY,
                ENDING_BACK,
                full,
                1.0,
                Color::WHITE,
            ) {
                return false;
            }
            let still = |clip| crate::skin::Pose {
                clip,
                elapsed: 0,
                flash: false,
                tint: Color::WHITE,
            };
            let (at, k) = ENDING_LANTERN;
            sk.draw_scaled(
                &mut list,
                Layer::OVERLAY,
                at,
                0.0,
                still("sunlantern_idle"),
                k,
            );
            let (light, elapsed) = ENDING_LANTERN_LIGHT;
            let lit = crate::skin::Pose {
                elapsed,
                ..still(light)
            };
            sk.draw_scaled(&mut list, Layer::OVERLAY, at, 0.0, lit, k);
            let (at, k) = ENDING_SPROUT;
            sk.draw_scaled(
                &mut list,
                Layer::OVERLAY,
                at,
                0.0,
                still("sproutling_walk"),
                k,
            );
            if let Some(size) = sk.frame_size(ENDING_FRONT) {
                let front = Rect::from_min_size(ENDING_FRONT_POS, size);
                sk.draw_in(
                    &mut list,
                    Layer::OVERLAY,
                    ENDING_FRONT,
                    front,
                    1.0,
                    Color::WHITE,
                );
            }
            if let Some(size) = sk.frame_size(crate::hud::UI_BANNER) {
                let board = Rect::from_center_size(ENDING_LINE_POS, size);
                sk.draw_in(
                    &mut list,
                    Layer::OVERLAY,
                    crate::hud::UI_BANNER,
                    board,
                    1.0,
                    Color::WHITE,
                );
            }
            list.text_aligned(
                Layer::OVERLAY,
                ENDING_LINE_POS - Vec2::new(0.0, 10.0),
                line,
                20.0,
                Color::from_hex(0xFF_F59D),
                TextAlign::Center,
            );
            true
        });
        world.resources.insert(list);
        drew
    }

    /// 第 `i` 个菜单按钮。结局静帧：按钮放在玻璃墙的空处，不挡住光束、根须与小芽怪。
    fn slot(&self, i: usize) -> Rect {
        let top = if self.ending { 250.0 } else { 440.0 };
        menu_slot(LOGICAL_SIZE.x / 2.0, top, 66.0, i)
    }

    /// 按钮数：下一关（胜利且有下一关）、重试、返回。
    fn buttons(&self) -> usize {
        if self.won && self.has_next { 3 } else { 2 }
    }

    fn primary(&self) -> UiAction {
        if self.won && self.has_next {
            UiAction::NextLevel
        } else if self.won {
            UiAction::BackToLevelSelect
        } else {
            UiAction::Restart
        }
    }
}

impl Scene for OutcomeScene {
    fn name(&self) -> &'static str {
        if self.won { "Victory" } else { "Defeat" }
    }

    fn on_enter(&mut self, world: &mut World) {
        sfx::play_music(world, Track::Outcome);
    }

    fn update(&mut self, world: &mut World) {
        let input = world.resources.expect::<Input>();
        if input.key_just_pressed(Key::Enter) || input.key_just_pressed(Key::Space) {
            let action = self.primary();
            send(world, action);
        } else if input.key_just_pressed(Key::Escape) {
            send(world, UiAction::BackToLevelSelect);
        }
    }

    fn draw(&mut self, world: &mut World) {
        self.draw_screen(world);
        // 菜单木牌（裁决九第 2 节），按钮本身在 `ui` 中为无边框 egui 按钮
        let slots: Vec<(Rect, bool)> = (0..self.buttons()).map(|i| (self.slot(i), true)).collect();
        draw_wood_buttons(world, &slots);
    }

    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {
        self.ui_buttons(world, ctx);
    }
}

impl OutcomeScene {
    fn draw_screen(&mut self, world: &mut World) {
        let title = t(
            world,
            if self.won {
                "outcome.won"
            } else {
                "outcome.lost"
            },
        );
        let hint = if self.unlocked.is_empty() {
            String::new()
        } else {
            let names: Vec<String> = self
                .unlocked
                .iter()
                .map(|id| t(world, &format!("defender.{id}.name")))
                .collect();
            format!("{} {}", t(world, "outcome.unlocked"), names.join(", "))
        };
        if self.ending {
            let line = t(world, "ending.line");
            if Self::draw_ending(world, &line) {
                return;
            }
            // 占位模式：结局一句写在提示行
            let hint = if hint.is_empty() {
                line
            } else {
                format!("{line}  {hint}")
            };
            render::overlay(world.resources.expect_mut::<DrawList>(), &title, &hint);
            return;
        }
        render::overlay(world.resources.expect_mut::<DrawList>(), &title, &hint);
    }

    fn ui_buttons(&mut self, world: &mut World, ctx: &egui::Context) {
        let slot = |i| self.slot(i);
        let mut i = 0;
        if self.won && self.has_next {
            if menu_button(
                world,
                ctx,
                "outcome_next",
                slot(i),
                &t(world, "outcome.next"),
                true,
            ) {
                send(world, UiAction::NextLevel);
            }
            i += 1;
        }
        if menu_button(
            world,
            ctx,
            "outcome_retry",
            slot(i),
            &t(world, "outcome.retry"),
            true,
        ) {
            send(world, UiAction::Restart);
        }
        if menu_button(
            world,
            ctx,
            "outcome_back",
            slot(i + 1),
            &t(world, "outcome.back"),
            true,
        ) {
            send(world, UiAction::BackToLevelSelect);
        }
    }
}
