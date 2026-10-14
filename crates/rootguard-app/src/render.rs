//! 对局渲染：占位渲染（SPEC 12.2）的背景、防御者、敌人、弹丸、Dew、悬停预览与调试形状；
//! `SkinMode::Skinned` 时角色改由 [`crate::skin`] 绘制，缺少皮肤的实体仍画占位（SPEC 12.1）。
//!
//! 只读模拟状态，不修改任何模拟组件或资源（AGENT 4.4）。

use std::collections::{BTreeMap, HashMap};

use rootguard_core::Grid;
use rootguard_core::components::{
    Armor, Cell, ChompState, Chomper, DefenderRef, DewPickup, EnemyRef, Health, Mine, OverlayOf,
    Placeholder, ProjectileBody, ProjectileRef, Shooter, StatusEffects, WalkState, Walker,
};
use rootguard_core::data::ArmorKind;
use rootguard_core::events::EnemyDamaged;
use rootguard_core::resources::{BattleState, LaneState};
use rootguard_core::systems::check_placement;
use trellia::prelude::*;

use crate::LOGICAL_SIZE;
use crate::hud;
use crate::skin::{self, Painter, Pose};

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
    /// 皮肤模式：生命 < 50% 的防御者着色（对应占位模式的红色描边）。
    pub const LOW_HP_TINT: u32 = 0xFF_B0B0;
}

/// 皮肤模式下 Chill 着色的混合比例（与占位模式一致）。
const CHILL_MIX: f32 = 0.5;

/// 同类角色的循环动画按实体错开相位，避免整齐划一（纯视觉）。
fn phase(e: Entity, tick: u64) -> u64 {
    tick + u64::from(e.id()) * 7
}

/// 收集后的 Dew 飞向 HUD 计数的时长（秒，SPEC 9“飘向 HUD 动画”）。
const DEW_FLY_SECS: f32 = 0.5;
/// 飞行结束时的缩放。
const DEW_FLY_END_SCALE: f32 = 0.5;
/// Dew 消失时过期计时剩余不超过该值视为“过期”而非“被收集”（一帧最多推进 5 tick，引擎 SPEC 4.2）。
const DEW_EXPIRE_SLACK_TICKS: u32 = 5;
/// 横幅入场时长（秒）与起始下移量（逻辑像素）。
const BANNER_IN_SECS: f32 = 0.35;
const BANNER_RISE_PX: f32 = 24.0;

/// 呈现层状态（纯视觉，AGENT 4.4）：受伤闪白时刻、Dew 收集飞行、横幅入场补间。
/// 补间按真实帧时间推进（引擎 SPEC 4.7），不参与模拟。
#[derive(Default)]
pub struct RenderState {
    /// 实体 → 最近受伤的 `fixed_tick`（只做按键查找）。
    hurt_at: HashMap<Entity, u64>,
    /// 上一帧可见的 Dew：实体位 → (位置, 过期计时剩余)。
    dew_seen: BTreeMap<u64, (Vec2, u32)>,
    /// 飞向 HUD 的 Dew。
    flying: Vec<TweenVec2>,
    /// 当前横幅文字与入场补间（0 → 1）。
    banner: Option<(String, TweenF32)>,
}

impl RenderState {
    /// 正在飞向 HUD 的 Dew 数。
    #[must_use]
    pub fn flying_dew(&self) -> usize {
        self.flying.len()
    }

    /// 横幅入场进度 0..=1（无横幅时为 `None`）。
    #[must_use]
    pub fn banner_progress(&self) -> Option<f32> {
        self.banner.as_ref().map(|(_, t)| t.value())
    }
}

/// `Update`（`BattleScene::update`，暂停时不调用）：推进全部呈现层状态。
pub fn track_visuals(world: &mut World) {
    track_hurt(world);
    let dt = world.resources.expect::<Time>().real_dt;
    let dew: BTreeMap<u64, (Vec2, u32)> = world
        .query::<(Entity, &Transform2D, &DewPickup)>()
        .iter()
        .map(|(e, t, d)| (e.to_bits().get(), (t.pos, d.expire.remaining)))
        .collect();
    let banner = hud::banner(world);
    let Some(state) = world.resources.get_mut::<RenderState>() else {
        return;
    };
    for (key, (pos, expire_left)) in &state.dew_seen {
        if !dew.contains_key(key) && *expire_left > DEW_EXPIRE_SLACK_TICKS {
            state.flying.push(TweenVec2::new(
                *pos,
                hud::DEW_ICON,
                DEW_FLY_SECS,
                Easing::EaseOutQuad,
            ));
        }
    }
    state.dew_seen = dew;
    for t in &mut state.flying {
        t.advance(dt);
    }
    state.flying.retain(|t| !t.finished());
    match (banner, &mut state.banner) {
        (None, slot) => *slot = None,
        (Some(text), Some((current, tween))) if *current == text => {
            tween.advance(dt);
        }
        (Some(text), slot) => {
            *slot = Some((
                text,
                TweenF32::new(0.0, 1.0, BANNER_IN_SECS, Easing::EaseOutQuad),
            ));
        }
    }
}

/// 横幅的绘制参数：(文字, 入场进度 0..=1)。
#[must_use]
pub fn banner_anim(world: &World) -> Option<(String, f32)> {
    let state = world.resources.get::<RenderState>()?;
    state
        .banner
        .as_ref()
        .map(|(text, t)| (text.clone(), t.value()))
}

/// 横幅入场时的下移量（逻辑像素）。
#[must_use]
pub fn banner_offset(progress: f32) -> f32 {
    (1.0 - progress) * BANNER_RISE_PX
}

/// 记录本帧可见的受伤事件，供闪白使用。
fn track_hurt(world: &mut World) {
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

/// 对局角色的图层；行遮挡由 `sort_y` 决定（SPEC 12.2，E-3）。
const WORLD: Layer = Layer::WORLD;

/// 行遮挡（SPEC 2.2 / 12.2）：同一行的全部命令共用行底 y，行内按提交顺序。
fn row_y(row: u8) -> f32 {
    Grid::sort_y(row)
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
    let painter = Painter::active(world);
    let painter = painter.as_ref();
    draw_background(world, &mut list);
    draw_defenders(world, &mut list, alpha, tick, painter);
    draw_enemies(world, &mut list, alpha, tick, painter);
    for (e, t, _, p, lane, pr) in world
        .query::<(
            Entity,
            &Transform2D,
            &ProjectileBody,
            &Placeholder,
            &rootguard_core::components::Lane,
            &ProjectileRef,
        )>()
        .iter()
    {
        let y = row_y(lane.row);
        let pos = t.lerp(alpha);
        let pose = Pose {
            clip: pr.id.as_str(),
            elapsed: phase(e, tick),
            flash: false,
            tint: Color::WHITE,
        };
        if !painter.is_some_and(|sk| sk.draw(&mut list, WORLD, pos, y, pose)) {
            list.circle_sorted(WORLD, y, pos, p.size.x / 2.0, color(p.color));
        }
    }
    // Dew 画在 HUD 之上：下落时会经过顶部 HUD 区域，仍需可见可点
    for (e, t, _) in world.query::<(Entity, &Transform2D, &DewPickup)>().iter() {
        let pos = t.lerp(alpha);
        let pose = Pose {
            clip: DEW_CLIP,
            elapsed: phase(e, tick),
            flash: false,
            tint: Color::WHITE,
        };
        if !painter.is_some_and(|sk| sk.draw(&mut list, DEW_LAYER, pos, pos.y, pose)) {
            draw_dew_placeholder(&mut list, pos, 1.0);
        }
    }
    // 收集后飞向 HUD 的 Dew（纯视觉，SPEC 9）
    if let Some(state) = world.resources.get::<RenderState>() {
        for t in &state.flying {
            let progress = if t.duration_secs > 0.0 {
                (t.elapsed_secs / t.duration_secs).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let pos = t.value();
            let pose = Pose {
                clip: DEW_CLIP,
                elapsed: 0,
                flash: false,
                tint: Color::WHITE,
            };
            if !painter.is_some_and(|sk| sk.draw(&mut list, DEW_LAYER, pos, pos.y, pose)) {
                let scale = 1.0 + (DEW_FLY_END_SCALE - 1.0) * progress;
                draw_dew_placeholder(&mut list, pos, scale);
            }
        }
    }
    draw_preview(world, &mut list);
    hud::draw(world, &mut list, painter);
    world.resources.insert(list);
    #[cfg(feature = "devtools")]
    debug_shapes(world);
}

/// Dew 所在图层：HUD 之上（ADR-003 第 3 条）。
pub const DEW_LAYER: Layer = Layer(Layer::OVERLAY.0 + 1);

/// Dew 的皮肤片段名。
pub const DEW_CLIP: &str = "dew";

/// 占位 Dew：黄色圆 + 微光描边（SPEC 12.2）；`scale` 用于收集飞行动画。
pub fn draw_dew_placeholder(list: &mut DrawList, pos: Vec2, scale: f32) {
    list.circle(
        DEW_LAYER,
        pos,
        22.0 * scale,
        color(palette::DEW_GLOW).with_alpha(0.5),
    );
    list.circle(DEW_LAYER, pos, 16.0 * scale, color(palette::DEW));
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

/// 防御者当前应播放的片段与已播放 tick（SPEC 12.3：idle；射手发射后 attack 一次；咬夹草咀嚼时
/// 循环 attack；块茎雷武装后 armed）。只读模拟组件。
fn defender_pose(
    sk: &Painter<'_>,
    id: &str,
    e: Entity,
    tick: u64,
    shooter: Option<&Shooter>,
    chomper: Option<&Chomper>,
    mine: Option<&Mine>,
) -> (String, u64) {
    let attack = format!("{id}_attack");
    if let Some(s) = shooter
        && !s.timer.finished()
        && let Some(len) = sk.ticks(&attack)
    {
        let since = u64::from(s.timer.duration.saturating_sub(s.timer.remaining));
        if since < len {
            return (attack, since);
        }
    }
    if chomper.is_some_and(|c| c.state == ChompState::Chewing) && sk.has(&attack) {
        return (attack, phase(e, tick));
    }
    let armed = format!("{id}_armed");
    if mine.is_some_and(|m| m.armed) && sk.has(&armed) {
        return (armed, phase(e, tick));
    }
    (format!("{id}_idle"), phase(e, tick))
}

fn draw_defenders(
    world: &World,
    list: &mut DrawList,
    alpha: f32,
    tick: u64,
    painter: Option<&Painter<'_>>,
) {
    // 地面防御者
    for (e, t, p, h, cell, d, shooter, chomper, mine) in world
        .query::<(
            Entity,
            &Transform2D,
            &Placeholder,
            &Health,
            &Cell,
            &DefenderRef,
            Option<&Shooter>,
            Option<&Chomper>,
            Option<&Mine>,
        )>()
        .without::<&OverlayOf>()
        .iter()
    {
        if !h.alive() {
            continue;
        }
        let y = row_y(cell.row);
        let pos = t.lerp(alpha);
        if let Some(sk) = painter {
            let (clip, elapsed) = defender_pose(sk, d.id.as_str(), e, tick, shooter, chomper, mine);
            let tint = if h.hp * 2 < h.max {
                color(palette::LOW_HP_TINT)
            } else {
                Color::WHITE
            };
            let pose = Pose {
                clip: &clip,
                elapsed,
                flash: false,
                tint,
            };
            if sk.draw(list, WORLD, skin::foot(pos), y, pose) {
                continue;
            }
        }
        let body = Rect::from_center_size(pos, p.size);
        list.rect_sorted(WORLD, y, body, color(p.color));
        if h.hp * 2 < h.max {
            list.rect_stroke_sorted(WORLD, y, body, color(palette::LOW_HP), 3.0);
        }
        list.text_sorted(
            WORLD,
            y,
            pos - Vec2::new(0.0, 16.0),
            p.label.as_str(),
            28.0,
            Color::WHITE,
        );
    }
    // 覆盖层：宿主外围描边环（皮肤模式：环形皮肤，排在宿主之后）
    for (e, t, p, h, cell, d, _) in world
        .query::<(
            Entity,
            &Transform2D,
            &Placeholder,
            &Health,
            &Cell,
            &DefenderRef,
            &OverlayOf,
        )>()
        .iter()
    {
        if !h.alive() {
            continue;
        }
        let pos = t.lerp(alpha);
        if let Some(sk) = painter {
            let clip = format!("{}_idle", d.id.as_str());
            let tint = if h.hp * 2 < h.max {
                color(palette::LOW_HP_TINT)
            } else {
                Color::WHITE
            };
            let pose = Pose {
                clip: &clip,
                elapsed: phase(e, tick),
                flash: false,
                tint,
            };
            if sk.draw(list, WORLD, skin::foot(pos), row_y(cell.row), pose) {
                continue;
            }
        }
        let ring = Rect::from_center_size(pos, p.size + Vec2::splat(12.0));
        let c = if h.hp * 2 < h.max {
            color(palette::LOW_HP)
        } else {
            color(palette::OVERLAY_RING)
        };
        list.rect_stroke_sorted(WORLD, row_y(cell.row), ring, c, 6.0);
    }
}

/// 敌人当前应播放的片段（SPEC 12.3：walk / eat，护甲破损后换 `bare_` 版）。
fn enemy_clip(sk: &Painter<'_>, id: &str, eating: bool, armored: bool) -> String {
    let act = if eating { "eat" } else { "walk" };
    if !armored {
        let bare = format!("{id}_bare_{act}");
        if sk.has(&bare) {
            return bare;
        }
    }
    format!("{id}_{act}")
}

fn draw_enemies(
    world: &World,
    list: &mut DrawList,
    alpha: f32,
    tick: u64,
    painter: Option<&Painter<'_>>,
) {
    let hurt = world.resources.get::<RenderState>();
    for (e, t, p, h, lane, status, armor, enemy, walker) in world
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
        let y = row_y(lane.row);
        let pos = t.lerp(alpha);
        let flashing = hurt
            .and_then(|s| s.hurt_at.get(&e))
            .is_some_and(|&at| tick.saturating_sub(at) < HURT_FLASH_TICKS);
        let armored = armor.is_some_and(|a| a.hp > 0);
        if let Some(sk) = painter {
            let eating = matches!(walker.state, WalkState::Eating(_));
            let clip = enemy_clip(sk, enemy.id.as_str(), eating, armored);
            let tint = if status.chilled() {
                mix(Color::WHITE, color(palette::CHILL_TINT), CHILL_MIX)
            } else {
                Color::WHITE
            };
            let pose = Pose {
                clip: &clip,
                elapsed: phase(e, tick),
                flash: flashing,
                tint,
            };
            if sk.draw(list, WORLD, skin::foot(pos), y, pose) {
                continue;
            }
        }
        let mut c = color(p.color);
        if status.chilled() {
            c = mix(c, color(palette::CHILL_TINT), CHILL_MIX);
        }
        if flashing {
            c = mix(c, Color::WHITE, 0.7);
        }
        let body = Rect::from_center_size(pos, p.size);
        list.rect_sorted(WORLD, y, body, c);
        if let Some(a) = armor.filter(|a| a.hp > 0) {
            match a.kind {
                ArmorKind::Helm => list.rect_sorted(
                    WORLD,
                    y,
                    Rect::from_center_size(
                        Vec2::new(pos.x, body.min.y - 6.0),
                        Vec2::new(p.size.x + 6.0, 16.0),
                    ),
                    color(palette::HELM),
                ),
                ArmorKind::Shield => list.rect_sorted(
                    WORLD,
                    y,
                    Rect::from_center_size(
                        Vec2::new(body.min.x - 6.0, pos.y),
                        Vec2::new(12.0, p.size.y + 10.0),
                    ),
                    color(palette::SHIELD),
                ),
                ArmorKind::Drum => list.circle_sorted(
                    WORLD,
                    y,
                    pos + Vec2::new(0.0, 18.0),
                    20.0,
                    color(palette::DRUM),
                ),
            }
        }
        list.text_sorted(
            WORLD,
            y,
            pos - Vec2::new(0.0, 16.0),
            p.label.as_str(),
            26.0,
            Color::WHITE,
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
