//! 标题场景（SPEC 3.4 / 11.3），栈底：继续（有存档时）/ 新游戏 / 设置 / 退出。

use rootguard_core::events::UiAction;
use rootguard_core::resources::Progress;
use trellia::egui;
use trellia::prelude::*;

use crate::i18n::t;
use crate::save::{self, SaveSlot};
use crate::scenes::{SettingsScene, button_at, draw_menu, menu_slot, send};
use crate::sfx::{self, Track};
use crate::{LOGICAL_SIZE, TITLE};

/// 标题场景。
pub struct TitleScene;

fn has_save(world: &World) -> bool {
    world.resources.get::<SaveSlot>().is_some_and(|s| s.existed)
        || world
            .resources
            .get::<Progress>()
            .is_some_and(|p| !p.cleared.is_empty())
}

impl Scene for TitleScene {
    fn name(&self) -> &'static str {
        "Title"
    }

    fn on_enter(&mut self, world: &mut World) {
        sfx::play_music(world, Track::Title);
    }

    fn draw(&mut self, world: &mut World) {
        let sub = t(world, "title.subtitle");
        draw_menu(world, TITLE, Some(&sub));
    }

    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {
        let slot = |i| menu_slot(LOGICAL_SIZE.x / 2.0, 280.0, 70.0, i);
        let can_continue = has_save(world);
        if button_at(
            world,
            ctx,
            "title_continue",
            slot(0),
            &t(world, "title.continue"),
            can_continue,
        ) {
            send(world, UiAction::BackToLevelSelect);
        }
        if button_at(
            world,
            ctx,
            "title_new",
            slot(1),
            &t(world, "title.new_game"),
            true,
        ) {
            // 新游戏：重置进度（保留设置）后进入选关
            let rules = world
                .resources
                .get::<rootguard_core::data::Rules>()
                .cloned();
            let fresh = save::default_progress(rules.as_ref());
            if let Some(p) = world.resources.get_mut::<Progress>() {
                let settings = p.settings;
                *p = fresh;
                p.settings = settings;
            }
            save::save(world);
            send(world, UiAction::BackToLevelSelect);
        }
        if button_at(
            world,
            ctx,
            "title_settings",
            slot(2),
            &t(world, "title.settings"),
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
            "title_quit",
            slot(3),
            &t(world, "title.quit"),
            true,
        ) {
            world.resources.expect_mut::<AppExit>().requested = true;
        }
    }
}
