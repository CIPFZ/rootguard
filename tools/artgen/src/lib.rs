//! Rootguard 美术管线 `artgen`（ART-SPEC 9，ADR-013）。
//!
//! 输入 `art/`：`palette.ron`（1.3）与 `characters/<id>/{parts/*.svg, rig.ron, clips.ron}`；
//! 输出 `assets/skins/garden/`：图集 PNG、引擎 `*.atlas.ron`、`anim/*.anim.ron` 与皮肤清单 `skin.ron`。
//!
//! 流程：校验调色板与部件 → 按 rig 层级求每帧位姿 → 合成 SVG → resvg 渲染 → 闪白帧 → 去重打包 →
//! 输出 RON。同一输入逐字节同一输出（只用有序容器、固定排布、固定 PNG 编码），
//! `art_assets_up_to_date` 在测试中重新生成并与提交的资产比对。

#![forbid(unsafe_code)]

pub mod atlas;
pub mod bg;
pub mod compose;
pub mod palette;
pub mod review;
pub mod rig;
pub mod rules;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::atlas::{ClipOut, FrameIn};
use crate::compose::{Image, Parts};
use crate::palette::Palette;
use crate::rig::{ClipDef, Clips, Duration, Rig, frame_ticks};
use crate::rules::Rules;

/// 输出皮肤目录（相对仓库根）。
pub const OUT_DIR: &str = "assets/skins/garden";

/// 输出路径在引擎资产根（`assets/`）下的前缀。
const ASSET_PREFIX: &str = "skins/garden";

/// 战场底图（ART-SPEC 8.1）：图像文件、图集名与片段名。
pub const BG_IMAGE: &str = "bg_base.png";
const BG_ATLAS: &str = "garden_bg";
/// 底图片段名（游戏侧按此名查找，ADR-015）。
pub const BG_CLIP: &str = "bg_base";
/// 可读性报告（随构建生成）。
pub const READABILITY_REPORT: &str = "docs/art/bg-readability.md";

/// 卡牌头像尺寸（ART-SPEC 2.1）。
pub const PORTRAIT: (u32, u32) = (80, 100);
/// 卡牌头像所在图集（ART-SPEC 2.1）。
const UI_ATLAS: &str = "garden_ui";
/// “不可用”头像的去饱和比例（ART-SPEC 4.13）。
const PORTRAIT_OFF_DESAT: f32 = 0.85;

/// 闪白帧的混白比例（ART-SPEC v1.3 2.3：填充色向白混合 70%，描边保持原色）。
pub const FLASH_WHITE_MIX: f64 = 0.7;

/// 尺寸校验的 alpha 阈值：抗锯齿边缘的半透明像素不计入外接框。
const SIZE_ALPHA: u8 = 64;

/// `eat` 片段的啃食事件（ART-SPEC 3.1 / 3.2）。
pub const EAT_HIT: &str = "eat_hit";
/// `attack` 片段的发射事件（ART-SPEC 3.1 / 3.2）。
pub const FIRE: &str = "fire";

/// 一个角色的全部输入。
pub struct Character {
    /// id（目录名，等于防御者 / 敌人 / 弹丸 id）。
    pub id: String,
    /// rig。
    pub rig: Rig,
    /// 片段。
    pub clips: Vec<ClipDef>,
    /// 部件源码。
    pub parts: Parts,
}

/// 读取 `art/characters/<id>/`。
///
/// # Errors
///
/// 文件缺失、解析失败或部件不合规时返回错误。
pub fn load_character(dir: &Path, palette: &Palette) -> Result<Character> {
    let id = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .context("角色目录名")?;
    let read = |name: &str| {
        let path = dir.join(name);
        std::fs::read_to_string(&path).with_context(|| format!("读取 {}", path.display()))
    };
    let rig: Rig = ron::from_str(&read("rig.ron")?).with_context(|| format!("{id}/rig.ron"))?;
    rig.check().with_context(|| format!("{id}/rig.ron"))?;
    let Clips(clips) =
        ron::from_str(&read("clips.ron")?).with_context(|| format!("{id}/clips.ron"))?;
    let parts = Parts::load(&dir.join("parts"), &id, &rig, palette)?;
    Ok(Character {
        id,
        rig,
        clips,
        parts,
    })
}

/// 片段时长：解析 `Rule(...)` 绑定，并检查 ART-SPEC 3.1 要求绑定的片段确实绑定了正确的规则。
///
/// # Errors
///
/// 规则无法解析、或要求绑定的片段写了固定时长 / 绑定了别的规则时返回错误。
pub fn clip_duration(rules: &Rules, id: &str, clip: &ClipDef) -> Result<u32> {
    let required = rules.required_binding(id, &clip.name);
    match (&clip.duration, required) {
        (Duration::Rule(path), Some(req)) if *path != req => {
            bail!(
                "{id}.{}：应绑定 Rule(\"{req}\")（ART-SPEC 3.1），实际 {path}",
                clip.name
            )
        }
        (Duration::Ticks(n), Some(req)) => {
            bail!(
                "{id}.{}：时长必须绑定 Rule(\"{req}\")（ART-SPEC 3.1），不能写固定的 {n}",
                clip.name
            )
        }
        (Duration::Rule(path), _) => rules.value(path),
        (Duration::Ticks(n), None) => Ok(*n),
    }
}

/// 检查片段的事件帧：tick 在片段内且是某帧的起点；`eat` 恰有一个 `eat_hit` 且在末帧；
/// `attack` 的 `fire` 数等于连发数（ART-SPEC 9.2）。
///
/// # Errors
///
/// 不满足时返回错误。
pub fn check_events(rules: &Rules, id: &str, clip: &ClipDef, ticks: &[u32]) -> Result<()> {
    let starts: Vec<u32> = ticks
        .iter()
        .scan(0, |acc, t| {
            let s = *acc;
            *acc += t;
            Some(s)
        })
        .collect();
    for (t, name) in &clip.events {
        if !starts.contains(t) {
            bail!(
                "{id}.{}：事件 {name} 的 tick {t} 不是任何帧的起点 {starts:?}",
                clip.name
            );
        }
    }
    let count = |n: &str| clip.events.iter().filter(|(_, e)| e == n).count();
    if clip.name == "eat" {
        let last = starts.last().copied().unwrap_or_default();
        if count(EAT_HIT) != 1 || !clip.events.iter().any(|(t, e)| e == EAT_HIT && *t == last) {
            bail!("{id}.eat：必须恰有一个 {EAT_HIT} 且在末帧（tick {last}，ART-SPEC 3.1）");
        }
    }
    if clip.name == "attack" {
        let volley = rules.volley(id).unwrap_or(1) as usize;
        // 第 k 枚弹丸在计时器重置后 k × volley_gap_ticks 生成（SPEC 7.2 Shooter），fire 帧须同 tick（ADR-014）
        let gap = rules
            .value(&format!("defenders.{id}.shooter.volley_gap_ticks"))
            .unwrap_or(0);
        let mut fires: Vec<u32> = clip
            .events
            .iter()
            .filter(|(_, e)| e == FIRE)
            .map(|(t, _)| *t)
            .collect();
        fires.sort_unstable();
        let expected: Vec<u32> = (0..volley as u32).map(|k| k * gap).collect();
        if count(FIRE) == volley && fires != expected {
            bail!(
                "{id}.attack：fire 应在 tick {expected:?}（k × volley_gap_ticks），实际 {fires:?}"
            );
        }
        if count(FIRE) != volley {
            bail!(
                "{id}.attack：{FIRE} 事件 {} 个，应等于连发数 {volley}（ART-SPEC 9.2）",
                count(FIRE)
            );
        }
    }
    Ok(())
}

/// 构建统计（写入 `docs/progress.md` 的资产数量）。
#[derive(Clone, Debug, Default)]
pub struct Summary {
    /// 角色数。
    pub characters: usize,
    /// 部件文件数（含变体）。
    pub parts: usize,
    /// 输出片段数（含受损状态）。
    pub clips: usize,
    /// 帧名数（含闪白）。
    pub frames: usize,
    /// 去重后的图像数。
    pub unique: usize,
    /// 图集：(名, 宽, 高)。
    pub atlases: Vec<(String, u32, u32)>,
    /// 主色未达 ART-SPEC 8.1 明度差的角色数（只报告，ADR-015）。
    pub readability_failing: usize,
}

/// 构建结果：相对仓库根的路径 → 文件内容。
pub struct Built {
    /// 输出文件。
    pub files: BTreeMap<String, Vec<u8>>,
    /// 统计。
    pub summary: Summary,
}

/// 角色与特效目录（`art/characters/`、`art/fx/` 下的目录，按 id 字典序）。
///
/// # Errors
///
/// 目录不可读时返回错误。
pub fn character_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    // 角色与特效（ART-SPEC 9.1：art/characters/<id>/、art/fx/<id>/，结构相同）
    for sub in ["art/characters", "art/fx"] {
        let dir = root.join(sub);
        if !dir.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&dir).with_context(|| format!("读取 {}", dir.display()))? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
            }
        }
    }
    dirs.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(dirs)
}

/// 渲染一个角色某状态某片段的全部帧图像（片段帧序）。
///
/// # Errors
///
/// 合成或渲染失败时返回错误。
pub fn render_clip(
    ch: &Character,
    palette: &Palette,
    clip: &ClipDef,
    state: &rig::StateDef,
    ticks: &[u32],
) -> Result<Vec<Image>> {
    let mut start = 0;
    let mut out = Vec::with_capacity(ticks.len());
    for t in ticks {
        let pose = ch.rig.pose(clip, state, start);
        out.push(
            ch.parts
                .render(&pose, palette)
                .with_context(|| format!("{}.{} tick {start}", ch.id, clip.name))?,
        );
        start += t;
    }
    Ok(out)
}

/// 角色默认位姿（rig `rest` 片段第 0 帧、基础状态）。
///
/// # Errors
///
/// `rest` 片段不存在或渲染失败时返回错误。
pub fn render_rest(ch: &Character, palette: &Palette) -> Result<Image> {
    let clip = ch
        .clips
        .iter()
        .find(|c| c.name == ch.rig.rest)
        .with_context(|| format!("{}：rest 片段 {} 不存在", ch.id, ch.rig.rest))?;
    let base = rig::StateDef {
        name: String::new(),
        set: BTreeMap::new(),
    };
    let pose = ch.rig.pose(clip, &base, 0);
    ch.parts.render(&pose, palette)
}

/// 卡牌头像（ART-SPEC 4.13）：默认位姿的胸像，以及去饱和的“不可用”版。取景窗口取 rig 的 `portrait`，
/// 缺省时以默认位姿可见范围为准：宽 = 可见宽 + 8，顶部留 4 px，水平居中。
///
/// # Errors
///
/// `rest` 片段不存在或渲染失败时返回错误。
pub fn render_portrait(ch: &Character, palette: &Palette) -> Result<(Image, Image)> {
    let clip = ch
        .clips
        .iter()
        .find(|c| c.name == ch.rig.rest)
        .with_context(|| format!("{}：rest 片段 {} 不存在", ch.id, ch.rig.rest))?;
    let base = rig::StateDef {
        name: String::new(),
        set: BTreeMap::new(),
    };
    let pose = ch.rig.pose(clip, &base, 0);
    let window = match ch.rig.portrait {
        Some(w) => w,
        None => {
            let rest = ch.parts.render(&pose, palette)?;
            let (x0, y0, x1, _) = rest
                .opaque_bounds(SIZE_ALPHA)
                .unwrap_or((0, 0, rest.w - 1, 0));
            let w = (x1 - x0 + 1) as f32 + 8.0;
            ((x0 + x1) as f32 / 2.0 - w / 2.0, y0 as f32 - 4.0, w)
        }
    };
    let img = ch.parts.render_window(&pose, palette, window, PORTRAIT)?;
    let off = img.desaturate(PORTRAIT_OFF_DESAT);
    Ok((img, off))
}

fn check_keys(ch: &Character, clip: &ClipDef) -> Result<()> {
    for part in clip.keys.keys() {
        if ch.rig.part(part).is_none() {
            bail!("{}.{}：关键帧引用了不存在的部件 {part}", ch.id, clip.name);
        }
    }
    for (part, swaps) in &clip.swaps {
        for (_, v) in swaps {
            ch.rig
                .check_variant(part, v)
                .with_context(|| format!("{}.{}", ch.id, clip.name))?;
        }
    }
    for (part, keys) in &clip.keys {
        if keys.windows(2).any(|w| w[0].t >= w[1].t) {
            bail!(
                "{}.{}：部件 {part} 的关键帧 tick 必须严格递增",
                ch.id,
                clip.name
            );
        }
    }
    Ok(())
}

/// 从仓库根 `root` 构建全部输出（不写盘）。
///
/// # Errors
///
/// 任一校验失败（ART-SPEC 9.2）时返回错误，不产生部分输出。
pub fn build(root: &Path) -> Result<Built> {
    let palette = Palette::load(&root.join("art/palette.ron"))?;
    let rules = Rules::load(&root.join("assets"))?;
    let mut summary = Summary::default();
    // 图集名 → 帧；图集名 → 片段
    let mut atlas_frames: BTreeMap<&'static str, Vec<FrameIn>> = BTreeMap::new();
    let mut mains: Vec<(String, palette::Rgb)> = Vec::new();
    let mut atlas_clips: BTreeMap<&'static str, Vec<ClipOut>> = BTreeMap::new();
    for dir in character_dirs(root)? {
        let ch = load_character(&dir, &palette)?;
        summary.characters += 1;
        summary.parts += ch
            .rig
            .parts
            .iter()
            .map(|p| 1 + p.variants.len())
            .sum::<usize>();
        let kind = ch.rig.kind;
        let rest = render_rest(&ch, &palette)?;
        let outlines = ["outline", "husk_outline"]
            .iter()
            .filter_map(|n| palette.resolve(&ch.id, kind.material(), n))
            .collect::<Vec<_>>();
        if let Some(c) = rest.dominant_color(&outlines) {
            mains.push((ch.id.clone(), c));
        }
        let (lw, lh) = kind.size_limit();
        if let Some((x0, y0, x1, y1)) = rest.opaque_bounds(SIZE_ALPHA) {
            let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
            if w > lw || h > lh {
                bail!(
                    "{}：默认位姿可见范围 {w} × {h} 超过 {lw} × {lh}（ART-SPEC 1.4）",
                    ch.id
                );
            }
        }
        if kind.has_portrait() {
            let (on, off) = render_portrait(&ch, &palette)?;
            for (suffix, image) in [("portrait", on), ("portrait_off", off)] {
                let name = format!("{}_{suffix}", ch.id);
                atlas_frames.entry(UI_ATLAS).or_default().push(FrameIn {
                    name: format!("{name}_0"),
                    image,
                    anchor: (0.5, 0.5),
                });
                atlas_clips.entry(UI_ATLAS).or_default().push(ClipOut {
                    frames: vec![(format!("{name}_0"), 60, None)],
                    name,
                    looping: false,
                });
            }
        }
        for clip in &ch.clips {
            check_keys(&ch, clip)?;
            let duration = clip_duration(&rules, &ch.id, clip)?;
            let ticks = frame_ticks(&clip.frames, duration)
                .with_context(|| format!("{}.{}", ch.id, clip.name))?;
            check_events(&rules, &ch.id, clip, &ticks)?;
            let mut starts = Vec::new();
            let mut acc = 0;
            for t in &ticks {
                starts.push(acc);
                acc += t;
            }
            for state in ch.rig.output_states() {
                let name = if state.name.is_empty() {
                    format!("{}_{}", ch.id, clip.name)
                } else {
                    format!("{}_{}_{}", ch.id, state.name, clip.name)
                };
                let images = render_clip(&ch, &palette, clip, &state, &ticks)?;
                let mut frames = Vec::new();
                for (i, image) in images.into_iter().enumerate() {
                    let frame = format!("{name}_{i}");
                    let event = clip
                        .events
                        .iter()
                        .find(|(t, _)| *t == starts[i])
                        .map(|(_, e)| e.clone());
                    frames.push((frame.clone(), ticks[i], event));
                    let list = atlas_frames.entry(kind.atlas()).or_default();
                    if ch.rig.flash {
                        let pose = ch.rig.pose(clip, &state, starts[i]);
                        list.push(FrameIn {
                            name: format!("{frame}_flash"),
                            image: ch.parts.render_flash(&pose, &palette, FLASH_WHITE_MIX)?,
                            anchor: kind.anchor(),
                        });
                    }
                    list.push(FrameIn {
                        name: frame,
                        image,
                        anchor: kind.anchor(),
                    });
                }
                atlas_clips.entry(kind.atlas()).or_default().push(ClipOut {
                    name,
                    looping: clip.looping,
                    frames,
                });
            }
        }
    }
    let mut files = BTreeMap::new();
    let mut manifest = Vec::new();
    for (atlas, frames) in &atlas_frames {
        let packed = atlas::pack(atlas, frames)?;
        let image_rel = format!("{ASSET_PREFIX}/{atlas}.png");
        files.insert(format!("{OUT_DIR}/{atlas}.png"), packed.image.encode_png()?);
        files.insert(
            format!("{OUT_DIR}/{atlas}.atlas.ron"),
            atlas::atlas_ron(&image_rel, &packed).into_bytes(),
        );
        summary.frames += packed.frames.len();
        summary.unique += packed.unique;
        summary
            .atlases
            .push(((*atlas).to_owned(), packed.image.w, packed.image.h));
        let clips = atlas_clips.remove(atlas).unwrap_or_default();
        for c in &clips {
            files.insert(
                format!("{OUT_DIR}/anim/{}.anim.ron", c.name),
                atlas::clip_ron(c).into_bytes(),
            );
        }
        summary.clips += clips.len();
        manifest.push((
            format!("{ASSET_PREFIX}/{atlas}.atlas.ron"),
            clips.into_iter().map(|c| c.name).collect(),
        ));
    }
    if let Some(image) = bg::render(root, &palette)? {
        files.insert(format!("{OUT_DIR}/{BG_IMAGE}"), image.encode_png()?);
        let packed = atlas::Packed {
            frames: vec![(format!("{BG_CLIP}_0"), 0, 0, image.w, image.h, (0.0, 0.0))],
            unique: 1,
            image,
        };
        files.insert(
            format!("{OUT_DIR}/{BG_ATLAS}.atlas.ron"),
            atlas::atlas_ron(&format!("{ASSET_PREFIX}/{BG_IMAGE}"), &packed).into_bytes(),
        );
        let clip = ClipOut {
            name: BG_CLIP.to_owned(),
            looping: false,
            frames: vec![(format!("{BG_CLIP}_0"), 60, None)],
        };
        files.insert(
            format!("{OUT_DIR}/anim/{BG_CLIP}.anim.ron"),
            atlas::clip_ron(&clip).into_bytes(),
        );
        manifest.push((
            format!("{ASSET_PREFIX}/{BG_ATLAS}.atlas.ron"),
            vec![BG_CLIP.to_owned()],
        ));
        summary.clips += 1;
        summary
            .atlases
            .push((BG_ATLAS.to_owned(), packed.image.w, packed.image.h));
        // ART-SPEC 8.1 可读性（ADR-015）：描边与格子底色的明度差为强制项，主色只报告
        let cells = bg::all_cell_colors(&palette)?;
        let outlines: Vec<(String, palette::Rgb, f64)> = ["outline", "husk_outline"]
            .iter()
            .filter_map(|n| {
                palette
                    .resolve(palette::BACKGROUND, palette::Material::Prop, n)
                    .map(|c| ((*n).to_owned(), c, bg::min_diff(c, &cells)))
            })
            .collect();
        if let Some((n, _, d)) = outlines.iter().find(|o| o.2 < bg::MIN_LIGHTNESS_DIFF) {
            bail!(
                "描边 {n} 与格子底色的明度差只有 {d:.1}，低于 {}（ART-SPEC 8.1）",
                bg::MIN_LIGHTNESS_DIFF
            );
        }
        let readability = bg::Readability {
            mains: mains
                .iter()
                .map(|(id, c)| (id.clone(), *c, bg::min_diff(*c, &cells)))
                .collect(),
            outlines,
        };
        summary.readability_failing = readability.failing_mains().len();
        files.insert(
            READABILITY_REPORT.to_owned(),
            readability.report().into_bytes(),
        );
    }
    files.insert(
        format!("{OUT_DIR}/skin.ron"),
        atlas::manifest_ron(&manifest).into_bytes(),
    );
    Ok(Built { files, summary })
}

/// 输出目录中现有的生成文件（相对仓库根）。
fn existing_outputs(root: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for sub in ["", "/anim"] {
        let dir = root.join(format!("{OUT_DIR}{sub}"));
        if !dir.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.path().is_file() {
                out.push(format!(
                    "{OUT_DIR}{sub}/{}",
                    entry.file_name().to_string_lossy()
                ));
            }
        }
    }
    out.sort();
    Ok(out)
}

/// 写出构建结果，并删除输出目录中不再生成的旧文件。
///
/// # Errors
///
/// 写盘失败时返回错误。
pub fn write(root: &Path, built: &Built) -> Result<()> {
    for old in existing_outputs(root)? {
        if !built.files.contains_key(&old) {
            std::fs::remove_file(root.join(&old))?;
        }
    }
    for (rel, bytes) in &built.files {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes).with_context(|| format!("写入 {}", path.display()))?;
    }
    Ok(())
}

/// PNG 像素比较的容差（每通道）：x86_64 Linux（生成资产的平台）要求逐字节一致；其他平台的 tiny-skia
/// SIMD 路径与系统数学库在抗锯齿边缘、半透明叠加处会有差异（ADR-013 第 8 条）。
#[must_use]
pub fn pixel_tolerance() -> u8 {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        0
    } else {
        2
    }
}

/// 非参考平台上允许超过容差的通道比例（‰）：真实的美术改动会改变远多于此的像素，参考平台仍逐字节把关。
pub const OUTLIER_PER_MILLE: usize = 1;

/// 两张图的差异统计：(超过容差的通道数, 总通道数, 最大差)；尺寸不同为 `None`。
#[must_use]
pub fn pixel_diff(a: &Image, b: &Image, tolerance: u8) -> Option<(usize, usize, u8)> {
    if a.w != b.w || a.h != b.h {
        return None;
    }
    let mut bad = 0;
    let mut max = 0;
    for (x, y) in a.data.iter().zip(&b.data) {
        let d = x.abs_diff(*y);
        max = max.max(d);
        if d > tolerance {
            bad += 1;
        }
    }
    Some((bad, a.data.len(), max))
}

/// 去掉回车符：Windows 检出时 git 可能把文本文件转换为 CRLF（`core.autocrlf`），文本输出按 LF 比较。
fn strip_cr(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().copied().filter(|&b| b != b'\r').collect()
}

/// 与磁盘上已提交的资产比较；返回差异描述（空 = 一致）。文本文件忽略换行风格差异。
///
/// # Errors
///
/// 读取磁盘文件失败（除不存在外）时返回错误。
pub fn diff(root: &Path, built: &Built) -> Result<Vec<String>> {
    let tolerance = pixel_tolerance();
    let mut problems = Vec::new();
    for (rel, bytes) in &built.files {
        let Ok(disk) = std::fs::read(root.join(rel)) else {
            problems.push(format!("缺少 {rel}"));
            continue;
        };
        if disk == *bytes || (!rel.ends_with(".png") && strip_cr(&disk) == strip_cr(bytes)) {
            continue;
        }
        if rel.ends_with(".png") && tolerance > 0 {
            let (a, b) = (Image::decode_png(&disk)?, Image::decode_png(bytes)?);
            match pixel_diff(&a, &b, tolerance) {
                Some((bad, total, _)) if bad * 1000 <= total * OUTLIER_PER_MILLE => continue,
                Some((bad, total, max)) => {
                    problems.push(format!(
                        "{rel} 与生成结果不一致（{bad} / {total} 个通道差超过 {tolerance}，最大差 {max}）"
                    ));
                    continue;
                }
                None => {}
            }
        }
        problems.push(format!("{rel} 与生成结果不一致"));
    }
    for old in existing_outputs(root)? {
        if !built.files.contains_key(&old) {
            problems.push(format!("多余的旧文件 {old}"));
        }
    }
    Ok(problems)
}

#[cfg(test)]
mod diff_tests {
    use super::*;

    #[test]
    fn pixel_diff_counts_outliers() {
        let a = Image {
            w: 2,
            h: 1,
            data: vec![10, 10, 10, 255, 0, 0, 0, 0],
        };
        let mut b = a.clone();
        b.data[0] = 12;
        assert_eq!(pixel_diff(&a, &b, 2), Some((0, 8, 2)));
        b.data[1] = 60;
        assert_eq!(pixel_diff(&a, &b, 2), Some((1, 8, 50)));
        let c = Image {
            w: 1,
            h: 2,
            data: a.data.clone(),
        };
        assert_eq!(pixel_diff(&a, &c, 2), None);
    }
}
