//! 选关场景（SPEC 3.4 / 11.3）：关卡按钮，未解锁置灰，显示已通关标记。

use rootguard_core::events::UiAction;
use rootguard_core::registry::LevelRegistry;
use rootguard_core::resources::Progress;
use trellia::egui;
use trellia::prelude::*;

use crate::flow::level_unlocked;
use crate::i18n::t;
use crate::scenes::{button_at, draw_menu, send};
use crate::sfx::{self, Track};

/// 每行关卡数。
const PER_ROW: usize = 5;

/// 选关场景。
pub struct LevelSelectScene;

impl Scene for LevelSelectScene {
    fn name(&self) -> &'static str {
        "LevelSelect"
    }

    fn on_enter(&mut self, world: &mut World) {
        sfx::play_music(world, Track::Title);
    }

    fn update(&mut self, world: &mut World) {
        if world
            .resources
            .expect::<Input>()
            .key_just_pressed(Key::Escape)
        {
            send(world, UiAction::BackToTitle);
        }
    }

    fn draw(&mut self, world: &mut World) {
        let title = t(world, "level_select.title");
        draw_menu(world, &title, None);
    }

    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {
        let levels: Vec<_> = world
            .resources
            .get::<LevelRegistry>()
            .map(|l| l.iter().map(|d| d.id).collect())
            .unwrap_or_default();
        let cleared_label = t(world, "level_select.cleared");
        let locked_label = t(world, "level_select.locked");
        for (i, id) in levels.into_iter().enumerate() {
            let col = (i % PER_ROW) as f32;
            let row = (i / PER_ROW) as f32;
            let rect = Rect::from_min_size(
                Vec2::new(190.0 + 185.0 * col, 220.0 + 130.0 * row),
                Vec2::new(160.0, 100.0),
            );
            let unlocked = level_unlocked(world, id);
            let cleared = world
                .resources
                .get::<Progress>()
                .is_some_and(|p| p.cleared.contains(&id));
            let label = if cleared {
                format!("{id}\n{cleared_label}")
            } else if unlocked {
                id.to_string()
            } else {
                format!("{id}\n{locked_label}")
            };
            if button_at(world, ctx, &format!("level_{id}"), rect, &label, unlocked) {
                send(world, UiAction::StartLevel(id));
            }
        }
        let back = Rect::from_min_size(Vec2::new(40.0, 630.0), Vec2::new(180.0, 54.0));
        if button_at(
            world,
            ctx,
            "level_back",
            back,
            &t(world, "level_select.back"),
            true,
        ) {
            send(world, UiAction::BackToTitle);
        }
    }
}
