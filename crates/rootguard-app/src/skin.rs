//! 皮肤渲染（SPEC 12.1 / 12.3）：`SkinMode::Skinned` 时以 `assets/skins/default/` 的图集与动画片段
//! 绘制防御者、敌人、弹丸与 Dew；缺少片段的实体回退占位渲染（SPEC 12.1“允许部分实体回退占位”）。
//!
//! 只在 Render 阶段读取模拟状态并直接提交 `DrawList::sprite`：不给模拟实体添加 `Sprite` / `Animator`
//! 组件，也不创建实体（ADR-007）。动画帧由引擎 `AnimationClip` 数据按 `fixed_tick` 计算。

use std::collections::BTreeMap;

use serde::Deserialize;
use trellia::gfx::AtlasFrame;
use trellia::prelude::*;

/// 皮肤清单（由 `scripts/gen_skins.py` 生成）。
pub const MANIFEST: &str = "skins/default/skin.ron";

/// 动画片段目录。
const ANIM_DIR: &str = "skins/default/anim";

/// 地面角色的脚底相对 `Transform2D.pos`（格子中心）的下移量：96 px 高的帧在格内垂直居中。
pub const FOOT_OFFSET_Y: f32 = 46.0;

/// 卡槽图标的边长（SPEC 11.2“皮肤图”）。
const ICON_SIZE: f32 = 40.0;

#[derive(Deserialize)]
#[serde(rename = "Skin")]
struct Manifest {
    atlas: String,
    clips: Vec<String>,
}

/// 已加载的皮肤（资源）：图集与片段名 → 句柄。
#[derive(Clone, Debug)]
pub struct SkinAssets {
    atlas: Handle<Atlas>,
    clips: BTreeMap<String, Handle<AnimationClip>>,
}

impl SkinAssets {
    /// 按清单加载图集与全部片段（引擎格式，校验见 `load_atlas` / `load_clip`）。
    ///
    /// # Errors
    ///
    /// 清单、图集、图像或任一片段缺失 / 解析失败 / 校验失败时返回 [`AssetError`]。
    pub fn load(world: &mut World, server: &AssetServer) -> Result<Self, AssetError> {
        let manifest: Manifest = server.load_ron(MANIFEST)?;
        let atlas = load_atlas_into(world, &manifest.atlas)?;
        let mut clips = BTreeMap::new();
        for name in &manifest.clips {
            let rel = format!("{ANIM_DIR}/{name}.anim.ron");
            clips.insert(name.clone(), load_clip_into(world, atlas, &rel)?);
        }
        Ok(Self { atlas, clips })
    }

    /// 片段名是否存在。
    #[must_use]
    pub fn has_clip(&self, name: &str) -> bool {
        self.clips.contains_key(name)
    }

    /// 全部片段名（字典序）。
    pub fn clip_names(&self) -> impl Iterator<Item = &str> {
        self.clips.keys().map(String::as_str)
    }

    /// 片段句柄。
    #[must_use]
    pub fn clip(&self, name: &str) -> Option<Handle<AnimationClip>> {
        self.clips.get(name).copied()
    }

    /// 图集句柄。
    #[must_use]
    pub fn atlas(&self) -> Handle<Atlas> {
        self.atlas
    }
}

/// 片段播放 `elapsed` tick 时的帧序号（循环片段取模；非循环片段停在最后一帧）。
#[must_use]
pub fn frame_index_at(clip: &AnimationClip, elapsed: u64) -> usize {
    let total: u64 = clip
        .frames
        .iter()
        .map(|f| u64::from(f.duration_ticks.max(1)))
        .sum();
    if total == 0 {
        return 0;
    }
    let mut t = if clip.looping {
        elapsed % total
    } else {
        elapsed
    };
    for (i, f) in clip.frames.iter().enumerate() {
        let d = u64::from(f.duration_ticks.max(1));
        if t < d {
            return i;
        }
        t -= d;
    }
    clip.frames.len().saturating_sub(1)
}

/// 片段总时长（tick）。
#[must_use]
pub fn clip_ticks(clip: &AnimationClip) -> u64 {
    clip.frames
        .iter()
        .map(|f| u64::from(f.duration_ticks.max(1)))
        .sum()
}

/// 片段从已播放 `from` tick 推进到 `to` tick 的过程中进入的带事件帧（`from` 不含、`to` 含），
/// 语义同引擎 `AnimationEvent`“进入带事件的帧”（ADR-010）。
pub fn events_between(clip: &AnimationClip, from: u64, to: u64) -> impl Iterator<Item = &str> {
    (from.saturating_add(1)..=to).filter_map(move |t| {
        let i = frame_index_at(clip, t);
        if i == frame_index_at(clip, t - 1) {
            return None;
        }
        clip.frames.get(i)?.event.as_deref()
    })
}

/// 一次精灵绘制的参数。
#[derive(Clone, Copy, Debug)]
pub struct Pose<'n> {
    /// 片段名。
    pub clip: &'n str,
    /// 片段已播放的 tick。
    pub elapsed: u64,
    /// 是否使用受伤闪白帧（`<帧名>_flash`，缺失时用原帧）。
    pub flash: bool,
    /// 着色。
    pub tint: Color,
}

/// Render 阶段的皮肤绘制器：只读借用图集与片段。
pub struct Painter<'w> {
    skin: &'w SkinAssets,
    atlas: &'w Atlas,
    clips: &'w Assets<AnimationClip>,
}

impl<'w> Painter<'w> {
    /// 当前设置为 `Skinned` 且皮肤已加载时返回绘制器（SPEC 12.1）。
    #[must_use]
    pub fn active(world: &'w World) -> Option<Self> {
        let skinned = world
            .resources
            .get::<rootguard_core::resources::Progress>()
            .is_some_and(|p| p.settings.skin == rootguard_core::events::SkinMode::Skinned);
        if !skinned {
            return None;
        }
        let skin = world.resources.get::<SkinAssets>()?;
        let atlas = world.resources.get::<Assets<Atlas>>()?.get(skin.atlas)?;
        let clips = world.resources.get::<Assets<AnimationClip>>()?;
        Some(Self { skin, atlas, clips })
    }

    /// 片段是否存在。
    #[must_use]
    pub fn has(&self, clip: &str) -> bool {
        self.skin.has_clip(clip)
    }

    /// 片段总时长（tick）。
    #[must_use]
    pub fn ticks(&self, clip: &str) -> Option<u64> {
        let handle = self.skin.clip(clip)?;
        self.clips.get(handle).map(clip_ticks)
    }

    fn frame(&self, pose: &Pose<'_>) -> Option<&'w AtlasFrame> {
        let clip = self.clips.get(self.skin.clip(pose.clip)?)?;
        let index = clip
            .frames
            .get(frame_index_at(clip, pose.elapsed))?
            .atlas_frame;
        let frame = self.atlas.frames.get(index)?;
        if pose.flash
            && let Some(flash) = self.atlas.frame(&format!("{}_flash", frame.name))
        {
            return self.atlas.frames.get(flash);
        }
        Some(frame)
    }

    /// 以帧锚点对齐 `pos` 绘制；片段缺失时返回 false（调用方回退占位渲染）。
    pub fn draw(
        &self,
        list: &mut DrawList,
        layer: Layer,
        pos: Vec2,
        sort_y: f32,
        pose: Pose<'_>,
    ) -> bool {
        let Some(frame) = self.frame(&pose) else {
            return false;
        };
        let size = frame.src_px.size();
        let dst = Rect::from_min_size(pos - frame.anchor * size, size);
        list.sprite(layer, sort_y, self.atlas.image, frame.src_px, dst)
            .tint(pose.tint);
        true
    }

    /// 在 `rect` 中央绘制片段第 0 帧的缩小图标（卡槽，SPEC 11.2）；缺失时返回 false。
    pub fn icon(&self, list: &mut DrawList, layer: Layer, rect: Rect, clip: &str) -> bool {
        let pose = Pose {
            clip,
            elapsed: 0,
            flash: false,
            tint: Color::WHITE,
        };
        let Some(frame) = self.frame(&pose) else {
            return false;
        };
        let side = ICON_SIZE.min(rect.size().y + 8.0);
        let dst = Rect::from_center_size(rect.center(), Vec2::splat(side));
        list.sprite(layer, 0.0, self.atlas.image, frame.src_px, dst);
        true
    }
}

/// 地面角色的绘制锚点（脚底）。
#[must_use]
pub fn foot(pos: Vec2) -> Vec2 {
    pos + Vec2::new(0.0, FOOT_OFFSET_Y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(durations: &[u32], looping: bool) -> AnimationClip {
        AnimationClip {
            name: "t".into(),
            frames: durations
                .iter()
                .enumerate()
                .map(|(i, &d)| trellia::gfx::AnimationFrame {
                    atlas_frame: i,
                    duration_ticks: d,
                    event: None,
                })
                .collect(),
            looping,
        }
    }

    #[test]
    fn frame_index_loops_and_clamps() {
        let looping = clip(&[10, 10, 10], true);
        assert_eq!(frame_index_at(&looping, 0), 0);
        assert_eq!(frame_index_at(&looping, 9), 0);
        assert_eq!(frame_index_at(&looping, 10), 1);
        assert_eq!(frame_index_at(&looping, 29), 2);
        assert_eq!(frame_index_at(&looping, 30), 0);
        let once = clip(&[6, 6, 6], false);
        assert_eq!(frame_index_at(&once, 13), 2);
        assert_eq!(frame_index_at(&once, 1000), 2);
        assert_eq!(clip_ticks(&once), 18);
    }

    #[test]
    fn events_between_reports_entered_event_frames() {
        let mut eat = clip(&[10, 10, 10], true);
        eat.frames[1].event = Some("eat_hit".into());
        assert_eq!(events_between(&eat, 5, 9).count(), 0);
        assert_eq!(events_between(&eat, 9, 10).collect::<Vec<_>>(), ["eat_hit"]);
        assert_eq!(
            events_between(&eat, 10, 19).count(),
            0,
            "停留在事件帧内不重复"
        );
        assert_eq!(events_between(&eat, 0, 70).count(), 3, "每个循环一次");
    }
}
