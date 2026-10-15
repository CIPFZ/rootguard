//! 部件加载与校验、逐帧合成 SVG、resvg 渲染（ART-SPEC 9.2）。
//!
//! 部件是完整的 SVG 文件（便于单独预览），`viewBox` 必须等于角色画布。只允许基本形状元素，颜色只能写
//! `fill="@名"` / `stroke="@名"`（或 `none`），禁止位图、文字、渐变、滤镜、蒙版与 `style`（ART-SPEC 1.5）。
//! 合成时每个部件包一层 `<g transform="matrix(…)">`，按 z 顺序拼成一份 SVG 交给 usvg / resvg。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result, bail};
use resvg::tiny_skia;
use resvg::usvg;

use crate::palette::{Material, Palette, Rgb};
use crate::rig::{Pose, Rig};

/// 允许出现在部件中的元素（ART-SPEC 1.5：无位图、渐变、滤镜）。
const ALLOWED_ELEMENTS: &[&str] = &[
    "svg", "g", "path", "circle", "ellipse", "rect", "polygon", "polyline", "line", "title", "desc",
];

/// 颜色属性：值只能是 `none` 或 `@名`。
const COLOR_ATTRS: &[&str] = &["fill", "stroke"];

/// 禁止的属性（颜色必须经调色板引用；`id` 会在拼接部件时冲突）。
const FORBIDDEN_ATTRS: &[&str] = &[
    "style",
    "color",
    "stop-color",
    "flood-color",
    "lighting-color",
    "id",
    "class",
    "href",
    "filter",
    "mask",
    "clip-path",
];

/// 渲染结果：预乘 RGBA。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Image {
    /// 宽。
    pub w: u32,
    /// 高。
    pub h: u32,
    /// 预乘 RGBA 像素，行优先。
    pub data: Vec<u8>,
}

impl Image {
    /// 受伤闪白版：保留 alpha，颜色全白（ART-SPEC 2.3）。
    #[must_use]
    pub fn flash(&self) -> Self {
        let mut data = self.data.clone();
        for px in data.as_chunks_mut::<4>().0 {
            let a = px[3];
            px[0] = a;
            px[1] = a;
            px[2] = a;
        }
        Self {
            w: self.w,
            h: self.h,
            data,
        }
    }

    /// 去饱和版：每个像素混向其亮度灰（`amount` 0..1），用于“不可用”的卡牌头像（ART-SPEC 4.13）。
    #[must_use]
    pub fn desaturate(&self, amount: f32) -> Self {
        let mut data = self.data.clone();
        for px in data.as_chunks_mut::<4>().0 {
            let [r, g, b, _] = px.map(f32::from);
            let lum = 0.299 * r + 0.587 * g + 0.114 * b;
            for c in &mut px[..3] {
                // 预乘值混合后仍 ≤ alpha，转换不越界
                *c = (f32::from(*c) + (lum - f32::from(*c)) * amount).round() as u8;
            }
        }
        Self {
            w: self.w,
            h: self.h,
            data,
        }
    }

    /// 不透明像素中出现最多的颜色（排除 `exclude`，如描边色）：角色主色（ART-SPEC 8.1 可读性校验）。
    #[must_use]
    pub fn dominant_color(&self, exclude: &[Rgb]) -> Option<Rgb> {
        let mut counts: BTreeMap<Rgb, u32> = BTreeMap::new();
        for px in self.data.as_chunks::<4>().0 {
            if px[3] == 255 {
                let c = Rgb(px[0], px[1], px[2]);
                if !exclude.contains(&c) {
                    *counts.entry(c).or_default() += 1;
                }
            }
        }
        counts
            .into_iter()
            .max_by_key(|(c, n)| (*n, *c))
            .map(|(c, _)| c)
    }

    /// alpha ≥ `threshold` 的像素外接框 `(x0, y0, x1, y1)`（含端点）；全透明时为 `None`。
    #[must_use]
    pub fn opaque_bounds(&self, threshold: u8) -> Option<(u32, u32, u32, u32)> {
        let mut b: Option<(u32, u32, u32, u32)> = None;
        for y in 0..self.h {
            for x in 0..self.w {
                if self.data[((y * self.w + x) * 4 + 3) as usize] >= threshold {
                    b = Some(match b {
                        None => (x, y, x, y),
                        Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                    });
                }
            }
        }
        b
    }

    /// 编码为 PNG（非预乘，tiny-skia / png）。
    ///
    /// # Errors
    ///
    /// 尺寸为 0 或编码失败时返回错误。
    pub fn encode_png(&self) -> Result<Vec<u8>> {
        let size = tiny_skia::IntSize::from_wh(self.w, self.h).context("图像尺寸为 0")?;
        let pixmap =
            tiny_skia::Pixmap::from_vec(self.data.clone(), size).context("像素数据长度不符")?;
        Ok(pixmap.encode_png()?)
    }

    /// 解码 PNG（转为预乘 RGBA）。
    ///
    /// # Errors
    ///
    /// 解码失败时返回错误。
    pub fn decode_png(bytes: &[u8]) -> Result<Self> {
        let pixmap = tiny_skia::Pixmap::decode_png(bytes)?;
        Ok(Self {
            w: pixmap.width(),
            h: pixmap.height(),
            data: pixmap.data().to_vec(),
        })
    }
}

/// 一个部件文件：`viewBox` 内的子元素源码（颜色引用尚未替换）与引用到的颜色名。
#[derive(Clone, Debug)]
pub(crate) struct PartSource {
    pub(crate) body: String,
}

/// 一个角色的全部部件源码：(部件名, 变体) → 源码。
pub struct Parts {
    character: String,
    material: Material,
    canvas: (u32, u32),
    sources: BTreeMap<(String, Option<String>), PartSource>,
}

/// 读取并校验单个部件 SVG，返回替换颜色前的内部源码。
pub(crate) fn load_part(
    path: &Path,
    canvas: (u32, u32),
    palette: &Palette,
    character: &str,
    material: Material,
) -> Result<PartSource> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("读取部件 {}", path.display()))?;
    let doc = usvg::roxmltree::Document::parse(&text)
        .with_context(|| format!("解析部件 {}", path.display()))?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        bail!("{}：根元素必须是 <svg>", path.display());
    }
    let view_box = root.attribute("viewBox").unwrap_or_default();
    let nums: Vec<f32> = view_box
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    let (w, h) = canvas;
    if nums != [0.0, 0.0, w as f32, h as f32] {
        bail!(
            "{}：viewBox 应为 \"0 0 {w} {h}\"（ART-SPEC 2.1 画布），实际 {view_box:?}",
            path.display()
        );
    }
    for node in root.descendants().filter(usvg::roxmltree::Node::is_element) {
        let name = node.tag_name().name();
        if !ALLOWED_ELEMENTS.contains(&name) {
            bail!(
                "{}：不允许的元素 <{name}>（ART-SPEC 1.5 / 9.2）",
                path.display()
            );
        }
        for attr in node.attributes() {
            let key = attr.name();
            if FORBIDDEN_ATTRS.contains(&key) && !(node == root && key == "id") {
                bail!("{}：<{name}> 不允许属性 {key}", path.display());
            }
            if COLOR_ATTRS.contains(&key) {
                let value = attr.value().trim();
                if value == "none" {
                    continue;
                }
                let token = value.strip_prefix('@').with_context(|| {
                    format!(
                        "{}：<{name} {key}=\"{value}\"> 颜色必须写 @调色板名（ART-SPEC 1.3）",
                        path.display()
                    )
                })?;
                if palette.resolve(character, material, token).is_none() {
                    bail!(
                        "{}：颜色 @{token} 不在调色板中（角色 {character}）",
                        path.display()
                    );
                }
            }
        }
    }
    let children: Vec<_> = root.children().collect();
    let body = match (children.first(), children.last()) {
        (Some(first), Some(last)) => text[first.range().start..last.range().end].to_owned(),
        _ => String::new(),
    };
    Ok(PartSource { body })
}

impl Parts {
    /// 读取角色目录 `parts/` 下 rig 声明的全部部件与变体。
    ///
    /// # Errors
    ///
    /// 任一部件文件缺失或不合规时返回错误；目录中有 rig 未声明的 SVG 也报错（防止遗漏）。
    pub fn load(dir: &Path, character: &str, rig: &Rig, palette: &Palette) -> Result<Self> {
        let canvas = rig.kind.canvas();
        let material = rig.kind.material();
        let mut sources = BTreeMap::new();
        let mut expected = BTreeSet::new();
        for p in &rig.parts {
            let variants = std::iter::once(None).chain(p.variants.iter().map(Some));
            for v in variants {
                if v.is_some_and(|v| v == "base" || v == "hidden") {
                    bail!("部件 {} 的变体名不能是保留字 {v:?}", p.name);
                }
                let file = match v {
                    None => format!("{}.svg", p.name),
                    Some(v) => format!("{}_{v}.svg", p.name),
                };
                let src = load_part(&dir.join(&file), canvas, palette, character, material)?;
                expected.insert(file);
                sources.insert((p.name.clone(), v.cloned()), src);
            }
        }
        let mut extra = Vec::new();
        for entry in std::fs::read_dir(dir).with_context(|| format!("读取 {}", dir.display()))? {
            let name = entry?.file_name().to_string_lossy().into_owned();
            if name.ends_with(".svg") && !expected.contains(&name) {
                extra.push(name);
            }
        }
        if !extra.is_empty() {
            extra.sort();
            bail!("{}：rig.ron 未声明的部件文件 {extra:?}", dir.display());
        }
        Ok(Self {
            character: character.to_owned(),
            material,
            canvas,
            sources,
        })
    }

    /// 合成一帧的 SVG 文本（颜色引用替换为十六进制）。
    ///
    /// # Errors
    ///
    /// 位姿引用了不存在的部件变体时返回错误。
    pub fn compose(&self, pose: &Pose<'_>, palette: &Palette) -> Result<String> {
        let (w, h) = self.canvas;
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n"
        );
        for (part, variant, m, op) in pose {
            let key = (part.name.clone(), variant.map(str::to_owned));
            let src = self
                .sources
                .get(&key)
                .with_context(|| format!("部件 {} 没有变体 {variant:?}", part.name))?;
            let [a, b, c, d, e, f] = m.0;
            let opacity = if *op < 1.0 {
                format!(" opacity=\"{op:.3}\"")
            } else {
                String::new()
            };
            let _ = writeln!(
                svg,
                "<g transform=\"matrix({a:.5} {b:.5} {c:.5} {d:.5} {e:.4} {f:.4})\"{opacity}>"
            );
            svg.push_str(&substitute(
                &src.body,
                palette,
                &self.character,
                self.material,
            ));
            svg.push_str("\n</g>\n");
        }
        svg.push_str("</svg>\n");
        Ok(svg)
    }

    /// 合成并渲染一帧；渲染前检查实际出现的每个颜色都来自调色板（含未写 fill 时 SVG 默认的黑色）。
    ///
    /// # Errors
    ///
    /// 合成、解析失败或出现调色板外颜色时返回错误。
    pub fn render(&self, pose: &Pose<'_>, palette: &Palette) -> Result<Image> {
        let (w, h) = self.canvas;
        self.render_window(pose, palette, (0.0, 0.0, w as f32), (w, h))
    }

    /// 合成并把画布中以 `(x, y)` 为左上角、宽 `win_w` 的窗口缩放渲染到 `out` 尺寸（卡牌头像，ART-SPEC 4.13）；
    /// 窗口高度按输出宽高比推出。颜色校验同 [`Parts::render`]。
    ///
    /// # Errors
    ///
    /// 同 [`Parts::render`]。
    pub fn render_window(
        &self,
        pose: &Pose<'_>,
        palette: &Palette,
        (x, y, win_w): (f32, f32, f32),
        (out_w, out_h): (u32, u32),
    ) -> Result<Image> {
        let svg = self.compose(pose, palette)?;
        let tree = usvg::Tree::from_str(&svg, &usvg::Options::default())
            .with_context(|| format!("{}：合成后的 SVG 无法解析", self.character))?;
        let allowed = palette.allowed(&self.character, self.material);
        check_colors(tree.root(), &allowed).with_context(|| {
            format!(
                "{}：出现调色板外的颜色（ART-SPEC 1.3）；检查部件是否漏写 fill",
                self.character
            )
        })?;
        let mut pixmap = tiny_skia::Pixmap::new(out_w, out_h).context("画布尺寸为 0")?;
        let k = out_w as f32 / win_w;
        let t = tiny_skia::Transform::from_row(k, 0.0, 0.0, k, -x * k, -y * k);
        resvg::render(&tree, t, &mut pixmap.as_mut());
        Ok(Image {
            w: out_w,
            h: out_h,
            data: pixmap.take(),
        })
    }
}

/// 把 `fill="@名"` / `stroke="@名"` 替换为十六进制颜色（校验已在加载时完成）。
pub(crate) fn substitute(
    body: &str,
    palette: &Palette,
    character: &str,
    material: Material,
) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(i) = rest.find("\"@") {
        out.push_str(&rest[..=i]);
        rest = &rest[i + 2..];
        let end = rest.find('"').unwrap_or(rest.len());
        let token = &rest[..end];
        match palette.resolve(character, material, token) {
            Some(c) => out.push_str(&c.hex()),
            None => {
                out.push('@');
                out.push_str(token);
            }
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn paint_color(paint: &usvg::Paint) -> Result<Rgb> {
    match paint {
        usvg::Paint::Color(c) => Ok(Rgb(c.red, c.green, c.blue)),
        _ => bail!("只允许纯色填充（ART-SPEC 1.5）"),
    }
}

pub(crate) fn check_colors(group: &usvg::Group, allowed: &BTreeSet<Rgb>) -> Result<()> {
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => check_colors(g, allowed)?,
            usvg::Node::Path(p) => {
                let paints = p
                    .fill()
                    .map(usvg::Fill::paint)
                    .into_iter()
                    .chain(p.stroke().map(usvg::Stroke::paint));
                for paint in paints {
                    let c = paint_color(paint)?;
                    if !allowed.contains(&c) {
                        bail!("颜色 {}", c.hex());
                    }
                }
            }
            usvg::Node::Image(_) => bail!("不允许位图（ART-SPEC 1.5）"),
            usvg::Node::Text(_) => bail!("不允许文字"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PALETTE: &str = r##"Palette(
        global: { "leaf": "#5DBB63", "outline": "#2F3A2A" },
        shade: (guardian: 0.18, husk: 0.22),
        characters: {},
    )"##;

    fn write_part(dir: &Path, name: &str, body: &str) {
        std::fs::write(
            dir.join(name),
            format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 32 32\">{body}</svg>"),
        )
        .expect("写入");
    }

    fn temp(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("artgen-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("目录");
        dir
    }

    #[test]
    fn substitutes_palette_tokens() {
        let p = Palette::parse(PALETTE).expect("调色板");
        let s = substitute(
            r#"<circle fill="@leaf" stroke="@outline"/>"#,
            &p,
            "x",
            Material::Guardian,
        );
        assert_eq!(s, r##"<circle fill="#5DBB63" stroke="#2F3A2A"/>"##);
    }

    #[test]
    fn rejects_raw_colors_and_forbidden_elements() {
        let p = Palette::parse(PALETTE).expect("调色板");
        let dir = temp("parts");
        let check = |body: &str| {
            write_part(&dir, "a.svg", body);
            load_part(&dir.join("a.svg"), (32, 32), &p, "x", Material::Guardian)
        };
        assert!(check(r#"<circle cx="4" cy="4" r="2" fill="@leaf"/>"#).is_ok());
        assert!(
            check(r##"<circle r="2" fill="#5DBB63"/>"##).is_err(),
            "十六进制颜色"
        );
        assert!(check(r#"<circle r="2" fill="red"/>"#).is_err(), "颜色名");
        assert!(
            check(r#"<circle r="2" fill="@nope"/>"#).is_err(),
            "调色板外"
        );
        assert!(
            check(r#"<circle r="2" style="fill:#000"/>"#).is_err(),
            "style"
        );
        assert!(check(r#"<linearGradient/>"#).is_err(), "渐变");
        assert!(check(r#"<image href="x.png"/>"#).is_err(), "位图");
        std::fs::write(dir.join("b.svg"), r#"<svg viewBox="0 0 16 16"/>"#).expect("写入");
        assert!(
            load_part(&dir.join("b.svg"), (32, 32), &p, "x", Material::Guardian).is_err(),
            "画布"
        );
    }

    #[test]
    fn missing_fill_is_caught_after_parse() {
        let tree = usvg::Tree::from_str(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><circle cx="4" cy="4" r="2"/></svg>"#,
            &usvg::Options::default(),
        )
        .expect("解析");
        let allowed = BTreeSet::from([Rgb(0x5D, 0xBB, 0x63)]);
        assert!(
            check_colors(tree.root(), &allowed).is_err(),
            "默认黑色不在调色板"
        );
    }

    #[test]
    fn flash_keeps_alpha() {
        let img = Image {
            w: 1,
            h: 2,
            data: vec![10, 20, 30, 128, 0, 0, 0, 0],
        };
        assert_eq!(img.flash().data, vec![128, 128, 128, 128, 0, 0, 0, 0]);
        assert_eq!(img.opaque_bounds(1), Some((0, 0, 0, 0)));
    }
}
