//! A1 审核表（ART-SPEC 10，AGENT 第 9 章 A1）：全部角色的默认位姿、受损 / 护甲状态与卡牌头像拼成一张图，
//! 另出纯黑剪影版（剪影测试）。审核表是文档，不是资产，写到 `docs/art/`。
//!
//! 葫芦壳另附一张罩在石籽荚上的示意（壳沿下露出宿主的眼睛，ART-SPEC 4.9）。
//! 每格左上角是角色序号（SPEC 7.1 / 8.1 的表格顺序，部分审核表中序号不变），对照表见 `docs/art/review-A1.md`。
//! 管线不渲染文字（不引入字体依赖，ADR-013），序号用内置的 3 × 5 点阵数字绘制。

use std::path::Path;

use anyhow::{Context, Result};
use resvg::tiny_skia;

use crate::compose::Image;
use crate::palette::Palette;
use crate::{Character, character_dirs, load_character, render_portrait};

/// 角色顺序：SPEC 7.1 防御者表、8.1 敌人表。
pub const ORDER: [&str; 21] = [
    "pebble_pod",
    "dewcup",
    "barkwall",
    "frostpod",
    "twinpod",
    "thornmat",
    "tuberblast",
    "emberbulb",
    "gourdshell",
    "snapmaw",
    "gustbloom",
    "sunlantern",
    "husk",
    "twig_husk",
    "clay_husk",
    "sprinter_husk",
    "vault_husk",
    "shield_husk",
    "carrier_husk",
    "drum_husk",
    "sproutling",
];

const COLUMNS: usize = 3;
const GAP: u32 = 8;
const LABEL_H: u32 = 22;
const BACKDROP: [u8; 3] = [238, 236, 226];
const TILE: [u8; 3] = [222, 217, 200];
const INK: [u8; 3] = [60, 50, 40];

/// 3 × 5 点阵数字（每行 3 位，高位在左）。
const DIGITS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b010, 0b010, 0b010],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
];

/// 一格：序号与按顺序排开的图像。
struct Tile {
    index: usize,
    images: Vec<Image>,
}

impl Tile {
    fn size(&self) -> (u32, u32) {
        let w = self.images.iter().map(|i| i.w + GAP).sum::<u32>() + GAP;
        let h = LABEL_H + self.images.iter().map(|i| i.h).max().unwrap_or(0) + GAP;
        (w, h)
    }
}

fn color([r, g, b]: [u8; 3]) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(r, g, b, 255)
}

fn fill(p: &mut tiny_skia::Pixmap, x: u32, y: u32, w: u32, h: u32, c: [u8; 3]) {
    if let Some(rect) = tiny_skia::Rect::from_xywh(x as f32, y as f32, w as f32, h as f32) {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(color(c));
        p.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
    }
}

fn draw_number(p: &mut tiny_skia::Pixmap, x: u32, y: u32, n: usize) {
    const SCALE: u32 = 3;
    for (i, ch) in n.to_string().chars().enumerate() {
        let Some(d) = ch.to_digit(10) else { continue };
        let ox = x + i as u32 * 4 * SCALE;
        for (row, bits) in DIGITS[d as usize].iter().enumerate() {
            for col in 0..3 {
                if bits & (0b100 >> col) != 0 {
                    fill(
                        p,
                        ox + col * SCALE,
                        y + row as u32 * SCALE,
                        SCALE,
                        SCALE,
                        INK,
                    );
                }
            }
        }
    }
}

fn blit(p: &mut tiny_skia::Pixmap, img: &Image, x: u32, y: u32) -> Result<()> {
    let size = tiny_skia::IntSize::from_wh(img.w, img.h).context("图像尺寸为 0")?;
    let src = tiny_skia::Pixmap::from_vec(img.data.clone(), size).context("像素数据")?;
    p.draw_pixmap(
        x as i32,
        y as i32,
        src.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );
    Ok(())
}

/// 纯黑剪影（保留 alpha）。
fn silhouette(img: &Image) -> Image {
    let mut data = img.data.clone();
    for px in data.as_chunks_mut::<4>().0 {
        px[0] = 0;
        px[1] = 0;
        px[2] = 0;
    }
    Image {
        w: img.w,
        h: img.h,
        data,
    }
}

fn sheet(tiles: &[Tile], backdrop: [u8; 3], tile_bg: [u8; 3]) -> Result<tiny_skia::Pixmap> {
    let cell_w = tiles.iter().map(|t| t.size().0).max().unwrap_or(1);
    let rows: Vec<&[Tile]> = tiles.chunks(COLUMNS).collect();
    let row_h: Vec<u32> = rows
        .iter()
        .map(|r| r.iter().map(|t| t.size().1).max().unwrap_or(0))
        .collect();
    let width = GAP + COLUMNS as u32 * (cell_w + GAP);
    let height = GAP + row_h.iter().map(|h| h + GAP).sum::<u32>();
    let mut p = tiny_skia::Pixmap::new(width, height.max(1)).context("审核表尺寸为 0")?;
    p.fill(color(backdrop));
    let mut y = GAP;
    for (r, row) in rows.iter().enumerate() {
        for (c, tile) in row.iter().enumerate() {
            let x = GAP + c as u32 * (cell_w + GAP);
            fill(&mut p, x, y, cell_w, row_h[r], tile_bg);
            draw_number(&mut p, x + 6, y + 4, tile.index);
            let mut ix = x + GAP;
            for img in &tile.images {
                blit(&mut p, img, ix, y + LABEL_H)?;
                ix += img.w + GAP;
            }
        }
        y += row_h[r] + GAP;
    }
    Ok(p)
}

/// 把 `top` 叠在 `base` 上（同尺寸画布、同锚点，如葫芦壳罩在宿主上，ART-SPEC 2.1）。
fn over(base: &Image, top: &Image) -> Result<Image> {
    let size = tiny_skia::IntSize::from_wh(base.w, base.h).context("图像尺寸为 0")?;
    let mut p = tiny_skia::Pixmap::from_vec(base.data.clone(), size).context("像素数据")?;
    blit_pixmap(&mut p, top)?;
    Ok(Image {
        w: base.w,
        h: base.h,
        data: p.take(),
    })
}

fn blit_pixmap(p: &mut tiny_skia::Pixmap, img: &Image) -> Result<()> {
    blit(p, img, 0, 0)
}

/// 一个角色的审核内容：各状态的默认位姿，守卫另加头像与“不可用”头像；覆盖层（葫芦壳）另加罩在
/// 宿主 `host` 上的样子。
fn tile_images(
    ch: &Character,
    palette: &Palette,
    host: Option<&Image>,
) -> Result<(Vec<Image>, Vec<Image>)> {
    let clip = ch
        .clips
        .iter()
        .find(|c| c.name == ch.rig.rest)
        .with_context(|| format!("{}：rest 片段 {} 不存在", ch.id, ch.rig.rest))?;
    let mut poses = Vec::new();
    for state in ch.rig.output_states() {
        poses.push(ch.parts.render(&ch.rig.pose(clip, &state, 0), palette)?);
    }
    let mut all = poses.clone();
    if ch.rig.kind == crate::rig::Kind::Overlay
        && let (Some(host), Some(shell)) = (host, poses.first())
    {
        all.push(over(host, shell)?);
    }
    if ch.rig.kind.has_portrait() {
        let (on, off) = render_portrait(ch, palette)?;
        all.push(on);
        all.push(off);
    }
    Ok((all, poses))
}

/// 生成审核表，返回写出的文件（相对仓库根）。`partial` 为真时只写中间审核表 `review-A1-partial.png`
/// （裁决（五）第 3 节），否则写 `review-A1.png` 与剪影版 `review-A1-silhouette.png`。
///
/// # Errors
///
/// 读取角色、渲染或写盘失败时返回错误。
pub fn review(root: &Path, partial: bool) -> Result<Vec<String>> {
    let palette = Palette::load(&root.join("art/palette.ron"))?;
    let mut chars = Vec::new();
    for dir in character_dirs(root)? {
        let ch = load_character(&dir, &palette)?;
        // 审核表只含 21 个角色；弹丸与 Dew 不在 A1 审核范围
        let Some(i) = ORDER.iter().position(|id| *id == ch.id) else {
            continue;
        };
        chars.push((i + 1, ch));
    }
    chars.sort_by_key(|(i, _)| *i);
    // 覆盖层的示意宿主：守卫模板石籽荚（裁决五）
    let host = chars
        .iter()
        .find(|(_, c)| c.id == "pebble_pod")
        .map(|(_, c)| crate::render_rest(c, &palette))
        .transpose()?;
    let mut color_tiles = Vec::new();
    let mut black_tiles = Vec::new();
    for (index, ch) in &chars {
        let (all, poses) = tile_images(ch, &palette, host.as_ref())?;
        color_tiles.push(Tile {
            index: *index,
            images: all,
        });
        black_tiles.push(Tile {
            index: *index,
            images: poses.iter().map(silhouette).collect(),
        });
    }
    let dir = root.join("docs/art");
    std::fs::create_dir_all(&dir)?;
    let mut written = Vec::new();
    let mut save = |name: &str, p: &tiny_skia::Pixmap| -> Result<()> {
        let path = dir.join(name);
        std::fs::write(&path, p.encode_png()?)
            .with_context(|| format!("写入 {}", path.display()))?;
        written.push(format!("docs/art/{name}"));
        Ok(())
    };
    if partial {
        save(
            "review-A1-partial.png",
            &sheet(&color_tiles, BACKDROP, TILE)?,
        )?;
    } else {
        save("review-A1.png", &sheet(&color_tiles, BACKDROP, TILE)?)?;
        save(
            "review-A1-silhouette.png",
            &sheet(&black_tiles, [255, 255, 255], [255, 255, 255])?,
        )?;
    }
    Ok(written)
}
