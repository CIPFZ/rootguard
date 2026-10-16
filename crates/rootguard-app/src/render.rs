//! 对局渲染：占位渲染（SPEC 12.2）的背景、防御者、敌人、弹丸、Dew、悬停预览与调试形状；
//! `SkinMode::Skinned` 时角色改由 [`crate::skin`] 绘制，缺少皮肤的实体仍画占位（SPEC 12.1）。
//!
//! 只读模拟状态，不修改任何模拟组件或资源（AGENT 4.4）。

use std::collections::{BTreeMap, HashMap};

use rootguard_core::Grid;
use rootguard_core::components::{
    Armor, Biter, Blast, Cell, ChompState, Chomper, DefenderRef, DewPickup, EnemyRef, Gust, Health,
    Mine, OverlayOf, Placeholder, Producer, ProjectileBody, ProjectileRef, Shooter, Spikes,
    StatusEffects, Vaulter, WalkState, Walker,
};
use rootguard_core::data::{ArmorKind, Rules, TimeOfDay};
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
    /// 战场底图上的非激活车道遮罩（ART-SPEC 8.1 L3：40% `#1E2A1A`）。
    pub const LANE_MASK: u32 = 0x1E_2A1A;
}

/// 非激活车道遮罩的不透明度（ART-SPEC 8.1 L3）。
const LANE_MASK_ALPHA: f32 = 0.4;

/// 皮肤中的战场底图片段（ART-SPEC 8.1 L0 ～ L2，`tools/artgen` 生成）。
pub const BG_CLIP: &str = "bg_base";

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
/// 一帧最多推进的 FixedUpdate tick 数（引擎 SPEC 4.2）。
const MAX_TICKS_PER_FRAME: u32 = 5;
/// Dew 消失时过期计时剩余不超过该值视为“过期”而非“被收集”。
const DEW_EXPIRE_SLACK_TICKS: u32 = MAX_TICKS_PER_FRAME;
/// 横幅入场时长（秒）与起始下移量（逻辑像素）。
const BANNER_IN_SECS: f32 = 0.35;
const BANNER_RISE_PX: f32 = 24.0;

/// 呈现层状态（纯视觉，AGENT 4.4）：受伤闪白时刻、Dew 收集飞行、横幅入场补间、动画事件进度。
/// 补间按真实帧时间推进（引擎 SPEC 4.7），不参与模拟。
#[derive(Default)]
pub struct RenderState {
    /// 实体 → 最近受伤的 `fixed_tick`（只做按键查找）。
    hurt_at: HashMap<Entity, u64>,
    /// 防御者上一帧的生命值（只做按键查找），用于判定受伤闪白。
    defender_hp: HashMap<Entity, i32>,
    /// 敌人实体位 → 呈现层首次看到它的 `fixed_tick`（`spawn` 片段，ADR-016 第 4 节）。
    first_seen: BTreeMap<u64, u64>,
    /// 上一帧可见的 Dew：实体位 → (位置, 过期计时剩余)。
    dew_seen: BTreeMap<u64, (Vec2, u32)>,
    /// 飞向 HUD 的 Dew。
    flying: Vec<TweenVec2>,
    /// 当前横幅文字与入场补间（0 → 1）。
    banner: Option<(String, TweenF32)>,
    /// 上一帧处于啃食的敌人：实体位 → (啃食目标, `Biter.elapsed`)，用于判断 eat 片段进入事件帧。
    eat_seen: BTreeMap<u64, (Entity, u32)>,
    /// 正在播放的特效（ART-SPEC 7，[`crate::fx`]）。
    pub(crate) fx: Vec<crate::fx::Fx>,
    /// 上一帧生成特效的事件键（去重双缓冲事件，[`crate::fx::capture`]）。
    pub(crate) fx_keys: std::collections::BTreeSet<String>,
    /// 飞行中的弹丸：实体位 → 弹丸 id（命中特效用，[`crate::fx::capture`]）。
    pub(crate) projectile_kinds: BTreeMap<u64, String>,
    /// 防御者实体位 → 呈现层首次看到它的 `fixed_tick`（放置时的弹性缩放，ART-SPEC 4.13）。
    defender_born: BTreeMap<u64, u64>,
    /// 是否已过进入对局的第一帧。
    anim_started: bool,
    /// 上一帧各角色的片段与已播放 tick（非 eat 片段的事件帧，ADR-016 第 7 节）。
    clip_seen: BTreeMap<u64, (String, u64)>,
}

impl RenderState {
    /// 正在播放的特效。
    #[must_use]
    pub fn effects(&self) -> &[crate::fx::Fx] {
        &self.fx
    }

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
    track_enemies(world);
    crate::fx::prune(world);
    emit_anim_events(world);
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

/// 记录每个敌人首次出现的 tick（生成动画用），并删掉已不存在的。
fn track_enemies(world: &mut World) {
    let tick = world.resources.expect::<Time>().fixed_tick;
    let alive: Vec<u64> = world
        .query::<(Entity, &EnemyRef)>()
        .iter()
        .map(|(e, _)| e.to_bits().get())
        .collect();
    let defenders: Vec<u64> = world
        .query::<(Entity, &DefenderRef)>()
        .iter()
        .map(|(e, _)| e.to_bits().get())
        .collect();
    if let Some(state) = world.resources.get_mut::<RenderState>() {
        let mut next = BTreeMap::new();
        for key in alive {
            next.insert(key, state.first_seen.get(&key).copied().unwrap_or(tick));
        }
        state.first_seen = next;
        // 进入对局第一帧已存在的防御者不做放置动画
        let first = state.anim_started;
        let mut born = BTreeMap::new();
        for key in defenders {
            let at = state
                .defender_born
                .get(&key)
                .copied()
                .unwrap_or(if first { tick } else { 0 });
            born.insert(key, at);
        }
        state.defender_born = born;
        state.anim_started = true;
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

/// 记录本帧受伤的实体，供闪白使用（ART-SPEC 2.3）：敌人看 `EnemyDamaged` 事件；防御者没有受伤事件，
/// 以生命值比上一帧减少判定（纯视觉）。
fn track_hurt(world: &mut World) {
    let tick = world.resources.expect::<Time>().fixed_tick;
    let mut hurt: Vec<Entity> = world
        .events
        .get::<EnemyDamaged>()
        .iter()
        .map(|d| d.entity)
        .collect();
    let hp: Vec<(Entity, i32)> = world
        .query::<(Entity, &Health, &DefenderRef)>()
        .iter()
        .map(|(e, h, _)| (e, h.hp))
        .collect();
    if let Some(state) = world.resources.get_mut::<RenderState>() {
        let mut next = HashMap::with_capacity(hp.len());
        for (e, now) in hp {
            if state
                .defender_hp
                .get(&e)
                .is_some_and(|&before| now < before)
            {
                hurt.push(e);
            }
            next.insert(e, now);
        }
        state.defender_hp = next;
        // 事件在两帧内可见，重复写入只会让闪白多持续一帧（纯视觉）
        for e in hurt {
            state.hurt_at.insert(e, tick);
        }
        state
            .hurt_at
            .retain(|_, at| tick.saturating_sub(*at) < HURT_FLASH_TICKS);
    }
}

/// 敌人 eat 片段进入事件帧（`eat_hit`）时发送引擎 `AnimationEvent`（SPEC 12.3，ADR-010），供音效消费。
///
/// eat 片段的相位由 `Biter` 计时器决定（[`eat_elapsed`]，负责人裁决 2026-10-15（四）），因此只需比较
/// 本帧与上一帧的 `Biter.elapsed`：未回绕时检查两者之间进入的事件帧，回绕（本帧之间发生过一次啃食）
/// 时把本帧位置加一个片段长度。啃食目标变化或刚开始啃食时只记录不发送。与渲染模式无关，只要皮肤已
/// 加载就发送，使两种模式的音效一致；暂停时不调用，`Biter` 也不推进，恢复后不会补发。
fn emit_anim_events(world: &mut World) {
    let factor = chill_bite_factor(world);
    let Some(seen) = world
        .resources
        .get_mut::<RenderState>()
        .map(|s| std::mem::take(&mut s.eat_seen))
    else {
        return;
    };
    // 与其他片段事件一样固定用 garden 视图（ADR-016 第 7 节）
    let (Some(skin), Some(clips)) = (
        world
            .resources
            .get::<skin::Skins>()
            .map(|s| s.view(rootguard_core::events::SkinName::Garden)),
        world.resources.get::<Assets<AnimationClip>>(),
    ) else {
        return;
    };
    let mut next = BTreeMap::new();
    let mut events = Vec::new();
    for (e, h, armor, enemy, walker, biter, status, vaulter) in world
        .query::<(
            Entity,
            &Health,
            Option<&Armor>,
            &EnemyRef,
            &Walker,
            &Biter,
            &StatusEffects,
            Option<&Vaulter>,
        )>()
        .iter()
    {
        let WalkState::Eating(target) = walker.state else {
            continue;
        };
        if !h.alive() {
            continue;
        }
        let key = e.to_bits().get();
        next.insert(key, (target, biter.elapsed));
        let Some(&(prev_target, prev)) = seen.get(&key) else {
            continue;
        };
        if prev_target != target {
            continue;
        }
        let look = EnemyLook {
            id: enemy.id.as_str(),
            x: 0.0,
            walker,
            biter: Some(biter),
            status,
            armor,
            vaulter,
            def_speed: None,
            age: None,
        };
        let name = enemy_clip(
            |c| skin.has_clip(c),
            enemy.id.as_str(),
            enemy_state(&look),
            "eat",
        );
        let Some(clip) = skin.clip(&name).and_then(|h| clips.get(h)) else {
            continue;
        };
        let len = skin::clip_ticks(clip);
        let interval = bite_interval(biter, status, factor);
        let from = eat_elapsed(prev, interval, len);
        let mut to = eat_elapsed(biter.elapsed, interval, len);
        if biter.elapsed < prev {
            to += len;
        }
        for event in skin::events_between(clip, from, to) {
            events.push(AnimationEvent {
                entity: e,
                event: event.to_owned(),
            });
        }
    }
    let (clip_events, clip_seen) = other_clip_events(world, factor);
    events.extend(clip_events);
    if let Some(state) = world.resources.get_mut::<RenderState>() {
        state.eat_seen = next;
        state.clip_seen = clip_seen;
    }
    for event in events {
        world.events.send(event);
    }
}

/// 片段从 `prev`（上一帧的片段与已播放 tick）推进到 `name` / `now` 时进入的事件帧。片段改变（或第一次看到）
/// 时从片段开头算（第 0 帧的事件只在仍处于第 0 帧时算）；同片段倒退（循环回绕或被风推回）时补一个周期。
fn clip_events_since<'c>(
    clip: &'c AnimationClip,
    name: &str,
    prev: Option<&(String, u64)>,
    now: u64,
) -> Vec<&'c str> {
    let len = skin::clip_ticks(clip);
    match prev {
        Some((p, from)) if p == name && now >= *from => {
            skin::events_between(clip, *from, now).collect()
        }
        Some((p, from)) if p == name && clip.looping => {
            skin::events_between(clip, *from, now + len).collect()
        }
        // 新片段：只有确实从第 0 帧开始（如跨越刚开始）才算进入第 0 帧；中途切入（敌人走进荆棘垫时扎刺计时器
        // 已在周期中段）不发第 0 帧的事件
        _ => clip
            .frames
            .first()
            .filter(|f| now < u64::from(f.duration_ticks))
            .and_then(|f| f.event.as_deref())
            .into_iter()
            .chain(skin::events_between(clip, 0, now))
            .collect(),
    }
}

/// 啃食以外的片段事件帧（ART-SPEC 3.2）：敌人 walk / sprint / vault / spawn（鼓腹枯壳每步 `thud`、跳栏 `vault`）
/// 与防御者片段（荆棘垫扎刺 `prick`、咬夹草 `gulp`）。相位与绘制同一规则；不论渲染模式。
fn other_clip_events(
    world: &World,
    factor: u32,
) -> (Vec<AnimationEvent>, BTreeMap<u64, (String, u64)>) {
    let mut events = Vec::new();
    let mut seen = BTreeMap::new();
    let (Some(sk), Some(state)) = (
        Painter::for_events(world),
        world.resources.get::<RenderState>(),
    ) else {
        return (events, seen);
    };
    let tick = world.resources.expect::<Time>().fixed_tick;
    let mut emit = |e: Entity, name: String, elapsed: u64| {
        let key = e.to_bits().get();
        if let Some(clip) = sk.clip_data(&name).filter(|_| !name.ends_with("_eat")) {
            for ev in clip_events_since(clip, &name, state.clip_seen.get(&key), elapsed) {
                events.push(AnimationEvent {
                    entity: e,
                    event: ev.to_owned(),
                });
            }
        }
        seen.insert(key, (name, elapsed));
    };
    for (e, enemy, t, walker, armor, biter, status, vaulter, h) in world
        .query::<(
            Entity,
            &EnemyRef,
            &Transform2D,
            &Walker,
            Option<&Armor>,
            Option<&Biter>,
            &StatusEffects,
            Option<&Vaulter>,
            &Health,
        )>()
        .iter()
    {
        if !h.alive() {
            continue;
        }
        let look = EnemyLook {
            id: enemy.id.as_str(),
            x: t.pos.x,
            walker,
            biter,
            status,
            armor,
            vaulter,
            def_speed: enemy_def_speed(world, enemy),
            age: enemy_age(world, e, tick),
        };
        let (name, elapsed) = enemy_pose_with(|c| sk.ticks(c), &look, tick, e, factor);
        // 啃食事件由 Biter 相位单独判定（上方），这里只记录片段
        emit(e, name, elapsed);
    }
    let enemies = enemy_positions(world);
    for (e, d, h, cell, shooter, mine, blast, gust, chomper, spikes) in world
        .query::<(
            Entity,
            &DefenderRef,
            &Health,
            &Cell,
            Option<&Shooter>,
            Option<&Mine>,
            Option<&Blast>,
            Option<&Gust>,
            Option<&Chomper>,
            Option<&Spikes>,
        )>()
        .without::<&OverlayOf>()
        .iter()
    {
        if !h.alive() || (chomper.is_none() && spikes.is_none()) {
            continue;
        }
        let timers = DefenderTimers {
            shooter,
            mine,
            blast,
            gust,
            chomper,
            spikes,
            occupied: cell_occupied(&enemies, *cell),
        };
        let state_name = defender_state(|c| sk.has(c), d.id.as_str(), h);
        let (name, elapsed) = defender_pose_with(&sk, d.id.as_str(), e, tick, state_name, timers);
        emit(e, name, elapsed);
    }
    (events, seen)
}

/// Chill 啃食间隔倍率（规则数据，SPEC 8.2）；规则缺失时为 1。
fn chill_bite_factor(world: &World) -> u32 {
    world
        .resources
        .get::<Rules>()
        .map_or(1, |r| r.status.chill_bite_interval_factor)
}

/// 本 tick 的有效啃食间隔：与 `bite_system` 同一算法（SPEC 8.2，Chill 时按规则倍增）。
fn bite_interval(biter: &Biter, status: &StatusEffects, chill_factor: u32) -> u32 {
    let interval = if status.chilled() {
        biter.interval * chill_factor
    } else {
        biter.interval
    };
    interval.max(1)
}

/// eat 片段的已播放 tick：啃食进度 `elapsed / interval` 映射到片段总长（负责人裁决 2026-10-15（四））。
/// 片段末帧带 `eat_hit`，对应计时器到期前的最后一段；Chill 下间隔翻倍，相位自动随之变慢。
#[must_use]
pub fn eat_elapsed(elapsed: u32, interval: u32, clip_ticks: u64) -> u64 {
    let interval = interval.max(1);
    u64::from(elapsed.min(interval - 1)) * clip_ticks / u64::from(interval)
}

/// 行走相位的参考 x：距离 = 参考 x − 当前 x（任意常数都可，取逻辑宽度）。
const WALK_REF_X: f32 = 1280.0;

/// 敌人的呈现输入（只读模拟组件）。
#[derive(Clone, Copy)]
struct EnemyLook<'a> {
    id: &'a str,
    /// 当前 x（插值后）。
    x: f32,
    walker: &'a Walker,
    biter: Option<&'a Biter>,
    status: &'a StatusEffects,
    armor: Option<&'a Armor>,
    vaulter: Option<&'a Vaulter>,
    /// 数据中的基础速度（Sprint 判定用）。
    def_speed: Option<f32>,
    /// 生成后经过的 tick（呈现层记录，`spawn` 用）。
    age: Option<u64>,
}

/// 敌人的状态前缀（ART-SPEC 2.4 / 5.5，ADR-016 第 4 节）：跨越落地后 `vaulted`；护甲 ≤ 1/2 时 `cracked`；
/// 护甲没了 `bare`；否则无。皮肤缺少带前缀的片段时回退无前缀的。
fn enemy_state(look: &EnemyLook<'_>) -> &'static str {
    if look.vaulter.is_some_and(|v| v.used)
        && !matches!(look.walker.state, WalkState::Vaulting { .. })
    {
        return "vaulted";
    }
    match look.armor {
        Some(a) if a.hp > 0 && a.hp * 2 <= a.max => "cracked",
        Some(a) if a.hp > 0 => "",
        _ => "bare",
    }
}

/// 片段名：`<id>_<状态>_<动作>`，缺失时回退 `<id>_<动作>`。
fn enemy_clip(has: impl Fn(&str) -> bool, id: &str, state: &str, act: &str) -> String {
    if !state.is_empty() {
        let name = format!("{id}_{state}_{act}");
        if has(&name) {
            return name;
        }
    }
    format!("{id}_{act}")
}

/// 位置驱动的循环相位（ADR-016 第 4 节）：走过的距离 ÷ 步幅 × 片段长，步幅 = 速度 × 片段长（ART-SPEC 3.1“基础
/// 速度 0.30 时一周期 12 px”）。脚不打滑，Chill 减速、停步时自然变慢 / 停住。
#[must_use]
pub fn stride_elapsed(x: f32, speed: f32, len: u64) -> u64 {
    let stride = speed * len as f32;
    if stride <= 0.0 {
        return 0;
    }
    let cycles = ((WALK_REF_X - x) / stride).rem_euclid(1.0);
    ((cycles * len as f32) as u64).min(len.saturating_sub(1))
}

/// 敌人当前应播放的片段与已播放 tick：`eat` 按 Biter 相位（[`eat_elapsed`]）、`vault` 按跨越进度、`spawn` 按生成
/// 后经过的 tick、`sprint` / `walk` 按走过的距离（[`stride_elapsed`]）。`ticks` 返回片段总长（缺失为 `None`）。
fn enemy_pose_with(
    ticks: impl Fn(&str) -> Option<u64>,
    look: &EnemyLook<'_>,
    tick: u64,
    e: Entity,
    factor: u32,
) -> (String, u64) {
    let has = |c: &str| ticks(c).is_some();
    let state = enemy_state(look);
    let clip = |act: &str| enemy_clip(has, look.id, state, act);
    match look.walker.state {
        WalkState::Eating(_) => {
            let name = clip("eat");
            let elapsed = match (look.biter, ticks(&name)) {
                (Some(b), Some(len)) => {
                    eat_elapsed(b.elapsed, bite_interval(b, look.status, factor), len)
                }
                _ => phase(e, tick),
            };
            return (name, elapsed);
        }
        WalkState::Vaulting { t, .. } => {
            let name = clip("vault");
            if let (Some(len), Some(v)) = (ticks(&name), look.vaulter)
                && let Some(el) = timed_elapsed(t, v.ticks, len)
            {
                return (name, el);
            }
        }
        _ => {}
    }
    let spawn = clip("spawn");
    if let (Some(len), Some(age)) = (ticks(&spawn), look.age)
        && age < len
    {
        return (spawn, age);
    }
    let sprinting = look
        .def_speed
        .is_some_and(|s| look.walker.base_speed > s + f32::EPSILON);
    let sprint = clip("sprint");
    let name = if sprinting && has(&sprint) {
        sprint
    } else {
        clip("walk")
    };
    let elapsed = ticks(&name).map_or_else(
        || phase(e, tick),
        |len| stride_elapsed(look.x, look.walker.base_speed, len),
    );
    (name, elapsed)
}

/// 敌人 `e` 当前的皮肤片段名与已播放 tick（与绘制、动画事件同一规则）；不是存活敌人或皮肤未加载时为 `None`。
#[must_use]
pub fn enemy_pose(world: &World, e: Entity) -> Option<(String, u64)> {
    let skin = skin::SkinView::selected(world)?;
    let clips = world.resources.get::<Assets<AnimationClip>>()?;
    let tick = world.resources.get::<Time>()?.fixed_tick;
    let factor = chill_bite_factor(world);
    let mut query = world.entities.query_one::<(
        &EnemyRef,
        &Transform2D,
        &Walker,
        Option<&Armor>,
        Option<&Biter>,
        &StatusEffects,
        Option<&Vaulter>,
    )>(e);
    let (enemy, t, walker, armor, biter, status, vaulter) = query.get().ok()?;
    let ticks = |c: &str| {
        skin.clip(c)
            .and_then(|h| clips.get(h))
            .map(skin::clip_ticks)
    };
    let look = EnemyLook {
        id: enemy.id.as_str(),
        x: t.pos.x,
        walker,
        biter,
        status,
        armor,
        vaulter,
        def_speed: enemy_def_speed(world, enemy),
        age: enemy_age(world, e, tick),
    };
    Some(enemy_pose_with(ticks, &look, tick, e, factor))
}

/// 数据中的基础速度（`EnemyRegistry`）。
fn enemy_def_speed(world: &World, enemy: &EnemyRef) -> Option<f32> {
    world
        .resources
        .get::<rootguard_core::registry::EnemyRegistry>()?
        .get(enemy.id)
        .map(|d| d.speed)
}

/// 敌人生成后经过的 tick（呈现层首次看到它的 tick 起算）。
fn enemy_age(world: &World, e: Entity, tick: u64) -> Option<u64> {
    let born = *world
        .resources
        .get::<RenderState>()?
        .first_seen
        .get(&e.to_bits().get())?;
    Some(tick.saturating_sub(born))
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
    draw_background(world, &mut list, painter);
    draw_defenders(world, &mut list, alpha, tick, painter);
    draw_enemies(world, &mut list, alpha, tick, painter);
    crate::fx::draw(world, &mut list, painter, tick);
    if painter.is_some() {
        draw_time_tint(world, &mut list);
    }
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
        let clip = painter.map_or_else(String::new, |sk| projectile_clip(sk, pr.id.as_str()));
        let pose = Pose {
            clip: &clip,
            elapsed: phase(e, tick),
            flash: false,
            tint: Color::WHITE,
        };
        if !painter.is_some_and(|sk| sk.draw(&mut list, WORLD, pos, y, pose)) {
            list.circle_sorted(WORLD, y, pos, p.size.x / 2.0, color(p.color));
        }
    }
    // Dew 画在 HUD 之上：下落时会经过顶部 HUD 区域，仍需可见可点
    let dew_clip = painter.map_or(DEW_CLIP, dew_clip);
    for (e, t, dew) in world.query::<(Entity, &Transform2D, &DewPickup)>().iter() {
        let mut pos = t.lerp(alpha);
        if painter.is_some() {
            // ART-SPEC 6：下落时轻微左右摆；过期前 180 tick 开始闪烁（纯视觉）
            if !dew.landed {
                pos.x += dew_sway(phase(e, tick));
            } else if dew.expire.remaining < DEW_BLINK_TICKS
                && dew_blink_hidden(dew.expire.remaining)
            {
                continue;
            }
        }
        let pose = Pose {
            clip: dew_clip,
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
                clip: dew_clip,
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

/// Dew 的皮肤片段名（`default` 皮肤）；garden 皮肤为 [`DEW_IDLE_CLIP`]。
pub const DEW_CLIP: &str = "dew";
/// garden 皮肤的 Dew 静置片段（ART-SPEC 6，`tools/artgen` 的 `<id>_<片段>` 命名）。
pub const DEW_IDLE_CLIP: &str = "dew_idle";
/// Dew 过期前开始闪烁的 tick 数（ART-SPEC 6）。
const DEW_BLINK_TICKS: u32 = 180;
/// 闪烁的半周期（tick）。
const DEW_BLINK_HALF_TICKS: u32 = 10;
/// 下落时左右摆的幅度（px）与周期（tick）。
const DEW_SWAY_PX: f32 = 2.5;
const DEW_SWAY_TICKS: u64 = 48;

/// 皮肤中 Dew 用的片段：优先 garden 的 `dew_idle`。
fn dew_clip(sk: &Painter<'_>) -> &'static str {
    if sk.has(DEW_IDLE_CLIP) {
        DEW_IDLE_CLIP
    } else {
        DEW_CLIP
    }
}

/// 弹丸片段：优先 garden 的 `<id>_fly`，否则 `default` 皮肤的 `<id>`。
fn projectile_clip(sk: &Painter<'_>, id: &str) -> String {
    let fly = format!("{id}_fly");
    if sk.has(&fly) { fly } else { id.to_owned() }
}

/// 下落中的 Dew 的水平摆动（三角波，纯视觉）。
fn dew_sway(phase: u64) -> f32 {
    let t = (phase % DEW_SWAY_TICKS) as f32 / DEW_SWAY_TICKS as f32;
    DEW_SWAY_PX * (4.0 * (t - 0.5).abs() - 1.0)
}

/// 闪烁中本帧是否隐藏。
fn dew_blink_hidden(remaining: u32) -> bool {
    (remaining / DEW_BLINK_HALF_TICKS) % 2 == 1
}

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

/// 背景：皮肤有战场底图时画底图并按关卡叠加非激活车道遮罩（ART-SPEC 8.1 L0 ～ L3），否则画占位色带
/// （SPEC 12.2）。温室外墙、入口与 Hose 标记目前两种模式共用占位图形（L4 ～ L6 在 A3）。
fn draw_background(world: &World, list: &mut DrawList, painter: Option<&Painter<'_>>) {
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
    let bg = Pose {
        clip: BG_CLIP,
        elapsed: 0,
        flash: false,
        tint: Color::WHITE,
    };
    if painter.is_some_and(|sk| sk.draw(list, Layer::BACKGROUND, Vec2::ZERO, 0.0, bg)) {
        for row in (0..Grid::ROWS).filter(|&r| !lanes.is_active(r)) {
            let top = Grid::ORIGIN.y + Grid::CELL_SIZE.y * f32::from(row);
            list.rect(
                Layer::BACKGROUND,
                Rect::from_min_size(
                    Vec2::new(Grid::ORIGIN.x, top),
                    Vec2::new(Grid::CELL_SIZE.x * f32::from(Grid::COLS), Grid::CELL_SIZE.y),
                ),
                color(palette::LANE_MASK).with_alpha(LANE_MASK_ALPHA),
            );
        }
        if !painter.is_some_and(|sk| draw_scene(list, sk, &lanes)) {
            draw_wall_and_entrance(list, &lanes);
        }
        return;
    }
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
    draw_wall_and_entrance(list, &lanes);
}

/// 左侧温室（含 Hose 可用标记）与右侧入口的占位图形（SPEC 12.2；皮肤版在 A3，ART-SPEC 8.1 L4 / L5）。
/// 时段色调叠层所在层：`Layer::OVERLAY` 下一层（ART-SPEC 8.2），盖住战场与特效，不盖 HUD。
pub const TIME_TINT_LAYER: Layer = Layer(Layer::OVERLAY.0 - 1);

/// 时段色调（ART-SPEC 8.2）：(颜色, 不透明度)；上午无叠层。
#[must_use]
pub fn time_tint(time: TimeOfDay) -> Option<(u32, f32)> {
    match time {
        TimeOfDay::Dawn => Some((0x9D_B9FF, 0.12)),
        TimeOfDay::Morning => None,
        TimeOfDay::Noon => Some((0xFF_F2C2, 0.10)),
        TimeOfDay::Dusk => Some((0xFF_9A5C, 0.18)),
    }
}

/// 整屏时段色调（皮肤模式；占位模式保持原样，便于读数）。
fn draw_time_tint(world: &World, list: &mut DrawList) {
    let time = world
        .resources
        .get::<BattleState>()
        .and_then(|s| {
            world
                .resources
                .get::<rootguard_core::registry::LevelRegistry>()?
                .get(s.level)
                .map(|l| l.time_of_day)
        })
        .unwrap_or_default();
    if let Some((hex, a)) = time_tint(time) {
        list.rect(
            TIME_TINT_LAYER,
            Rect::from_min_size(Vec2::ZERO, LOGICAL_SIZE),
            color(hex).with_alpha(a),
        );
    }
}

/// 战场精灵片段（ART-SPEC 8.1 L4 ～ L6，`artgen` 的 `scene_<名>`）。
pub const SCENE_WALL: &str = "scene_greenhouse_wall";
/// 阀门两态：默认关闭；该行的 Hose 用掉后歪斜滴水。
pub const SCENE_VALVE_CLOSED: &str = "scene_valve_closed";
/// 见 [`SCENE_VALVE_CLOSED`]。
pub const SCENE_VALVE_SPENT: &str = "scene_valve_spent";
const SCENE_ENTRANCE: &str = "scene_entrance";
/// 栅栏门柱：画在该行角色之前（Husk 从门后走出）。
pub const SCENE_GATE_POST: &str = "scene_gate_post";
const SCENE_PROPS_LEFT: &str = "scene_props_left";
const SCENE_PROPS_RIGHT: &str = "scene_props_right";
/// 门柱的 x（栅栏线）与相对行下沿的下移量。
const GATE_POST_X: f32 = 1174.0;
const GATE_POST_DROP: f32 = 4.0;
/// 门柱相对同一行角色的排序偏移：同行之后、下一行之前。
const GATE_POST_SORT: f32 = 0.5;

/// 皮肤的战场精灵（ART-SPEC 8.1 L4 ～ L6）：温室外墙、每行阀门（按 `hose_available` 两态）、右侧入口与门柱、
/// 前景杂物。皮肤缺少外墙时返回 false（调用方改画占位图形）。非激活车道也画阀门（关闭），外墙本来就贯穿 5 行。
fn draw_scene(list: &mut DrawList, sk: &Painter<'_>, lanes: &LaneState) -> bool {
    let still = |clip| Pose {
        clip,
        elapsed: 0,
        flash: false,
        tint: Color::WHITE,
    };
    let bg = Layer::BACKGROUND;
    if !sk.draw(
        list,
        bg,
        Vec2::new(0.0, Grid::ORIGIN.y),
        0.0,
        still(SCENE_WALL),
    ) {
        return false;
    }
    sk.draw(
        list,
        bg,
        Vec2::new(Grid::RIGHT_X, Grid::ORIGIN.y),
        0.0,
        still(SCENE_ENTRANCE),
    );
    sk.draw(
        list,
        bg,
        Vec2::new(0.0, LOGICAL_SIZE.y),
        0.0,
        still(SCENE_PROPS_LEFT),
    );
    sk.draw(list, bg, LOGICAL_SIZE, 0.0, still(SCENE_PROPS_RIGHT));
    for row in 0..Grid::ROWS {
        let spent = lanes.is_active(row) && !lanes.hose_available[usize::from(row)];
        let valve = if spent {
            SCENE_VALVE_SPENT
        } else {
            SCENE_VALVE_CLOSED
        };
        let y = Grid::row_center_y(row);
        sk.draw(list, bg, Vec2::new(Grid::HOSE_X, y), 0.0, still(valve));
        let bottom = Grid::sort_y(row);
        sk.draw(
            list,
            WORLD,
            Vec2::new(GATE_POST_X, bottom + GATE_POST_DROP),
            bottom + GATE_POST_SORT,
            still(SCENE_GATE_POST),
        );
    }
    true
}

fn draw_wall_and_entrance(list: &mut DrawList, lanes: &LaneState) {
    let grid_rect = Rect::from_min_size(
        Grid::ORIGIN,
        Vec2::new(
            Grid::CELL_SIZE.x * f32::from(Grid::COLS),
            Grid::CELL_SIZE.y * f32::from(Grid::ROWS),
        ),
    );
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
    for row in 0..Grid::ROWS {
        if lanes.is_active(row) && lanes.hose_available[usize::from(row)] {
            let y = Grid::row_center_y(row);
            list.rect(
                Layer::BACKGROUND,
                Rect::from_center_size(Vec2::new(Grid::HOSE_X, y), Vec2::new(30.0, 24.0)),
                color(palette::HOSE),
            );
        }
    }
}

/// 计时器驱动片段的已播放 tick（ART-SPEC 3.1，ADR-014）：`since` 为计时器本周期已经过的 tick，
/// `duration` 为周期。片段不短于周期时按进度映射（`since / duration × 片段长`，garden 皮肤的片段长 =
/// 规则时长，即逐 tick 对应）；更短的片段（`default` 皮肤的 18 tick 一次性 attack）从周期起点播放一次。
/// 已播放完时返回 `None`（调用方改用 idle）。
#[must_use]
pub fn timed_elapsed(since: u32, duration: u32, clip_ticks: u64) -> Option<u64> {
    let since = u64::from(since.min(duration.saturating_sub(1)));
    let elapsed = if clip_ticks >= u64::from(duration) {
        since * clip_ticks / u64::from(duration.max(1))
    } else {
        since
    };
    (elapsed < clip_ticks).then_some(elapsed)
}

/// 计时器本周期已经过的 tick。
fn timer_since(t: &Timer) -> u32 {
    t.duration.saturating_sub(t.remaining)
}

/// 防御者的模拟状态（只读）。
#[derive(Clone, Copy, Default)]
struct DefenderTimers<'a> {
    shooter: Option<&'a Shooter>,
    mine: Option<&'a Mine>,
    blast: Option<&'a Blast>,
    gust: Option<&'a Gust>,
    chomper: Option<&'a Chomper>,
    spikes: Option<&'a Spikes>,
    /// 格内是否有存活敌人（荆棘垫的 `trigger`，与模拟的 Spikes 判定相同：`Grid::enemy_overlaps_cols`）。
    occupied: bool,
}

/// 防御者当前应播放的片段与已播放 tick（SPEC 12.3，ART-SPEC 2.4 / 3.1）。计时类片段由对应计时器的
/// 进度驱动（与裁决（四）的啃食相同）：`attack` ← 射手冷却（就绪且无目标时回到 idle）、`arm` ← 武装计时、
/// `fuse` ← 引信、`trigger` ← 扎刺计时（格内有敌人时）、`snap` ← 咀嚼计时；生产者的 `produce` 是叠层，见
/// [`producer_overlay`]；块茎雷武装后 `armed`、咬夹草咀嚼时循环 `chew`（旧皮肤为 `attack`）、其余
/// idle 按游戏时钟。`hurt` 为真且皮肤有 `<id>_hurt_<片段>` 时用受损版本。
fn defender_pose_with(
    sk: &Painter<'_>,
    id: &str,
    e: Entity,
    tick: u64,
    state: &str,
    timers: DefenderTimers<'_>,
) -> (String, u64) {
    let name = |act: &str| {
        let damaged = format!("{id}_{state}_{act}");
        if !state.is_empty() && sk.has(&damaged) {
            damaged
        } else {
            format!("{id}_{act}")
        }
    };
    let timed = |act: &str, t: &Timer| {
        let clip = name(act);
        let len = sk.ticks(&clip)?;
        timed_elapsed(timer_since(t), t.duration, len).map(|el| (clip, el))
    };
    let pick = timers
        .shooter
        .filter(|s| !s.timer.finished())
        .and_then(|s| timed("attack", &s.timer))
        .or_else(|| {
            timers
                .mine
                .filter(|m| !m.armed)
                .and_then(|m| timed("arm", &m.arm))
        })
        .or_else(|| timers.blast.and_then(|b| timed("fuse", &b.fuse)))
        .or_else(|| timers.gust.and_then(|g| timed("fuse", &g.fuse)))
        .or_else(|| {
            timers
                .spikes
                .filter(|_| timers.occupied)
                .and_then(|sp| timed("trigger", &sp.timer))
        })
        .or_else(|| {
            // 咬夹草：吞食后咀嚼计时器的前 `snap` 长度内播放 snap（由吞食触发，ART-SPEC 3.1）
            timers
                .chomper
                .filter(|c| c.state == ChompState::Chewing)
                .and_then(|c| {
                    let clip = name("snap");
                    let len = sk.ticks(&clip)?;
                    let since = u64::from(timer_since(&c.chew));
                    (since < len).then_some((clip, since))
                })
        });
    if let Some(pose) = pick {
        return pose;
    }
    let looped = |act: &str| {
        let clip = name(act);
        sk.has(&clip).then(|| (clip, phase(e, tick)))
    };
    if timers
        .chomper
        .is_some_and(|c| c.state == ChompState::Chewing)
        && let Some(pose) = looped("chew").or_else(|| looped("attack"))
    {
        return pose;
    }
    if timers.mine.is_some_and(|m| m.armed)
        && let Some(pose) = looped("armed")
    {
        return pose;
    }
    (name("idle"), phase(e, tick))
}

/// 防御者 `e` 当前的皮肤片段名与已播放 tick（与绘制同一规则）；不在皮肤模式、或不是存活的地面防御者时为 `None`。
#[must_use]
pub fn defender_pose(world: &World, e: Entity) -> Option<(String, u64)> {
    let painter = Painter::active(world)?;
    let tick = world.resources.get::<Time>()?.fixed_tick;
    let mut query = world.entities.query_one::<(
        &DefenderRef,
        &Health,
        &Cell,
        Option<&Shooter>,
        Option<&Mine>,
        Option<&Blast>,
        Option<&Gust>,
        Option<&Chomper>,
        Option<&Spikes>,
    )>(e);
    let (d, h, cell, shooter, mine, blast, gust, chomper, spikes) = query.get().ok()?;
    let timers = DefenderTimers {
        shooter,
        mine,
        blast,
        gust,
        chomper,
        spikes,
        occupied: cell_occupied(&enemy_positions(world), *cell),
    };
    Some(defender_pose_with(
        &painter,
        d.id.as_str(),
        e,
        tick,
        defender_state(|c| painter.has(c), d.id.as_str(), h),
        timers,
    ))
}

/// 生产者的叠层（ADR-016 第 2 节）：身体播 `idle`（时钟），叠层播 `produce`（生产计时器进度，`pop` 在 0）；
/// 下一次生产是额外产量（每 `bonus_every` 次的那一次）时用 `produce_gold`（ART-SPEC 4.12）。皮肤没有叠层片段时为 `None`。
fn producer_overlay(sk: &Painter<'_>, id: &str, p: &Producer) -> Option<(String, u64)> {
    let bonus_next = p.bonus_every > 0 && (p.produced + 1).is_multiple_of(p.bonus_every);
    let gold = format!("{id}_produce_gold");
    let clip = if bonus_next && sk.has(&gold) {
        gold
    } else {
        format!("{id}_produce")
    };
    let len = sk.ticks(&clip)?;
    timed_elapsed(timer_since(&p.timer), p.timer.duration, len).map(|el| (clip, el))
}

/// 生产者 `e` 当前叠层的片段与已播放 tick（与绘制同一规则）；不在皮肤模式或没有叠层时为 `None`。
#[must_use]
pub fn producer_pose(world: &World, e: Entity) -> Option<(String, u64)> {
    let painter = Painter::active(world)?;
    let mut query = world.entities.query_one::<(&DefenderRef, &Producer)>(e);
    let (d, p) = query.get().ok()?;
    producer_overlay(&painter, d.id.as_str(), p)
}

/// 放置时的弹性缩放时长（ART-SPEC 4.13“1.2 → 1.0，10 tick”）。
const PLANT_POP_TICKS: u64 = 10;
/// 放置时的起始缩放。
const PLANT_POP_SCALE: f32 = 1.2;

/// 放置后 `age` tick 的缩放：1.2 → 1.0，带一次轻微回弹（纯视觉）。
#[must_use]
pub fn plant_scale(age: u64) -> f32 {
    if age >= PLANT_POP_TICKS {
        return 1.0;
    }
    let k = age as f32 / PLANT_POP_TICKS as f32;
    let wobble = (k * std::f32::consts::PI * 2.0).sin() * 0.04 * (1.0 - k);
    1.0 + (PLANT_POP_SCALE - 1.0) * (1.0 - k) * (1.0 - k) - wobble
}

/// Chill 霜晶的片段（ART-SPEC 7 `frost_crown`）。
const FROST_CROWN_CLIP: &str = "frost_crown_play";

/// 霜晶相对脚底的高度（px）：各 Husk 头顶的大致位置（ART-SPEC 2.1 画布与 5 章造型）。
fn crown_height(enemy: &str) -> f32 {
    match enemy {
        "drum_husk" => -146.0,
        "sproutling" => -60.0,
        "sprinter_husk" => -120.0,
        _ => -104.0,
    }
}

/// 受伤抖动的幅度（px，ART-SPEC 4.3）。
const HURT_SHAKE_PX: f32 = 2.0;

/// 受伤抖动：逐 tick 左右交替。
fn hurt_shake(tick: u64) -> f32 {
    if tick.is_multiple_of(2) {
        HURT_SHAKE_PX
    } else {
        -HURT_SHAKE_PX
    }
}

/// 存活敌人的 (车道, x)。
fn enemy_positions(world: &World) -> Vec<(u8, f32)> {
    world
        .query::<(
            &Transform2D,
            &Health,
            &rootguard_core::components::Lane,
            &EnemyRef,
        )>()
        .iter()
        .filter(|(_, h, _, _)| h.alive())
        .map(|(t, _, l, _)| (l.row, t.pos.x))
        .collect()
}

/// 格内是否有存活敌人（与模拟 Spikes 的判定相同，SPEC 7.2）。
fn cell_occupied(enemies: &[(u8, f32)], cell: Cell) -> bool {
    enemies
        .iter()
        .any(|&(row, x)| row == cell.row && Grid::enemy_overlaps_cols(x, cell.col, 0, 0))
}

/// 防御者是否处于受损阶段（ART-SPEC 2.4 / 4.1：如石籽荚 hp ≤ 3 / 6）。
fn defender_hurt(h: &Health) -> bool {
    h.hp * 2 <= h.max
}

/// 防御者的受损状态前缀（ART-SPEC 2.4）：皮肤有三级片段（`<id>_cracked_idle`，树皮墙 / 葫芦壳）时
/// hp > 2/3 完好、> 1/3 `cracked`、其余 `split`；否则两级，hp ≤ 1/2 为 `hurt`（石籽荚 4.1）。
fn defender_state(has: impl Fn(&str) -> bool, id: &str, h: &Health) -> &'static str {
    if has(&format!("{id}_cracked_idle")) {
        if h.hp * 3 > h.max * 2 {
            ""
        } else if h.hp * 3 > h.max {
            "cracked"
        } else {
            "split"
        }
    } else if defender_hurt(h) {
        "hurt"
    } else {
        ""
    }
}

/// 片段名是否带受损 / 护甲状态前缀。
fn has_state_prefix(clip: &str) -> bool {
    ["_hurt_", "_cracked_", "_split_"]
        .iter()
        .any(|p| clip.contains(p))
}

fn draw_defenders(
    world: &World,
    list: &mut DrawList,
    alpha: f32,
    tick: u64,
    painter: Option<&Painter<'_>>,
) {
    let hurt_at = world.resources.get::<RenderState>();
    let enemies = enemy_positions(world);
    // 地面防御者
    for (e, t, p, h, cell, d, shooter, producer, mine, blast, gust, chomper, spikes) in world
        .query::<(
            Entity,
            &Transform2D,
            &Placeholder,
            &Health,
            &Cell,
            &DefenderRef,
            Option<&Shooter>,
            Option<&Producer>,
            Option<&Mine>,
            Option<&Blast>,
            Option<&Gust>,
            Option<&Chomper>,
            Option<&Spikes>,
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
            let timers = DefenderTimers {
                shooter,
                mine,
                blast,
                gust,
                chomper,
                spikes,
                occupied: cell_occupied(&enemies, *cell),
            };
            let state = defender_state(|c| sk.has(c), d.id.as_str(), h);
            let (clip, elapsed) = defender_pose_with(sk, d.id.as_str(), e, tick, state, timers);
            // 有受损变体时由造型表达受损（ART-SPEC 2.4），否则沿用低生命着色
            let tint = if h.hp * 2 < h.max && !has_state_prefix(&clip) {
                color(palette::LOW_HP_TINT)
            } else {
                Color::WHITE
            };
            let flash = hurt_at
                .and_then(|s| s.hurt_at.get(&e))
                .is_some_and(|&at| tick.saturating_sub(at) < HURT_FLASH_TICKS);
            let pose = Pose {
                clip: &clip,
                elapsed,
                flash,
                tint,
            };
            // 受伤时左右抖 2 px（ART-SPEC 4.3 / 4.9“被啃时轻微抖动”），与闪白同时长
            let shake = if flash { hurt_shake(tick) } else { 0.0 };
            let at = skin::foot(pos) + Vec2::new(shake, 0.0);
            let born = hurt_at.and_then(|st| st.defender_born.get(&e.to_bits().get()).copied());
            let scale = born.map_or(1.0, |b| plant_scale(tick.saturating_sub(b)));
            if sk.draw_scaled(list, WORLD, at, y, pose, scale) {
                // 生产者叠层画在身体之上（同 sort_y，后提交）
                if let Some((clip, elapsed)) =
                    producer.and_then(|p| producer_overlay(sk, d.id.as_str(), p))
                {
                    let overlay = Pose {
                        clip: &clip,
                        elapsed,
                        flash,
                        tint,
                    };
                    sk.draw(list, WORLD, at, y, overlay);
                }
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
            // 葫芦壳 3 级受损（ART-SPEC 2.4 / 4.9）；被啃时闪白并随抖动（4.9“壳随 eat_hit 震动”）
            let id = d.id.as_str();
            let state = defender_state(|c| sk.has(c), id, h);
            let damaged = format!("{id}_{state}_idle");
            let clip = if !state.is_empty() && sk.has(&damaged) {
                damaged
            } else {
                format!("{id}_idle")
            };
            let tint = if h.hp * 2 < h.max && !has_state_prefix(&clip) {
                color(palette::LOW_HP_TINT)
            } else {
                Color::WHITE
            };
            let flash = hurt_at
                .and_then(|s| s.hurt_at.get(&e))
                .is_some_and(|&at| tick.saturating_sub(at) < HURT_FLASH_TICKS);
            let shake = if flash { hurt_shake(tick) } else { 0.0 };
            let pose = Pose {
                clip: &clip,
                elapsed: phase(e, tick),
                flash,
                tint,
            };
            if sk.draw(
                list,
                WORLD,
                skin::foot(pos) + Vec2::new(shake, 0.0),
                row_y(cell.row),
                pose,
            ) {
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

fn draw_enemies(
    world: &World,
    list: &mut DrawList,
    alpha: f32,
    tick: u64,
    painter: Option<&Painter<'_>>,
) {
    let hurt = world.resources.get::<RenderState>();
    let factor = chill_bite_factor(world);
    let mut query = world.query::<(
        Entity,
        &Transform2D,
        &Placeholder,
        &Health,
        &rootguard_core::components::Lane,
        &StatusEffects,
        Option<&Armor>,
        &EnemyRef,
        &Walker,
        Option<&Biter>,
        Option<&Vaulter>,
    )>();
    let mut enemies: Vec<_> = query.iter().collect();
    // 同一行内从右到左提交（sort_y 相同时按提交顺序叠放）：最靠左者画在最上面。它是弹丸最先命中的目标
    // （SPEC 2.5），闪白时整只白色剪影可见，而不是被身后的同伴盖住只露出白边（裁决（六）第 2 节）。
    enemies.sort_by(|a, b| {
        (a.4.row, b.1.pos.x, a.0.to_bits())
            .partial_cmp(&(b.4.row, a.1.pos.x, b.0.to_bits()))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (e, t, p, h, lane, status, armor, enemy, walker, biter, vaulter) in enemies {
        if !h.alive() {
            continue;
        }
        let y = row_y(lane.row);
        let pos = t.lerp(alpha);
        let flashing = hurt
            .and_then(|s| s.hurt_at.get(&e))
            .is_some_and(|&at| tick.saturating_sub(at) < HURT_FLASH_TICKS);
        if let Some(sk) = painter {
            let look = EnemyLook {
                id: enemy.id.as_str(),
                x: pos.x,
                walker,
                biter,
                status,
                armor,
                vaulter,
                def_speed: enemy_def_speed(world, enemy),
                age: enemy_age(world, e, tick),
            };
            let (clip, elapsed) = enemy_pose_with(|c| sk.ticks(c), &look, tick, e, factor);
            let tint = if status.chilled() {
                mix(Color::WHITE, color(palette::CHILL_TINT), CHILL_MIX)
            } else {
                Color::WHITE
            };
            let pose = Pose {
                clip: &clip,
                elapsed,
                flash: flashing,
                tint,
            };
            if sk.draw(list, WORLD, skin::foot(pos), y, pose) {
                // Chill：头顶 3 粒霜晶（ART-SPEC 3.3）
                if status.chilled() {
                    let crown = Pose {
                        clip: FROST_CROWN_CLIP,
                        elapsed: phase(e, tick),
                        flash: false,
                        tint: Color::WHITE,
                    };
                    let head = skin::foot(pos) + Vec2::new(0.0, crown_height(enemy.id.as_str()));
                    sk.draw(list, WORLD, head, y, crown);
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timed_elapsed_maps_progress_or_plays_short_clips_once() {
        // garden：片段长 = 周期，逐 tick 对应；片段更长时按进度缩放
        assert_eq!(timed_elapsed(0, 85, 85), Some(0));
        assert_eq!(timed_elapsed(84, 85, 85), Some(84));
        assert_eq!(timed_elapsed(720, 1440, 2880), Some(1440));
        // 计时器已到期（remaining = 0，since = duration）时停在末帧而不越界
        assert_eq!(timed_elapsed(900, 900, 900), Some(899));
        // default：18 tick 的一次性 attack 从周期起点播放一次，之后回到 idle
        assert_eq!(timed_elapsed(17, 85, 18), Some(17));
        assert_eq!(timed_elapsed(18, 85, 18), None);
    }

    #[test]
    fn eat_elapsed_follows_effective_interval() {
        assert_eq!(eat_elapsed(50, 60, 60), 50);
        assert_eq!(eat_elapsed(100, 120, 60), 50, "Chill 下间隔翻倍，相位减半");
        assert_eq!(eat_elapsed(200, 60, 60), 59, "超出间隔时钳在末 tick");
    }
}
