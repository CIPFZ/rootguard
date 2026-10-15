//! 图集打包与引擎 RON 输出（ART-SPEC 2.1 / 9.2）：同尺寸像素完全相同的帧只存一份，按货架法确定性排布，
//! 每张图集 ≤ 4096 × 4096；输出引擎 `Atlas(...)` / `AnimationClip(...)` 格式。

use std::collections::BTreeMap;
use std::fmt::Write as _;

use anyhow::{Result, bail};

use crate::compose::Image;

/// 图集边长上限（ART-SPEC 2.1）。
pub const MAX_ATLAS: u32 = 4096;

/// 帧间留白（像素），避免线性采样时相邻帧渗色。
const PAD: u32 = 2;

/// 生成文件的头注释。
pub const HEADER: &str = "// 由 tools/artgen 生成，勿手改（ART-SPEC 9）";

/// 一个待打包的帧。
pub struct FrameIn {
    /// 帧名（图集内唯一）。
    pub name: String,
    /// 图像。
    pub image: Image,
    /// 归一化锚点。
    pub anchor: (f32, f32),
}

/// 图集中的一帧：(帧名, x, y, w, h, 锚点)。
pub type PackedFrame = (String, u32, u32, u32, u32, (f32, f32));

/// 打包结果。
pub struct Packed {
    /// 图集图像（预乘 RGBA）。
    pub image: Image,
    /// 帧名 → (x, y, w, h, 锚点)，按输入顺序。
    pub frames: Vec<PackedFrame>,
    /// 去重后的不同图像数。
    pub unique: usize,
}

/// 打包一组帧。
///
/// # Errors
///
/// 图集超过 [`MAX_ATLAS`] 时返回错误。
pub fn pack(name: &str, frames: &[FrameIn]) -> Result<Packed> {
    // 去重：相同像素只存一份（如 idle 各周期中相同的呼吸帧）
    let mut unique: BTreeMap<&Image, usize> = BTreeMap::new();
    let mut order: Vec<&Image> = Vec::new();
    let mut first_name: Vec<&str> = Vec::new();
    let mut slot_of = Vec::with_capacity(frames.len());
    for f in frames {
        let slot = *unique.entry(&f.image).or_insert_with(|| {
            order.push(&f.image);
            first_name.push(&f.name);
            order.len() - 1
        });
        slot_of.push(slot);
    }
    // 货架法：高者优先，同高按宽、再按首个帧名，结果与输入顺序无关以外完全确定
    let mut sorted: Vec<usize> = (0..order.len()).collect();
    sorted.sort_by(|&a, &b| {
        (order[b].h, order[b].w, first_name[a]).cmp(&(order[a].h, order[a].w, first_name[b]))
    });
    let area: u64 = order
        .iter()
        .map(|i| u64::from(i.w + PAD) * u64::from(i.h + PAD))
        .sum();
    let widest = order.iter().map(|i| i.w + PAD).max().unwrap_or(1);
    let mut width = 256;
    while width < MAX_ATLAS && (u64::from(width) * u64::from(width) < area || width < widest) {
        width *= 2;
    }
    let mut pos = vec![(0, 0); order.len()];
    let (mut x, mut y, mut shelf) = (0u32, 0u32, 0u32);
    for &i in &sorted {
        let (w, h) = (order[i].w, order[i].h);
        if x + w > width {
            x = 0;
            y += shelf + PAD;
            shelf = 0;
        }
        pos[i] = (x, y);
        x += w + PAD;
        shelf = shelf.max(h);
    }
    let height = (y + shelf).next_multiple_of(4).max(4);
    if width > MAX_ATLAS || height > MAX_ATLAS {
        bail!("图集 {name} 为 {width} × {height}，超过上限 {MAX_ATLAS}（ART-SPEC 2.1）");
    }
    let mut data = vec![0u8; (width * height * 4) as usize];
    for (i, img) in order.iter().enumerate() {
        let (ox, oy) = pos[i];
        for row in 0..img.h {
            let src = (row * img.w * 4) as usize;
            let dst = (((oy + row) * width + ox) * 4) as usize;
            let len = (img.w * 4) as usize;
            data[dst..dst + len].copy_from_slice(&img.data[src..src + len]);
        }
    }
    let out = frames
        .iter()
        .zip(&slot_of)
        .map(|(f, &s)| {
            let (x, y) = pos[s];
            (f.name.clone(), x, y, order[s].w, order[s].h, f.anchor)
        })
        .collect();
    Ok(Packed {
        image: Image {
            w: width,
            h: height,
            data,
        },
        frames: out,
        unique: order.len(),
    })
}

/// 引擎图集 RON（`image` 为相对资产根目录的路径）。
#[must_use]
pub fn atlas_ron(image: &str, packed: &Packed) -> String {
    let mut s = format!("{HEADER}\nAtlas(\n    image: \"{image}\",\n    frames: [\n");
    for (name, x, y, w, h, (ax, ay)) in &packed.frames {
        let _ = writeln!(
            s,
            "        (name: \"{name}\", x: {x}, y: {y}, w: {w}, h: {h}, anchor: ({}, {})),",
            fmt_f(*ax),
            fmt_f(*ay)
        );
    }
    s.push_str("    ],\n)\n");
    s
}

/// 一个输出片段。
pub struct ClipOut {
    /// 片段名。
    pub name: String,
    /// 是否循环。
    pub looping: bool,
    /// (帧名, tick, 事件)。
    pub frames: Vec<(String, u32, Option<String>)>,
}

/// 引擎动画片段 RON。
#[must_use]
pub fn clip_ron(clip: &ClipOut) -> String {
    let mut s = format!(
        "{HEADER}\nAnimationClip(\n    name: \"{}\",\n    looping: {},\n    frames: [\n",
        clip.name, clip.looping
    );
    for (frame, ticks, event) in &clip.frames {
        let ev = event
            .as_ref()
            .map(|e| format!(", event: \"{e}\""))
            .unwrap_or_default();
        let _ = writeln!(s, "        (frame: \"{frame}\", ticks: {ticks}{ev}),");
    }
    s.push_str("    ],\n)\n");
    s
}

/// 皮肤清单 RON：每张图集及其片段（`rootguard-app::skin` 读取）。
#[must_use]
pub fn manifest_ron(atlases: &[(String, Vec<String>)]) -> String {
    let mut s = format!("{HEADER}\nSkin(\n    atlases: [\n");
    for (atlas, clips) in atlases {
        let _ = writeln!(
            s,
            "        (\n            atlas: \"{atlas}\",\n            clips: ["
        );
        for c in clips {
            let _ = writeln!(s, "                \"{c}\",");
        }
        s.push_str("            ],\n        ),\n");
    }
    s.push_str("    ],\n)\n");
    s
}

/// 浮点数的稳定文本形式（最多 4 位小数，去掉多余的 0）。
fn fmt_f(v: f32) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0');
    if s.ends_with('.') {
        format!("{s}0")
    } else {
        s.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(w: u32, h: u32, v: u8) -> Image {
        Image {
            w,
            h,
            data: vec![v; (w * h * 4) as usize],
        }
    }

    #[test]
    fn packs_deduplicated_frames_without_overlap() {
        let frames: Vec<FrameIn> = [(128, 128, 1), (128, 128, 2), (128, 128, 1), (32, 32, 3)]
            .iter()
            .enumerate()
            .map(|(i, &(w, h, v))| FrameIn {
                name: format!("f{i}"),
                image: img(w, h, v),
                anchor: (0.5, 0.94),
            })
            .collect();
        let p = pack("t", &frames).expect("打包");
        assert_eq!(p.unique, 3);
        assert_eq!(
            p.frames[0].1..=p.frames[0].2,
            p.frames[2].1..=p.frames[2].2,
            "相同图像共用矩形"
        );
        let rects: Vec<_> = p.frames.iter().map(|f| (f.1, f.2, f.3, f.4)).collect();
        for (i, a) in rects.iter().enumerate() {
            assert!(a.0 + a.2 <= p.image.w && a.1 + a.3 <= p.image.h);
            for b in &rects[i + 1..] {
                let disjoint =
                    a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1;
                assert!(disjoint || a == b, "{a:?} 与 {b:?} 重叠");
            }
        }
        let again = pack("t", &frames).expect("打包");
        assert_eq!(again.image, p.image, "确定性");
    }

    #[test]
    fn ron_text_is_stable() {
        assert_eq!(fmt_f(0.94), "0.94");
        assert_eq!(fmt_f(0.5), "0.5");
        assert_eq!(fmt_f(1.0), "1.0");
        let c = ClipOut {
            name: "husk_eat".into(),
            looping: true,
            frames: vec![
                ("husk_eat_0".into(), 30, None),
                ("husk_eat_1".into(), 30, Some("eat_hit".into())),
            ],
        };
        assert!(clip_ron(&c).contains("(frame: \"husk_eat_1\", ticks: 30, event: \"eat_hit\"),"));
    }
}
