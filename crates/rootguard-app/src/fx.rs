//! 特效实体（ART-SPEC 7）：呈现层根据模拟事件生成、寿命以 tick 计、不进入模拟层（ADR-016 第 5 节）。
//!
//! [`capture`] 在 `Stage::Update` 中先于 `SfxPlugin` 运行（`sfx_system` 会取走大部分事件），只读事件与实体位置，
//! 把要播放的特效记入 [`crate::render::RenderState`]；[`draw`] 在 Render 阶段按“当前 tick − 开始 tick”取帧绘制。
//! 皮肤缺少某特效片段时不画（占位模式没有特效，SPEC 12.2 不要求）。

use std::collections::BTreeSet;

use rootguard_core::components::{EnemyRef, Lane};
use rootguard_core::data::ArmorKind;
use rootguard_core::events::EnemyArmorBroken;
use trellia::prelude::*;

use crate::render::RenderState;
use crate::skin::{self, Painter, Pose};

/// 特效图层：角色之上、HUD 之下。
const FX_LAYER: Layer = Layer::FX;

/// 护甲破碎特效相对敌人脚底的高度（px）：Helm 在头顶、Shield 在胸前、Drum 在腹部（ART-SPEC 5.2 ～ 5.8）。
const HELM_Y: f32 = -112.0;
const SHIELD_Y: f32 = -54.0;
const DRUM_Y: f32 = -72.0;

/// 一个正在播放的特效。
#[derive(Clone, Debug, PartialEq)]
pub struct Fx {
    /// 皮肤片段名（`<特效 id>_play`）。
    pub clip: String,
    /// 中心位置（逻辑像素）。
    pub pos: Vec2,
    /// 开始的 `fixed_tick`。
    pub start: u64,
    /// 绘制尺寸；`None` 为帧原尺寸（ART-SPEC 7 的“缩放到 / 沿车道拉伸”用）。
    pub size: Option<Vec2>,
}

/// 特效片段名。
#[must_use]
pub fn clip_of(fx: &str) -> String {
    format!("{fx}_play")
}

/// 护甲破碎特效（ART-SPEC 7 `armor_break_*`）：按敌人与护甲种类。
#[must_use]
pub fn armor_break_fx(enemy: &str, kind: ArmorKind) -> (&'static str, f32) {
    match (enemy, kind) {
        ("clay_husk", _) => ("armor_break_pot", HELM_Y),
        (_, ArmorKind::Helm) => ("armor_break_twig", HELM_Y),
        (_, ArmorKind::Shield) => ("armor_break_plank", SHIELD_Y),
        (_, ArmorKind::Drum) => ("armor_break_gourd", DRUM_Y),
    }
}

/// `Update`（先于 `sfx_system`）：把本帧的模拟事件转成特效。只在对局中（有 `RenderState`）工作。
pub fn capture(world: &mut World) {
    if world.resources.get::<RenderState>().is_none() {
        return;
    }
    let tick = world.resources.expect::<Time>().fixed_tick;
    let mut spawned: Vec<(String, Fx)> = Vec::new();
    for e in world.events.get::<EnemyArmorBroken>().iter() {
        let mut q = world
            .entities
            .query_one::<(&EnemyRef, &Transform2D, &Lane)>(e.entity);
        let Ok((enemy, t, _)) = q.get() else {
            continue;
        };
        let (fx, dy) = armor_break_fx(enemy.id.as_str(), e.kind);
        spawned.push((
            format!("armor:{}", e.entity.to_bits().get()),
            Fx {
                clip: clip_of(fx),
                pos: skin::foot(t.pos) + Vec2::new(0.0, dy),
                start: tick,
                size: None,
            },
        ));
    }
    let Some(state) = world.resources.get_mut::<RenderState>() else {
        return;
    };
    // 未被 sfx_system 取走的事件在下一帧仍可读：上一帧已生成过的键不再生成
    let keys: BTreeSet<String> = spawned.iter().map(|(k, _)| k.clone()).collect();
    for (key, fx) in spawned {
        if !state.fx_keys.contains(&key) {
            state.fx.push(fx);
        }
    }
    state.fx_keys = keys;
}

/// 删除已播放完的特效（`Update`，片段长度取所选皮肤；皮肤没有该片段时立即删除）。
pub fn prune(world: &mut World) {
    let tick = world.resources.expect::<Time>().fixed_tick;
    let lens: Vec<Option<u64>> = {
        let Some(state) = world.resources.get::<RenderState>() else {
            return;
        };
        let view = skin::SkinView::selected(world);
        let clips = world.resources.get::<Assets<AnimationClip>>();
        state
            .fx
            .iter()
            .map(|f| {
                let h = view.as_ref()?.clip(&f.clip)?;
                clips?.get(h).map(skin::clip_ticks)
            })
            .collect()
    };
    if let Some(state) = world.resources.get_mut::<RenderState>() {
        let mut i = 0;
        state.fx.retain(|f| {
            let keep = lens[i].is_some_and(|len| tick.saturating_sub(f.start) < len);
            i += 1;
            keep
        });
    }
}

/// Render：绘制全部特效。
pub fn draw(world: &World, list: &mut DrawList, painter: Option<&Painter<'_>>, tick: u64) {
    let (Some(sk), Some(state)) = (painter, world.resources.get::<RenderState>()) else {
        return;
    };
    for f in &state.fx {
        let pose = Pose {
            clip: &f.clip,
            elapsed: tick.saturating_sub(f.start),
            flash: false,
            tint: Color::WHITE,
        };
        match f.size {
            Some(size) => sk.draw_sized(list, FX_LAYER, f.pos, f.pos.y, pose, size),
            None => sk.draw(list, FX_LAYER, f.pos, f.pos.y, pose),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armor_break_fx_by_enemy() {
        assert_eq!(
            armor_break_fx("twig_husk", ArmorKind::Helm).0,
            "armor_break_twig"
        );
        assert_eq!(
            armor_break_fx("clay_husk", ArmorKind::Helm).0,
            "armor_break_pot"
        );
        assert_eq!(
            armor_break_fx("shield_husk", ArmorKind::Shield).0,
            "armor_break_plank"
        );
        assert_eq!(
            armor_break_fx("drum_husk", ArmorKind::Drum).0,
            "armor_break_gourd"
        );
    }
}
