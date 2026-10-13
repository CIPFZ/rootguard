//! 设置场景（SPEC 3.4 / 11.3，push）：音效 / 音乐滑杆、语言、皮肤 / 占位；实时生效并写入存档。

use rootguard_core::events::{Lang, SkinMode, UiAction, VolumeBus};
use rootguard_core::resources::{Progress, Settings};
use trellia::egui;
use trellia::prelude::*;

use crate::LOGICAL_SIZE;
use crate::i18n::t;
use crate::scenes::{button_at, draw_menu, send, ui_rect, ui_scale};

/// 设置场景（push，覆盖在标题或暂停之上）。
pub struct SettingsScene;

fn close(world: &mut World) {
    world.resources.expect_mut::<SceneCommands>().pop();
}

impl Scene for SettingsScene {
    fn name(&self) -> &'static str {
        "Settings"
    }

    fn update(&mut self, world: &mut World) {
        if world
            .resources
            .expect::<Input>()
            .key_just_pressed(Key::Escape)
        {
            close(world);
        }
    }

    fn draw(&mut self, world: &mut World) {
        let title = t(world, "settings.title");
        let rows = [
            ("settings.sfx", 300.0),
            ("settings.music", 370.0),
            ("settings.language", 440.0),
            ("settings.skin", 510.0),
        ];
        let labels: Vec<(String, f32)> = rows.iter().map(|(k, y)| (t(world, k), *y)).collect();
        draw_menu(world, &title, None);
        let list = world.resources.expect_mut::<DrawList>();
        for (label, y) in labels {
            list.text(
                Layer::OVERLAY,
                Vec2::new(380.0, y - 14.0),
                label,
                24.0,
                Color::WHITE,
            );
        }
    }

    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {
        let settings: Settings = world
            .resources
            .get::<Progress>()
            .map(|p| p.settings)
            .unwrap_or_default();
        let scale = ui_scale(world, ctx);
        for (i, (bus, value)) in [
            (VolumeBus::Sfx, settings.sfx),
            (VolumeBus::Music, settings.music),
        ]
        .into_iter()
        .enumerate()
        {
            let rect = ui_rect(
                world,
                ctx,
                Rect::from_min_size(
                    Vec2::new(600.0, 285.0 + 70.0 * i as f32),
                    Vec2::new(300.0, 30.0),
                ),
            );
            let mut v = value;
            egui::Area::new(egui::Id::new(("settings_volume", i)))
                .fixed_pos(rect.min)
                .show(ctx, |ui| {
                    ui.spacing_mut().slider_width = rect.width();
                    ui.style_mut()
                        .text_styles
                        .iter_mut()
                        .for_each(|(_, f)| f.size = 16.0 * scale);
                    ui.add(egui::Slider::new(&mut v, 0.0..=1.0).show_value(false));
                });
            if (v - value).abs() > f32::EPSILON {
                send(world, UiAction::SetVolume { bus, volume: v });
            }
        }
        let opt = |x: f32, y: f32| Rect::from_min_size(Vec2::new(x, y), Vec2::new(150.0, 44.0));
        for (j, (lang, key)) in [(Lang::Zh, "lang.zh"), (Lang::En, "lang.en")]
            .into_iter()
            .enumerate()
        {
            let mut label = t(world, key);
            if settings.lang == lang {
                label = format!("[{label}]");
            }
            if button_at(
                world,
                ctx,
                key,
                opt(600.0 + 160.0 * j as f32, 418.0),
                &label,
                true,
            ) {
                send(world, UiAction::SetLanguage(lang));
            }
        }
        for (j, (skin, key)) in [
            (SkinMode::Placeholder, "settings.skin.placeholder"),
            (SkinMode::Skinned, "settings.skin.skinned"),
        ]
        .into_iter()
        .enumerate()
        {
            let mut label = t(world, key);
            if settings.skin == skin {
                label = format!("[{label}]");
            }
            if button_at(
                world,
                ctx,
                key,
                opt(600.0 + 160.0 * j as f32, 488.0),
                &label,
                true,
            ) {
                send(world, UiAction::SetSkin(skin));
            }
        }
        let back = Rect::from_center_size(
            Vec2::new(LOGICAL_SIZE.x / 2.0, 610.0),
            Vec2::new(220.0, 54.0),
        );
        if button_at(
            world,
            ctx,
            "settings_back",
            back,
            &t(world, "settings.back"),
            true,
        ) {
            close(world);
        }
    }

    /// 遮住下方场景（含其 egui 按钮），避免误点。
    fn blocks_draw_below(&self) -> bool {
        true
    }
}
