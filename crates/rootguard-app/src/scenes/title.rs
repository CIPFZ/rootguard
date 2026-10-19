//! 标题场景（SPEC 3.4 / 11.3），栈底：继续（有存档时）/ 新游戏 / 设置 / 退出。

use rootguard_core::events::UiAction;
use rootguard_core::resources::Progress;
use trellia::egui;
use trellia::prelude::*;

use crate::i18n::t;
use crate::save::{self, SaveSlot};
use crate::scenes::{SettingsScene, draw_menu, draw_wood_buttons, menu_button, menu_slot, send};
use crate::sfx::{self, Track};
use crate::{LOGICAL_SIZE, TITLE};

/// 标题背景片段（`art/story/title_bg.svg`）。
pub const TITLE_BG: &str = "story_title_bg";

/// 标题场景。
pub struct TitleScene;

/// 标题菜单：继续 / 新游戏 / 设置 / 退出。
const TITLE_BUTTONS: usize = 4;

fn title_slot(i: usize) -> Rect {
    menu_slot(LOGICAL_SIZE.x / 2.0, 280.0, 70.0, i)
}

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
        // ART-SPEC 8.4：花园皮肤的标题背景（沉睡的根母远景；标题写在上方的吊挂木牌上）
        let Some(mut list) = world.resources.remove::<DrawList>() else {
            return;
        };
        if let Some(sk) = crate::skin::Painter::active(world) {
            let full = Rect::from_min_size(Vec2::ZERO, LOGICAL_SIZE);
            sk.draw_in(
                &mut list,
                Layer::BACKGROUND,
                TITLE_BG,
                full,
                1.0,
                Color::WHITE,
            );
        }
        world.resources.insert(list);
        // 菜单木牌（裁决九第 2 节），按钮本身在 `ui` 中为无边框 egui 按钮
        let can_continue = has_save(world);
        let slots: Vec<(Rect, bool)> = (0..TITLE_BUTTONS)
            .map(|i| (title_slot(i), i != 0 || can_continue))
            .collect();
        draw_wood_buttons(world, &slots);
    }

    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {
        let slot = title_slot;
        let can_continue = has_save(world);
        if menu_button(
            world,
            ctx,
            "title_continue",
            slot(0),
            &t(world, "title.continue"),
            can_continue,
        ) {
            send(world, UiAction::BackToLevelSelect);
        }
        if menu_button(
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
        if menu_button(
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
        if menu_button(
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
