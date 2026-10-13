//! 结算覆盖层（SPEC 3.4 / 11.3）：胜利显示解锁的新卡；下一关 / 重试 / 返回。

use rootguard_core::DefenderId;
use rootguard_core::events::UiAction;
use trellia::egui;
use trellia::prelude::*;

use crate::i18n::t;
use crate::scenes::{button_at, menu_slot, send};
use crate::sfx::{self, Track};
use crate::{LOGICAL_SIZE, render};

/// 结算场景。
pub struct OutcomeScene {
    won: bool,
    unlocked: Vec<DefenderId>,
    has_next: bool,
}

impl OutcomeScene {
    /// 创建；`unlocked` 为本次新解锁的卡牌。
    #[must_use]
    pub fn new(won: bool, unlocked: Vec<DefenderId>, has_next: bool) -> Self {
        Self {
            won,
            unlocked,
            has_next,
        }
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
        render::overlay(world.resources.expect_mut::<DrawList>(), &title, &hint);
    }

    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {
        let slot = |i| menu_slot(LOGICAL_SIZE.x / 2.0, 440.0, 66.0, i);
        let mut i = 0;
        if self.won && self.has_next {
            if button_at(
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
        if button_at(
            world,
            ctx,
            "outcome_retry",
            slot(i),
            &t(world, "outcome.retry"),
            true,
        ) {
            send(world, UiAction::Restart);
        }
        if button_at(
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
