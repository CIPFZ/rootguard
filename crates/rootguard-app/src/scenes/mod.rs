//! 场景（SPEC 3.4）：Title / LevelSelect / SeedSelect / Battle / Pause / Outcome / Settings。
//!
//! 场景只把按钮与按键翻译为 `UiAction`（SPEC 11.1），由 `flow::ui_router_system` 执行切换。

mod battle;
mod level_select;
mod outcome;
mod pause;
mod seed_select;
mod settings;
mod title;

pub use battle::BattleScene;
pub use level_select::LevelSelectScene;
pub use outcome::{ENDING_BACK, OutcomeScene};
pub use pause::PauseScene;
pub use seed_select::SeedSelectScene;
pub use settings::SettingsScene;
pub use title::{TITLE_BG, TitleScene};

use rootguard_core::events::UiAction;
use trellia::egui;
use trellia::prelude::*;

use crate::LOGICAL_SIZE;

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

/// 在逻辑矩形处放置一个 egui 按钮，返回是否被点击。`enabled == false` 时置灰。
pub fn button_at(
    world: &World,
    ctx: &egui::Context,
    id: &str,
    r: Rect,
    text: &str,
    enabled: bool,
) -> bool {
    button_styled(world, ctx, id, r, text, enabled, true)
}

/// 同 [`button_at`]；`frame` 为 false 时不画按钮底（皮肤 HUD 的木牌由 `DrawList` 画在下面，ART-SPEC 8.3）。
pub fn button_styled(
    world: &World,
    ctx: &egui::Context,
    id: &str,
    r: Rect,
    text: &str,
    enabled: bool,
    frame: bool,
) -> bool {
    let rect = ui_rect(world, ctx, r);
    let size = 20.0 * ui_scale(world, ctx);
    let mut clicked = false;
    egui::Area::new(egui::Id::new(id))
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            let mut label = egui::RichText::new(text).size(size);
            if !frame {
                label = label.color(egui::Color32::WHITE);
            }
            let button = egui::Button::new(label).min_size(rect.size()).frame(frame);
            clicked = ui.add_enabled(enabled, button).clicked();
        });
    clicked
}

/// 菜单按钮列：以 `center_x` 为中心、自 `top` 起每行 `step` 的第 `i` 个按钮矩形。
#[must_use]
pub fn menu_slot(center_x: f32, top: f32, step: f32, i: usize) -> Rect {
    Rect::from_center_size(
        Vec2::new(center_x, top + step * i as f32),
        Vec2::new(280.0, 54.0),
    )
}

/// 发送一个 `UiAction`。
pub fn send(world: &mut World, action: UiAction) {
    world.events.send(action);
}

/// 取走本帧全部场景动作（对局动作已由 `buffer_ui_actions` 移入收件箱，ADR-003）。
pub fn take_scene_actions(world: &mut World) -> Vec<UiAction> {
    world
        .events
        .get_mut::<UiAction>()
        .drain()
        .filter(|a| !a.is_battle_action())
        .collect()
}

/// 菜单背景与标题。
pub fn draw_menu(world: &mut World, title: &str, subtitle: Option<&str>) {
    let list = world.resources.expect_mut::<DrawList>();
    list.rect(
        Layer::BACKGROUND,
        Rect::from_min_size(Vec2::ZERO, LOGICAL_SIZE),
        Color::from_hex(0x2E_4A3A),
    );
    for i in 0..6 {
        let y = 520.0 + 40.0 * i as f32;
        list.rect(
            Layer::BACKGROUND,
            Rect::from_min_size(Vec2::new(0.0, y), Vec2::new(LOGICAL_SIZE.x, 40.0)),
            Color::from_hex(if i % 2 == 0 { 0x3A_5C45 } else { 0x35_5540 }),
        );
    }
    list.text_aligned(
        Layer::OVERLAY,
        Vec2::new(LOGICAL_SIZE.x / 2.0, 70.0),
        title,
        64.0,
        Color::WHITE,
        TextAlign::Center,
    );
    if let Some(sub) = subtitle {
        list.text_aligned(
            Layer::OVERLAY,
            Vec2::new(LOGICAL_SIZE.x / 2.0, 150.0),
            sub,
            24.0,
            Color::from_hex(0xD0_E0C8),
            TextAlign::Center,
        );
    }
}
