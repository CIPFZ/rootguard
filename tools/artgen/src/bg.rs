//! 战场静态底图 `bg_base.png`（ART-SPEC v1.2 8.1 战场设计卡 L0 ～ L2）。
//!
//! - L0 天空：`sky_top` → `sky_bottom` 竖向两色渐变（设计卡规定的渐变，程序生成）；
//! - L1 远景：`art/bg/far.svg`（沉睡的根母、两侧枯玉米地剪影、云），与角色部件同一套调色板校验；
//! - L2 草地格：5 × 9 格、几何取自游戏的 `Grid`；列 0 ～ 2 为 `leaf` / `leaf_light` 各降 10% 明度，列 3 ～ 5 线性
//!   过渡到列 6 ～ 8 的 `grass_dry` / 其降 10%，棋盘交替；行间 3 px `leaf_dark` 土沟线；格内无图案。
//!
//! 渐变与过渡色由设计卡的公式从调色板算出，不是手写颜色。L3（非激活车道遮罩）由游戏渲染层叠加；L4 ～ L6
//! 在 A3 完成。可读性：见 [`Readability`]（ADR-015）。

use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result};
use resvg::tiny_skia;
use resvg::usvg;
use rootguard_core::Grid;

use crate::compose::{Image, check_colors, load_part, substitute};
use crate::palette::{BACKGROUND, Material, Palette, Rgb};

/// 背景源文件目录。
pub const BG_DIR: &str = "art/bg";
/// 逻辑分辨率（ART-SPEC 0 / SPEC 2.2）。
pub const SIZE: (u32, u32) = (1280, 720);
/// 天空层底边（ART-SPEC 8.1 L0：y ∈ [0, 90]）。
const SKY_BOTTOM_Y: f32 = 92.0;
/// 草地格各色的降明度（ART-SPEC 8.1 L2“各降 10% 明度”）。
const GRASS_DARKEN: f64 = 0.10;
/// 行间土沟线宽（ART-SPEC 8.1 L2）。
const FURROW_PX: f32 = 3.0;
/// 可读性校验的明度差下限（ART-SPEC 8.1，CIE L*）。
pub const MIN_LIGHTNESS_DIFF: f64 = 25.0;

fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let m = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round() as u8;
    Rgb(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// 背景色：调色板中的名字解析为颜色。
fn color(palette: &Palette, name: &str) -> Result<Rgb> {
    palette
        .resolve(BACKGROUND, Material::Prop, name)
        .with_context(|| format!("调色板缺少背景用色 {name}（ART-SPEC 8.1）"))
}

/// 第 `col` 列的两种棋盘色（ART-SPEC 8.1 L2）。
///
/// # Errors
///
/// 调色板缺色时返回错误。
pub fn cell_colors(palette: &Palette, col: u8) -> Result<[Rgb; 2]> {
    let wet = [
        color(palette, "leaf")?.darken(GRASS_DARKEN),
        color(palette, "leaf_light")?.darken(GRASS_DARKEN),
    ];
    let dry_base = color(palette, "grass_dry")?;
    let dry = [dry_base, dry_base.darken(GRASS_DARKEN)];
    let t = match col {
        0..=2 => 0.0,
        6.. => 1.0,
        c => f64::from(c - 2) / 4.0,
    };
    Ok([mix(wet[0], dry[0], t), mix(wet[1], dry[1], t)])
}

/// 全部格子底色（去重）。
///
/// # Errors
///
/// 调色板缺色时返回错误。
pub fn all_cell_colors(palette: &Palette) -> Result<Vec<Rgb>> {
    let mut v = Vec::new();
    for col in 0..Grid::COLS {
        for c in cell_colors(palette, col)? {
            if !v.contains(&c) {
                v.push(c);
            }
        }
    }
    Ok(v)
}

/// CIE L*（D65，sRGB），0 ～ 100。
#[must_use]
pub fn lightness(c: Rgb) -> f64 {
    let lin = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let y = 0.2126 * lin(c.0) + 0.7152 * lin(c.1) + 0.0722 * lin(c.2);
    if y > 0.008856 {
        116.0 * y.cbrt() - 16.0
    } else {
        903.3 * y
    }
}

/// 与任一格底色的最小明度差。
#[must_use]
pub fn min_diff(c: Rgb, cells: &[Rgb]) -> f64 {
    cells
        .iter()
        .map(|g| (lightness(c) - lightness(*g)).abs())
        .fold(f64::INFINITY, f64::min)
}

/// 战场独立精灵目录（ART-SPEC 8.1 L4 ～ L6）。
pub const SCENE_DIR: &str = "art/bg/scene";

/// 一个战场精灵：名字、图像、归一化锚点。
pub type SceneSprite = (String, Image, (f32, f32));

#[derive(serde::Deserialize)]
struct SceneEntry {
    name: String,
    anchor: (f32, f32),
}

/// 渲染 `art/bg/scene/` 中 `sprites.ron` 列出的精灵（L4 温室外墙与两态阀门、L5 入口与门柱、L6 前景杂物）。
/// 每个 SVG 的 `viewBox` 即精灵尺寸；颜色校验同角色部件。没有该目录时返回空表。
///
/// # Errors
///
/// 清单或 SVG 不合规、出现调色板外颜色时返回错误。
pub fn render_scene(root: &Path, palette: &Palette) -> Result<Vec<SceneSprite>> {
    render_sprites(root, SCENE_DIR, palette)
}

/// HUD 皮肤精灵目录（ART-SPEC 8.3）。
pub const UI_DIR: &str = "art/ui";

/// 渲染 `<rel>/sprites.ron` 列出的单帧精灵（战场 L4 ～ L6 与 HUD 共用）。没有该目录时返回空表。
///
/// # Errors
///
/// 清单或 SVG 不合规、出现调色板外颜色时返回错误。
pub fn render_sprites(root: &Path, rel: &str, palette: &Palette) -> Result<Vec<SceneSprite>> {
    let dir = root.join(rel);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let list = std::fs::read_to_string(dir.join("sprites.ron"))
        .with_context(|| format!("{rel}/sprites.ron"))?;
    let entries: Vec<SceneEntry> =
        ron::from_str(&list).with_context(|| format!("{rel}/sprites.ron"))?;
    let mut out = Vec::new();
    for e in entries {
        let path = dir.join(format!("{}.svg", e.name));
        let text =
            std::fs::read_to_string(&path).with_context(|| format!("读取 {}", path.display()))?;
        let size = view_box_size(&text).with_context(|| format!("{}：viewBox", path.display()))?;
        let part = load_part(&path, size, palette, BACKGROUND, Material::Prop)?;
        let body = substitute(&part.body, palette, BACKGROUND, Material::Prop);
        let (w, h) = size;
        let svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">{body}</svg>"
        );
        let tree = usvg::Tree::from_str(&svg, &usvg::Options::default())
            .with_context(|| format!("{}", path.display()))?;
        check_colors(tree.root(), &palette.allowed(BACKGROUND, Material::Prop))
            .with_context(|| format!("{}：出现调色板外的颜色（ART-SPEC 8.1）", path.display()))?;
        let mut pixmap = tiny_skia::Pixmap::new(w, h).context("精灵尺寸为 0")?;
        resvg::render(
            &tree,
            tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );
        out.push((
            e.name,
            Image {
                w,
                h,
                data: pixmap.take(),
            },
            e.anchor,
        ));
    }
    Ok(out)
}

/// `viewBox="0 0 W H"` 中的整数尺寸。
fn view_box_size(text: &str) -> Option<(u32, u32)> {
    let start = text.find("viewBox=\"")? + "viewBox=\"".len();
    let end = start + text[start..].find('"')?;
    let nums: Vec<u32> = text[start..end]
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    match nums.as_slice() {
        [0, 0, w, h] => Some((*w, *h)),
        _ => None,
    }
}

/// 渲染 L0 ～ L2；没有 `art/bg/` 时返回 `None`。
///
/// # Errors
///
/// 远景部件不合规、调色板缺色或渲染失败时返回错误。
pub fn render(root: &Path, palette: &Palette) -> Result<Option<Image>> {
    let dir = root.join(BG_DIR);
    if !dir.is_dir() {
        return Ok(None);
    }
    let (w, h) = SIZE;
    let far = load_part(
        &dir.join("far.svg"),
        SIZE,
        palette,
        BACKGROUND,
        Material::Prop,
    )?;
    let far = substitute(&far.body, palette, BACKGROUND, Material::Prop);
    // 远景单独校验一次实际颜色（含漏写 fill 时的默认黑色）
    let far_svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">{far}</svg>"
    );
    let tree = usvg::Tree::from_str(&far_svg, &usvg::Options::default()).context("far.svg")?;
    check_colors(tree.root(), &palette.allowed(BACKGROUND, Material::Prop))
        .context("art/bg/far.svg：出现调色板外的颜色（ART-SPEC 8.1）")?;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n"
    );
    // L0 天空
    let _ = writeln!(
        svg,
        "<defs><linearGradient id=\"sky\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\"><stop offset=\"0\" stop-color=\"{}\"/><stop offset=\"1\" stop-color=\"{}\"/></linearGradient></defs>",
        color(palette, "sky_top")?.hex(),
        color(palette, "sky_bottom")?.hex()
    );
    let _ = writeln!(
        svg,
        "<rect x=\"0\" y=\"0\" width=\"{w}\" height=\"{SKY_BOTTOM_Y}\" fill=\"url(#sky)\"/>"
    );
    // 格子区以外的地面（L4 ～ L6 在 A3 覆盖）
    let _ = writeln!(
        svg,
        "<rect x=\"0\" y=\"{}\" width=\"{w}\" height=\"{}\" fill=\"{}\"/>",
        Grid::ORIGIN.y,
        h as f32 - Grid::ORIGIN.y,
        color(palette, "leaf_dark")?.hex()
    );
    // L1 远景
    svg.push_str(&far);
    svg.push('\n');
    // L2 草地格
    for row in 0..Grid::ROWS {
        for col in 0..Grid::COLS {
            let c = cell_colors(palette, col)?[usize::from((row + col) % 2)];
            let x = Grid::cell_left(col);
            let y = Grid::ORIGIN.y + Grid::CELL_SIZE.y * f32::from(row);
            let _ = writeln!(
                svg,
                "<rect x=\"{x}\" y=\"{y}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                Grid::CELL_SIZE.x,
                Grid::CELL_SIZE.y,
                c.hex()
            );
        }
    }
    let furrow = color(palette, "leaf_dark")?.hex();
    for row in 0..=Grid::ROWS {
        let y = Grid::ORIGIN.y + Grid::CELL_SIZE.y * f32::from(row);
        let _ = writeln!(
            svg,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{FURROW_PX}\" fill=\"{furrow}\"/>",
            Grid::ORIGIN.x,
            y - FURROW_PX / 2.0,
            Grid::CELL_SIZE.x * f32::from(Grid::COLS)
        );
    }
    svg.push_str("</svg>\n");
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).context("背景合成")?;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).context("背景尺寸")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    Ok(Some(Image {
        w,
        h,
        data: pixmap.take(),
    }))
}

/// ART-SPEC 8.1 可读性：各角色主色（默认位姿中占比最多的颜色）与描边色，对每个格子底色的最小明度差。
pub struct Readability {
    /// (角色, 主色, 最小明度差)。
    pub mains: Vec<(String, Rgb, f64)>,
    /// (描边名, 颜色, 最小明度差)。
    pub outlines: Vec<(String, Rgb, f64)>,
}

impl Readability {
    /// 主色未达 [`MIN_LIGHTNESS_DIFF`] 的角色。
    #[must_use]
    pub fn failing_mains(&self) -> Vec<&(String, Rgb, f64)> {
        self.mains
            .iter()
            .filter(|m| m.2 < MIN_LIGHTNESS_DIFF)
            .collect()
    }

    /// Markdown 报告（`docs/art/bg-readability.md`）。
    #[must_use]
    pub fn report(&self) -> String {
        let mut s = String::from(
            "# 战场底图可读性（ART-SPEC 8.1，由 `artgen build` 生成，勿手改）\n\n\
             明度为 CIE L*；“最小差”是该颜色与 18 种格子底色的明度差的最小值，规定下限 25。\n\
             强制项为描边（ADR-015）；主色一栏只报告。\n\n| 项 | 颜色 | L* | 最小差 | 达标 |\n|---|---|---|---|---|\n",
        );
        for (name, c, d) in self.outlines.iter().chain(&self.mains) {
            let _ = writeln!(
                s,
                "| {name} | `{}` | {:.1} | {d:.1} | {} |",
                c.hex(),
                lightness(*c),
                if *d >= MIN_LIGHTNESS_DIFF {
                    "是"
                } else {
                    "否"
                }
            );
        }
        s
    }
}
