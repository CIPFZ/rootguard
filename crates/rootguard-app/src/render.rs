//! 占位渲染（SPEC 12.2）：背景、防御者、敌人、弹丸、Dew、悬停预览与调试形状。
//!
//! 只读模拟状态，不修改任何模拟组件或资源（AGENT 4.4）。

use std::collections::HashMap;

use rootguard_core::Grid;
use rootguard_core::components::{
    Armor, Cell, DefenderRef, DewPickup, EnemyRef, Health, OverlayOf, Placeholder, ProjectileBody,
    StatusEffects, Walker,
};
use rootguard_core::data::ArmorKind;
use rootguard_core::events::EnemyDamaged;
use rootguard_core::resources::{BattleState, LaneState};
use rootguard_core::systems::check_placement;
use trellia::prelude::*;

use crate::LOGICAL_SIZE;
use crate::hud;

/// 受伤闪白持续的 tick 数（SPEC 12.2）。
const HURT_FLASH_TICKS: u64 = 6;

mod palette {
    pub const BACKDROP: u32 = 0x1F_3324;
    pub const LANE_ACTIVE: [u32; 2] = [0x5D_A04A, 0x52_9441];
    pub const LANE_INACTIVE: [u32; 2] = [0x2F_4A2B, 0x2A_4327];
    pub const GRID_LINE: u32 = 0x3E_6B32;
    pub const GREENHOUSE: u32 = 0x6E_8796;
    pub const GREENHOUSE_PANE: u32 = 0x9F_B8C4;
    pub const ENTRANCE: u32 = 0x24_2A22;
    pub const HOSE: u32 = 0x3F_7FBF;
    pub const HELM: u32 = 0x8D_5A2B;
    pub const SHIELD: u32 = 0x8C_9AA3;
    pub const DRUM: u32 = 0x6B_3E26;
    pub const LOW_HP: u32 = 0xE5_3935;
    pub const OVERLAY_RING: u32 = 0xD9_A441;
    pub const CHILL_TINT: u32 = 0x9F_D3FF;
    pub const DEW: u32 = 0xFF_D84D;
    pub const DEW_GLOW: u32 = 0xFF_F3B0;
    pub const PREVIEW_OK: u32 = 0x66_FF66;
    pub const PREVIEW_BAD: u32 = 0xFF_5555;
}

/// 呈现层状态：受伤闪白时刻（实体 → 最近受伤的 `fixed_tick`）。只做按键查找，不迭代（确定性无关）。
#[derive(Default)]
pub struct RenderState {
    hurt_at: HashMap<Entity, u64>,
}

/// `Update`：记录本帧可见的受伤事件，供闪白使用。
pub fn track_hurt(world: &mut World) {
    let tick = world.resources.expect::<Time>().fixed_tick;
    let hurt: Vec<Entity> = world
        .events
        .get::<EnemyDamaged>()
        .iter()
        .map(|d| d.entity)
        .collect();
    if let Some(state) = world.resources.get_mut::<RenderState>() {
        // 事件在两帧内可见，重复写入只会让闪白多持续一帧（纯视觉）
        for e in hurt {
            state.hurt_at.insert(e, tick);
        }
        state
            .hurt_at
            .retain(|_, at| tick.saturating_sub(*at) < HURT_FLASH_TICKS);
    }
}

fn color(hex: u32) -> Color {
    Color::from_hex(hex)
}

/// 行遮挡（SPEC 12.2）：E-3 落地前每行一个图层。
fn row_layer(row: u8) -> Layer {
    Layer(Layer::WORLD.0 + i16::from(row))
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a,
    )
}

/// 对局画面（SPEC 12.2）。
pub fn draw_battle(world: &mut World) {
    let Some(mut list) = world.resources.remove::<DrawList>() else {
        return;
    };
    let alpha = world.resources.expect::<Time>().alpha;
    let tick = world.resources.expect::<Time>().fixed_tick;
    draw_background(world, &mut list);
    draw_defenders(world, &mut list, alpha);
    draw_enemies(world, &mut list, alpha, tick);
    for (t, body, p, lane) in world
        .query::<(
            &Transform2D,
            &ProjectileBody,
            &Placeholder,
            &rootguard_core::components::Lane,
        )>()
        .iter()
    {
        let _ = body;
        list.circle(
            row_layer(lane.row),
            t.lerp(alpha),
            p.size.x / 2.0,
            color(p.color),
        );
    }
    // Dew 画在 HUD 之上：下落时会经过顶部 HUD 区域，仍需可见可点
    let dew_layer = Layer(Layer::OVERLAY.0 + 1);
    for (t, _) in world.query::<(&Transform2D, &DewPickup)>().iter() {
        let pos = t.lerp(alpha);
        list.circle(
            dew_layer,
            pos,
            22.0,
            color(palette::DEW_GLOW).with_alpha(0.5),
        );
        list.circle(dew_layer, pos, 16.0, color(palette::DEW));
    }
    draw_preview(world, &mut list);
    hud::draw(world, &mut list);
    world.resources.insert(list);
    #[cfg(feature = "devtools")]
    debug_shapes(world);
}

fn draw_background(world: &World, list: &mut DrawList) {
    let lanes = world
        .resources
        .get::<LaneState>()
        .copied()
        .unwrap_or_else(|| LaneState::new(&[]));
    list.rect(
        Layer::BACKGROUND,
        Rect::from_min_size(Vec2::ZERO, LOGICAL_SIZE),
        color(palette::BACKDROP),
    );
    for row in 0..Grid::ROWS {
        let shades = if lanes.is_active(row) {
            palette::LANE_ACTIVE
        } else {
            palette::LANE_INACTIVE
        };
        for col in 0..Grid::COLS {
            let rect = Rect::from_center_size(Grid::cell_center(row, col), Grid::CELL_SIZE);
            let shade = shades[usize::from((row + col) % 2)];
            list.rect(Layer::BACKGROUND, rect, color(shade));
        }
        if lanes.is_active(row) && lanes.hose_available[usize::from(row)] {
            let y = Grid::row_center_y(row);
            list.rect(
                Layer::BACKGROUND,
                Rect::from_center_size(Vec2::new(Grid::HOSE_X, y), Vec2::new(30.0, 24.0)),
                color(palette::HOSE),
            );
        }
    }
    let grid_rect = Rect::from_min_size(
        Grid::ORIGIN,
        Vec2::new(
            Grid::CELL_SIZE.x * f32::from(Grid::COLS),
            Grid::CELL_SIZE.y * f32::from(Grid::ROWS),
        ),
    );
    for col in 0..=Grid::COLS {
        let x = Grid::ORIGIN.x + Grid::CELL_SIZE.x * f32::from(col);
        list.rect(
            Layer::BACKGROUND,
            Rect::from_min_size(
                Vec2::new(x - 0.5, grid_rect.min.y),
                Vec2::new(1.0, grid_rect.size().y),
            ),
            color(palette::GRID_LINE),
        );
    }
    // 左侧温室、右侧入口
    let greenhouse = Rect::from_min_size(
        Vec2::new(0.0, grid_rect.min.y),
        Vec2::new(Grid::HOSE_X - 20.0, grid_rect.size().y),
    );
    list.rect(Layer::BACKGROUND, greenhouse, color(palette::GREENHOUSE));
    for row in 0..Grid::ROWS {
        let pane = Rect::from_center_size(
            Vec2::new(greenhouse.center().x, Grid::row_center_y(row)),
            Vec2::new(greenhouse.size().x - 40.0, Grid::CELL_SIZE.y - 30.0),
        );
        list.rect(
            Layer::BACKGROUND,
            pane,
            color(palette::GREENHOUSE_PANE).with_alpha(0.5),
        );
    }
    list.rect(
        Layer::BACKGROUND,
        Rect::from_min_size(
            Vec2::new(Grid::RIGHT_X, grid_rect.min.y),
            Vec2::new(LOGICAL_SIZE.x - Grid::RIGHT_X, grid_rect.size().y),
        ),
        color(palette::ENTRANCE),
    );
}

fn draw_defenders(world: &World, list: &mut DrawList, alpha: f32) {
    // 地面防御者
    for (t, p, h, cell, _) in world
        .query::<(&Transform2D, &Placeholder, &Health, &Cell, &DefenderRef)>()
        .without::<&OverlayOf>()
        .iter()
    {
        if !h.alive() {
            continue;
        }
        let layer = row_layer(cell.row);
        let pos = t.lerp(alpha);
        let body = Rect::from_center_size(pos, p.size);
        list.rect(layer, body, color(p.color));
        if h.hp * 2 < h.max {
            list.rect_stroke(layer, body, color(palette::LOW_HP), 3.0);
        }
        list.text_aligned(
            layer,
            pos - Vec2::new(0.0, 16.0),
            p.label.as_str(),
            28.0,
            Color::WHITE,
            TextAlign::Center,
        );
    }
    // 覆盖层：宿主外围描边环
    for (t, p, h, cell, _) in world
        .query::<(&Transform2D, &Placeholder, &Health, &Cell, &OverlayOf)>()
        .iter()
    {
        if !h.alive() {
            continue;
        }
        let ring = Rect::from_center_size(t.lerp(alpha), p.size + Vec2::splat(12.0));
        let c = if h.hp * 2 < h.max {
            color(palette::LOW_HP)
        } else {
            color(palette::OVERLAY_RING)
        };
        list.rect_stroke(row_layer(cell.row), ring, c, 6.0);
    }
}

fn draw_enemies(world: &World, list: &mut DrawList, alpha: f32, tick: u64) {
    let hurt = world.resources.get::<RenderState>();
    for (e, t, p, h, lane, status, armor, _, _) in world
        .query::<(
            Entity,
            &Transform2D,
            &Placeholder,
            &Health,
            &rootguard_core::components::Lane,
            &StatusEffects,
            Option<&Armor>,
            &EnemyRef,
            &Walker,
        )>()
        .iter()
    {
        if !h.alive() {
            continue;
        }
        let layer = row_layer(lane.row);
        let pos = t.lerp(alpha);
        let mut c = color(p.color);
        if status.chilled() {
            c = mix(c, color(palette::CHILL_TINT), 0.5);
        }
        let flashing = hurt
            .and_then(|s| s.hurt_at.get(&e))
            .is_some_and(|&at| tick.saturating_sub(at) < HURT_FLASH_TICKS);
        if flashing {
            c = mix(c, Color::WHITE, 0.7);
        }
        let body = Rect::from_center_size(pos, p.size);
        list.rect(layer, body, c);
        if let Some(a) = armor.filter(|a| a.hp > 0) {
            match a.kind {
                ArmorKind::Helm => list.rect(
                    layer,
                    Rect::from_center_size(
                        Vec2::new(pos.x, body.min.y - 6.0),
                        Vec2::new(p.size.x + 6.0, 16.0),
                    ),
                    color(palette::HELM),
                ),
                ArmorKind::Shield => list.rect(
                    layer,
                    Rect::from_center_size(
                        Vec2::new(body.min.x - 6.0, pos.y),
                        Vec2::new(12.0, p.size.y + 10.0),
                    ),
                    color(palette::SHIELD),
                ),
                ArmorKind::Drum => list.circle(
                    layer,
                    pos + Vec2::new(0.0, 18.0),
                    20.0,
                    color(palette::DRUM),
                ),
            }
        }
        list.text_aligned(
            layer,
            pos - Vec2::new(0.0, 16.0),
            p.label.as_str(),
            26.0,
            Color::WHITE,
            TextAlign::Center,
        );
    }
}

/// 鼠标悬停格子且已选卡：半透明预览，可放置为绿、不可为红（SPEC 11.2）。
fn draw_preview(world: &World, list: &mut DrawList) {
    let Some(state) = world.resources.get::<BattleState>() else {
        return;
    };
    let Some(slot) = state.selected_card else {
        return;
    };
    let Some((row, col)) = world
        .resources
        .expect::<Input>()
        .mouse_pos
        .and_then(Grid::cell_at)
    else {
        return;
    };
    let ok = check_placement(world, slot, Cell { row, col }).is_some();
    let c = if ok {
        palette::PREVIEW_OK
    } else {
        palette::PREVIEW_BAD
    };
    list.rect(
        Layer::FX,
        Rect::from_center_size(Grid::cell_center(row, col), Grid::CELL_SIZE),
        color(c).with_alpha(0.35),
    );
}

/// 半透明覆盖层 + 标题 + 提示。
pub fn overlay(list: &mut DrawList, title: &str, hint: &str) {
    list.rect(
        Layer::OVERLAY,
        Rect::from_min_size(Vec2::ZERO, LOGICAL_SIZE),
        Color::BLACK.with_alpha(0.55),
    );
    list.text_aligned(
        Layer::OVERLAY,
        Vec2::new(LOGICAL_SIZE.x / 2.0, 250.0),
        title,
        72.0,
        Color::WHITE,
        TextAlign::Center,
    );
    list.text_aligned(
        Layer::OVERLAY,
        Vec2::new(LOGICAL_SIZE.x / 2.0, 345.0),
        hint,
        24.0,
        color(0xD0_D8D0),
        TextAlign::Center,
    );
}

/// DebugDraw 开启时：格子边界与敌人碰撞区间（SPEC 11.5）。
#[cfg(feature = "devtools")]
fn debug_shapes(world: &mut World) {
    let enemies: Vec<(f32, u8, i32)> = world
        .query::<(
            &Transform2D,
            &rootguard_core::components::Lane,
            &Health,
            &EnemyRef,
        )>()
        .iter()
        .map(|(t, l, h, _)| (t.pos.x, l.row, h.hp))
        .collect();
    let Some(debug) = world.resources.get_mut::<DebugDraw>() else {
        return;
    };
    if !debug.enabled() {
        return;
    }
    for row in 0..Grid::ROWS {
        for col in 0..Grid::COLS {
            debug.rect(
                Rect::from_center_size(Grid::cell_center(row, col), Grid::CELL_SIZE),
                color(0xEC_F0F1),
            );
        }
    }
    for (x, row, hp) in enemies {
        let top = Grid::ORIGIN.y + Grid::CELL_SIZE.y * f32::from(row);
        let rect = Rect::from_min_size(
            Vec2::new(x - Grid::ENEMY_HALF_WIDTH, top),
            Vec2::new(2.0 * Grid::ENEMY_HALF_WIDTH, Grid::CELL_SIZE.y),
        );
        debug.rect(rect, color(0xE7_4C3C));
        debug.text(rect.min - Vec2::new(0.0, 16.0), format!("x {x:.0} hp {hp}"));
    }
}
