//! 皮肤渲染（SPEC 12.1 / 12.3，ART-SPEC 0）：`SkinMode::Skinned` 时以 `assets/skins/<皮肤>/` 的图集与动画片段
//! 绘制防御者、敌人、弹丸与 Dew。启动时加载全部皮肤（`default` / `garden`）；所选皮肤缺少某片段时
//! 依次回退到 `default` 皮肤，再回退占位渲染（SPEC 12.1“允许部分实体回退占位”，ADR-014）。
//!
//! 只在 Render 阶段读取模拟状态并直接提交 `DrawList::sprite`：不给模拟实体添加 `Sprite` / `Animator`
//! 组件，也不创建实体（ADR-007）。动画帧由引擎 `AnimationClip` 数据计算。

use std::collections::BTreeMap;

use rootguard_core::events::{SkinMode, SkinName};
use rootguard_core::resources::Progress;
use serde::Deserialize;
use trellia::gfx::AtlasFrame;
use trellia::prelude::*;

/// 皮肤清单文件名（`skins/<皮肤>/skin.ron`，由 `scripts/gen_skins.py` / `tools/artgen` 生成）。
const MANIFEST: &str = "skin.ron";

/// 地面角色的脚底相对 `Transform2D.pos`（格子中心）的下移量：帧锚点对齐到格子底部附近。
pub const FOOT_OFFSET_Y: f32 = 46.0;

/// 卡槽图标的边长（SPEC 11.2“皮肤图”）。
const ICON_SIZE: f32 = 40.0;

#[derive(Deserialize)]
struct AtlasEntry {
    atlas: String,
    clips: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename = "Skin")]
struct Manifest {
    atlases: Vec<AtlasEntry>,
}

/// 一套已加载的皮肤：片段名 → (所在图集, 片段)。
#[derive(Clone, Debug, Default)]
pub struct SkinAssets {
    clips: BTreeMap<String, (Handle<Atlas>, Handle<AnimationClip>)>,
}

impl SkinAssets {
    /// 按清单加载 `skins/<name>/` 的全部图集与片段（引擎格式，校验见 `load_atlas` / `load_clip`）。
    ///
    /// # Errors
    ///
    /// 清单、图集、图像或任一片段缺失 / 解析失败 / 校验失败时返回 [`AssetError`]。
    pub fn load(
        world: &mut World,
        server: &AssetServer,
        name: SkinName,
    ) -> Result<Self, AssetError> {
        let dir = format!("skins/{}", name.dir());
        let manifest: Manifest = server.load_ron(&format!("{dir}/{MANIFEST}"))?;
        let mut clips = BTreeMap::new();
        for entry in &manifest.atlases {
            let atlas = load_atlas_into(world, &entry.atlas)?;
            for clip in &entry.clips {
                let rel = format!("{dir}/anim/{clip}.anim.ron");
                clips.insert(clip.clone(), (atlas, load_clip_into(world, atlas, &rel)?));
            }
        }
        Ok(Self { clips })
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
        self.clips.get(name).map(|(_, c)| *c)
    }

    /// 片段所在图集的句柄。
    #[must_use]
    pub fn atlas_of(&self, name: &str) -> Option<Handle<Atlas>> {
        self.clips.get(name).map(|(a, _)| *a)
    }
}

/// 全部已加载的皮肤（资源）。
#[derive(Clone, Debug, Default)]
pub struct Skins {
    by_name: BTreeMap<SkinName, SkinAssets>,
}

impl Skins {
    /// 加载全部皮肤；单个皮肤失败只警告（SPEC 12.1 皮肤为尽力而为）。
    pub fn load_all(world: &mut World, server: &AssetServer) -> Self {
        let mut by_name = BTreeMap::new();
        for name in SkinName::ALL {
            match SkinAssets::load(world, server, name) {
                Ok(skin) => {
                    by_name.insert(name, skin);
                }
                Err(e) => log::warn!("皮肤 {} 加载失败，将回退：{e}", name.dir()),
            }
        }
        Self { by_name }
    }

    /// 某套皮肤（未加载时为 `None`）。
    #[must_use]
    pub fn get(&self, name: SkinName) -> Option<&SkinAssets> {
        self.by_name.get(&name)
    }

    /// 以 `name` 为首、`default` 为回退的查找视图。
    #[must_use]
    pub fn view(&self, name: SkinName) -> SkinView<'_> {
        let mut chain: Vec<&SkinAssets> = self.get(name).into_iter().collect();
        if name != SkinName::Default {
            chain.extend(self.get(SkinName::Default));
        }
        SkinView { chain }
    }
}

/// 按回退顺序查找片段的视图。
#[derive(Clone, Debug)]
pub struct SkinView<'w> {
    chain: Vec<&'w SkinAssets>,
}

impl<'w> SkinView<'w> {
    /// 设置中所选皮肤的视图，与渲染模式无关（动画事件与音效在占位模式下同样需要，ADR-010）。
    #[must_use]
    pub fn selected(world: &'w World) -> Option<Self> {
        let name = world
            .resources
            .get::<Progress>()
            .map_or(SkinName::default(), |p| p.settings.skin_name);
        let view = world.resources.get::<Skins>()?.view(name);
        (!view.chain.is_empty()).then_some(view)
    }

    /// 片段所在图集与片段句柄（按回退顺序第一个有该片段的皮肤）。
    #[must_use]
    pub fn entry(&self, name: &str) -> Option<(Handle<Atlas>, Handle<AnimationClip>)> {
        self.chain
            .iter()
            .find_map(|s| Some((s.atlas_of(name)?, s.clip(name)?)))
    }

    /// 片段是否存在。
    #[must_use]
    pub fn has_clip(&self, name: &str) -> bool {
        self.chain.iter().any(|s| s.has_clip(name))
    }

    /// 片段句柄。
    #[must_use]
    pub fn clip(&self, name: &str) -> Option<Handle<AnimationClip>> {
        self.entry(name).map(|(_, c)| c)
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
    view: SkinView<'w>,
    atlases: &'w Assets<Atlas>,
    clips: &'w Assets<AnimationClip>,
}

impl<'w> Painter<'w> {
    /// 当前设置为 `Skinned` 且皮肤已加载时返回绘制器（SPEC 12.1）。
    #[must_use]
    pub fn active(world: &'w World) -> Option<Self> {
        let skinned = world
            .resources
            .get::<Progress>()
            .is_some_and(|p| p.settings.skin == SkinMode::Skinned);
        if !skinned {
            return None;
        }
        let view = SkinView::selected(world)?;
        let atlases = world.resources.get::<Assets<Atlas>>()?;
        let clips = world.resources.get::<Assets<AnimationClip>>()?;
        Some(Self {
            view,
            atlases,
            clips,
        })
    }

    /// 片段是否存在。
    #[must_use]
    pub fn has(&self, clip: &str) -> bool {
        self.view.has_clip(clip)
    }

    /// 片段总时长（tick）。
    #[must_use]
    pub fn ticks(&self, clip: &str) -> Option<u64> {
        self.clips.get(self.view.clip(clip)?).map(clip_ticks)
    }

    fn frame(&self, pose: &Pose<'_>) -> Option<(Handle<Image>, &'w AtlasFrame)> {
        let (atlas, clip) = self.view.entry(pose.clip)?;
        let atlas = self.atlases.get(atlas)?;
        let clip = self.clips.get(clip)?;
        let index = clip
            .frames
            .get(frame_index_at(clip, pose.elapsed))?
            .atlas_frame;
        let frame = atlas.frames.get(index)?;
        if pose.flash
            && let Some(flash) = atlas.frame(&format!("{}_flash", frame.name))
        {
            return Some((atlas.image, atlas.frames.get(flash)?));
        }
        Some((atlas.image, frame))
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
        let Some((image, frame)) = self.frame(&pose) else {
            return false;
        };
        let size = frame.src_px.size();
        let dst = Rect::from_min_size(pos - frame.anchor * size, size);
        list.sprite(layer, sort_y, image, frame.src_px, dst)
            .tint(pose.tint);
        true
    }

    /// 按缩放绘制（以锚点为中心，ART-SPEC 4.13 放置的弹性缩放）；片段缺失时返回 false。
    pub fn draw_scaled(
        &self,
        list: &mut DrawList,
        layer: Layer,
        pos: Vec2,
        sort_y: f32,
        pose: Pose<'_>,
        scale: f32,
    ) -> bool {
        let Some((image, frame)) = self.frame(&pose) else {
            return false;
        };
        let size = frame.src_px.size() * scale;
        let dst = Rect::from_min_size(pos - frame.anchor * size, size);
        list.sprite(layer, sort_y, image, frame.src_px, dst)
            .tint(pose.tint);
        true
    }

    /// 按指定尺寸绘制（锚点相同），用于需要拉伸的特效（ART-SPEC 7）；片段缺失时返回 false。
    pub fn draw_sized(
        &self,
        list: &mut DrawList,
        layer: Layer,
        pos: Vec2,
        sort_y: f32,
        pose: Pose<'_>,
        size: Vec2,
    ) -> bool {
        let Some((image, frame)) = self.frame(&pose) else {
            return false;
        };
        let dst = Rect::from_min_size(pos - frame.anchor * size, size);
        list.sprite(layer, sort_y, image, frame.src_px, dst)
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
        let Some((image, frame)) = self.frame(&pose) else {
            return false;
        };
        let side = ICON_SIZE.min(rect.size().y + 8.0);
        let dst = Rect::from_center_size(rect.center(), Vec2::splat(side));
        list.sprite(layer, 0.0, image, frame.src_px, dst);
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
