//! 场景（SPEC.md 3.4）。

use trellia::egui;
use trellia::prelude::*;

use crate::LOGICAL_SIZE;

/// 标题场景（SPEC.md 3.4 / 11.3），栈底。G0 只显示标题文字与退出按钮。
pub struct TitleScene;

impl Scene for TitleScene {
    fn name(&self) -> &'static str {
        "Title"
    }

    fn draw(&mut self, world: &mut World) {
        let list = world.resources.expect_mut::<DrawList>();
        list.rect(
            Layer::BACKGROUND,
            Rect::from_min_size(Vec2::ZERO, LOGICAL_SIZE),
            Color::from_hex(0x2E_4A3A),
        );
        list.text_aligned(
            Layer::OVERLAY,
            Vec2::new(LOGICAL_SIZE.x / 2.0, 220.0),
            crate::TITLE,
            96.0,
            Color::WHITE,
            TextAlign::Center,
        );
    }

    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {
        let mut quit = false;
        egui::Area::new(egui::Id::new("rootguard_title_menu"))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 120.0))
            .show(ctx, |ui| {
                quit = ui
                    .add(egui::Button::new(egui::RichText::new("Quit").size(28.0)))
                    .clicked();
            });
        if quit {
            world.resources.expect_mut::<AppExit>().requested = true;
        }
    }
}
