//! 场景（SPEC 3.4）。

mod battle;
mod title;

pub use battle::BattleScene;
pub use title::TitleScene;

use trellia::egui;
use trellia::prelude::*;

/// 把逻辑矩形映射为 egui 坐标（点），使 egui 控件与视口 letterbox 对齐（ADR-003）。
#[must_use]
pub fn ui_rect(world: &World, ctx: &egui::Context, r: Rect) -> egui::Rect {
    let ppp = ctx.pixels_per_point();
    let map = |p: Vec2| {
        let px = world
            .resources
            .get::<Viewport>()
            .map_or(p, |v| v.logical_to_px(p))
            / ppp;
        egui::pos2(px.x, px.y)
    };
    egui::Rect::from_min_max(map(r.min), map(r.max))
}

/// 视口缩放系数（逻辑像素 → egui 点），用于字号。
#[must_use]
pub fn ui_scale(world: &World, ctx: &egui::Context) -> f32 {
    world.resources.get::<Viewport>().map_or(1.0, |v| v.scale) / ctx.pixels_per_point()
}

/// 在逻辑矩形处放置一个 egui 按钮，返回是否被点击。
pub fn button_at(world: &World, ctx: &egui::Context, id: &str, r: Rect, text: &str) -> bool {
    let rect = ui_rect(world, ctx, r);
    let size = 20.0 * ui_scale(world, ctx);
    let mut clicked = false;
    egui::Area::new(egui::Id::new(id))
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            let button =
                egui::Button::new(egui::RichText::new(text).size(size)).min_size(rect.size());
            clicked = ui.add(button).clicked();
        });
    clicked
}

/// 取走本帧全部场景动作（对局动作已由 `buffer_ui_actions` 移入收件箱，ADR-003）。
pub fn take_scene_actions(world: &mut World) -> Vec<rootguard_core::events::UiAction> {
    world
        .events
        .get_mut::<rootguard_core::events::UiAction>()
        .drain()
        .filter(|a| !a.is_battle_action())
        .collect()
}
