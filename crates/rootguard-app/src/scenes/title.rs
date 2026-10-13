//! 标题场景（SPEC 3.4 / 11.3），栈底。

use trellia::egui;
use trellia::prelude::*;

use crate::LOGICAL_SIZE;
use crate::scenes::{BattleScene, button_at};

/// 标题场景。G2 只提供“开始”（进入冒险第一关）与“退出”；完整菜单见 G3。
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
            Vec2::new(LOGICAL_SIZE.x / 2.0, 200.0),
            crate::TITLE,
            96.0,
            Color::WHITE,
            TextAlign::Center,
        );
    }

    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {
        let at = |y: f32| {
            Rect::from_center_size(Vec2::new(LOGICAL_SIZE.x / 2.0, y), Vec2::new(240.0, 56.0))
        };
        if button_at(world, ctx, "title_start", at(400.0), "Start") {
            let first = world
                .resources
                .get::<rootguard_core::registry::LevelRegistry>()
                .and_then(|l| l.iter().next())
                .map(|l| l.id);
            if let Some(level) = first {
                world
                    .resources
                    .expect_mut::<SceneCommands>()
                    .replace(BattleScene::new(level, Vec::new(), None));
            }
        }
        if button_at(world, ctx, "title_quit", at(480.0), "Quit") {
            world.resources.expect_mut::<AppExit>().requested = true;
        }
    }
}
