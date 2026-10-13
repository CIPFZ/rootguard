//! 对局 HUD（SPEC 11.2）：Dew 计数、卡槽、波次进度、横幅。
//!
//! 视觉部分绘制在 `DrawList`（随视口缩放，headless 可检查）；卡槽点击由 `BattleScene::update`
//! 用 [`card_at`] 翻译为 `UiAction::SelectCard`（SPEC 11.1）；铲子与暂停按钮为 egui（ADR-003）。

use rootguard_core::registry::DefenderRegistry;
use rootguard_core::resources::{BattleState, CardTray, Phase, WaveState};
use trellia::prelude::*;

use crate::LOGICAL_SIZE;
use crate::i18n::{t, tf};

/// Dew 计数框。
pub const DEW_BOX: Rect = Rect {
    min: Vec2::new(10.0, 8.0),
    max: Vec2::new(120.0, 72.0),
};
/// 第一张卡的左上角。
const CARD_ORIGIN: Vec2 = Vec2::new(132.0, 8.0);
/// 卡牌尺寸。
const CARD_SIZE: Vec2 = Vec2::new(76.0, 64.0);
/// 卡牌间距。
const CARD_GAP: f32 = 6.0;
/// 波次进度条。
const WAVE_BAR: Rect = Rect {
    min: Vec2::new(820.0, 30.0),
    max: Vec2::new(1030.0, 50.0),
};
/// 铲子按钮。
pub const SHOVEL_BUTTON: Rect = Rect {
    min: Vec2::new(1050.0, 12.0),
    max: Vec2::new(1150.0, 68.0),
};
/// 暂停按钮。
pub const PAUSE_BUTTON: Rect = Rect {
    min: Vec2::new(1162.0, 12.0),
    max: Vec2::new(1270.0, 68.0),
};
/// 底部横幅中心。
const BANNER_POS: Vec2 = Vec2::new(640.0, 682.0);

/// 第 `i` 张卡的矩形。
#[must_use]
pub fn card_rect(i: usize) -> Rect {
    let x = CARD_ORIGIN.x + (CARD_SIZE.x + CARD_GAP) * i as f32;
    Rect::from_min_size(Vec2::new(x, CARD_ORIGIN.y), CARD_SIZE)
}

/// 逻辑坐标处的卡槽下标（只计实际存在的卡）。
#[must_use]
pub fn card_at(world: &World, pos: Vec2) -> Option<usize> {
    let n = world.resources.get::<CardTray>()?.cards.len();
    (0..n).find(|&i| card_rect(i).contains(pos))
}

/// 横幅文字（SPEC 11.2："准备"、"大波来袭"、"最后一波"）。
#[must_use]
pub fn banner(world: &World) -> Option<String> {
    let state = world.resources.get::<BattleState>()?;
    let waves = world.resources.get::<WaveState>()?;
    if state.phase == Phase::Intro {
        return Some(t(world, "banner.ready"));
    }
    if state.phase != Phase::Playing || waves.banner.is_none() {
        return None;
    }
    let key = if waves.next_wave + 1 == waves.total_waves {
        "banner.final_wave"
    } else {
        "banner.big_wave"
    };
    Some(t(world, key))
}

/// 绘制 HUD。
pub fn draw(world: &World, list: &mut DrawList) {
    let (Some(state), Some(tray), Some(defs)) = (
        world.resources.get::<BattleState>(),
        world.resources.get::<CardTray>(),
        world.resources.get::<DefenderRegistry>(),
    ) else {
        return;
    };
    let hud = Layer::OVERLAY;
    list.rect(
        hud,
        Rect::from_min_size(Vec2::ZERO, Vec2::new(LOGICAL_SIZE.x, 80.0)),
        Color::from_hex(0x2B_1F14).with_alpha(0.85),
    );
    // Dew 计数
    list.rect(hud, DEW_BOX, Color::from_hex(0x4A_3A28));
    list.circle(
        hud,
        DEW_BOX.min + Vec2::new(26.0, 32.0),
        16.0,
        Color::from_hex(0xFF_D84D),
    );
    list.text_aligned(
        hud,
        DEW_BOX.min + Vec2::new(76.0, 18.0),
        state.dew.to_string(),
        28.0,
        Color::WHITE,
        TextAlign::Center,
    );
    // 卡槽
    for (i, card) in tray.cards.iter().enumerate() {
        let rect = card_rect(i);
        list.rect(hud, rect, Color::from_hex(0x6B_5A40));
        let Some(def) = defs.get(card.defender) else {
            continue;
        };
        let swatch = Rect::from_min_size(rect.min + Vec2::new(8.0, 6.0), Vec2::new(60.0, 34.0));
        list.rect(hud, swatch, Color::from_hex(def.placeholder.color));
        list.text_aligned(
            hud,
            swatch.center() - Vec2::new(0.0, 11.0),
            def.placeholder.label.as_str(),
            20.0,
            Color::WHITE,
            TextAlign::Center,
        );
        list.text_aligned(
            hud,
            Vec2::new(rect.center().x, rect.max.y - 22.0),
            def.cost.to_string(),
            18.0,
            Color::WHITE,
            TextAlign::Center,
        );
        // 冷却遮罩：自上而下消退
        let left = if card.cooldown.duration == 0 {
            0.0
        } else {
            card.cooldown.remaining as f32 / card.cooldown.duration as f32
        };
        if left > 0.0 {
            let h = CARD_SIZE.y * left;
            list.rect(
                hud,
                Rect::from_min_size(
                    Vec2::new(rect.min.x, rect.max.y - h),
                    Vec2::new(CARD_SIZE.x, h),
                ),
                Color::BLACK.with_alpha(0.55),
            );
        } else if !card.ready {
            list.rect(hud, rect, Color::from_hex(0x30_3030).with_alpha(0.6));
        }
        if state.selected_card == Some(i) {
            list.rect_stroke(hud, rect, Color::from_hex(0xFF_EB3B), 3.0);
        }
        list.text(
            hud,
            rect.min + Vec2::new(3.0, 1.0),
            (i + 1).to_string(),
            12.0,
            Color::from_hex(0xE0_E0E0),
        );
    }
    // 波次进度条（已生成波 / 总波，flag 波有标记）
    if let Some(waves) = world.resources.get::<WaveState>() {
        list.rect(hud, WAVE_BAR, Color::from_hex(0x3A_2E22));
        let total = waves.total_waves.max(1) as f32;
        let done = waves.next_wave as f32 / total;
        list.rect(
            hud,
            Rect::from_min_size(
                WAVE_BAR.min,
                Vec2::new(WAVE_BAR.size().x * done, WAVE_BAR.size().y),
            ),
            Color::from_hex(0x8B_C34A),
        );
        if let Some(level) = world
            .resources
            .get::<rootguard_core::registry::LevelRegistry>()
            .and_then(|l| l.get(state.level))
        {
            for (k, w) in level.waves.iter().enumerate() {
                if w.flag {
                    let x = WAVE_BAR.min.x + WAVE_BAR.size().x * (k + 1) as f32 / total;
                    list.rect(
                        hud,
                        Rect::from_center_size(
                            Vec2::new(x - 4.0, WAVE_BAR.min.y - 4.0),
                            Vec2::new(6.0, 14.0),
                        ),
                        Color::from_hex(0xE5_3935),
                    );
                }
            }
        }
        list.text(
            hud,
            WAVE_BAR.min - Vec2::new(0.0, 22.0),
            format!(
                "{} {}/{}",
                tf(world, "battle.level", &[("id", state.level.as_str())]),
                waves.next_wave,
                waves.total_waves
            ),
            16.0,
            Color::WHITE,
        );
    }
    if let Some(text) = banner(world) {
        list.text_aligned(
            hud,
            BANNER_POS,
            text,
            30.0,
            Color::from_hex(0xFF_F59D),
            TextAlign::Center,
        );
    }
}
