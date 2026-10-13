//! 暂停覆盖层（SPEC 3.4 / 11.3）：push；下方对局停止更新但可见。

use rootguard_core::events::UiAction;
use trellia::egui;
use trellia::prelude::*;

use crate::i18n::t;
use crate::scenes::{SettingsScene, button_at, menu_slot, send};
use crate::{LOGICAL_SIZE, render};

/// 暂停场景。
pub struct PauseScene;

impl Scene for PauseScene {
    fn name(&self) -> &'static str {
        "Paused"
    }

    fn update(&mut self, world: &mut World) {
        let input = world.resources.expect::<Input>();
        if input.key_just_pressed(Key::Escape) || input.key_just_pressed(Key::Space) {
            send(world, UiAction::PauseToggle);
        }
    }

    fn draw(&mut self, world: &mut World) {
        let title = t(world, "pause.title");
        render::overlay(world.resources.expect_mut::<DrawList>(), &title, "");
    }

    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {
        let slot = |i| menu_slot(LOGICAL_SIZE.x / 2.0, 360.0, 66.0, i);
        if button_at(
            world,
            ctx,
            "pause_resume",
            slot(0),
            &t(world, "pause.resume"),
            true,
        ) {
            send(world, UiAction::PauseToggle);
        }
        if button_at(
            world,
            ctx,
            "pause_restart",
            slot(1),
            &t(world, "pause.restart"),
            true,
        ) {
            send(world, UiAction::Restart);
        }
        if button_at(
            world,
            ctx,
            "pause_settings",
            slot(2),
            &t(world, "pause.settings"),
            true,
        ) {
            world
                .resources
                .expect_mut::<SceneCommands>()
                .push(SettingsScene);
        }
        if button_at(
            world,
            ctx,
            "pause_title",
            slot(3),
            &t(world, "pause.to_title"),
            true,
        ) {
            send(world, UiAction::BackToTitle);
        }
    }
}
