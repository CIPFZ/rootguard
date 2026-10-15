//! `artgen` 命令行（ART-SPEC 9，ADR-013）。
//!
//! ```text
//! cargo run --release -p artgen -- build            # 生成 assets/skins/garden/
//! cargo run --release -p artgen -- check            # 只比较，不写盘；不一致时退出码 1
//! cargo run --release -p artgen -- preview <id>     # 自查图写到 tmp/artgen/（不入库）
//! ```
//!
//! 任意目录执行均可：仓库根由本 crate 位置推出，或用 `--root <dir>` 指定。

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use artgen::compose::Image;
use artgen::palette::Palette;
use artgen::rig::frame_ticks;
use artgen::rules::Rules;
use resvg::tiny_skia;

/// 预览图放大倍数。
const PREVIEW_SCALE: u32 = 3;

fn usage() -> &'static str {
    "用法: artgen [--root <仓库根>] <build | check | preview <角色 id>>"
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("artgen: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if let Some(i) = args.iter().position(|a| a == "--root") {
        if i + 1 >= args.len() {
            bail!("{}", usage());
        }
        root = PathBuf::from(args.remove(i + 1));
        args.remove(i);
    }
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["build"] => {
            let built = artgen::build(&root)?;
            artgen::write(&root, &built)?;
            print_summary(&built.summary);
            Ok(ExitCode::SUCCESS)
        }
        ["check"] => {
            let built = artgen::build(&root)?;
            let problems = artgen::diff(&root, &built)?;
            if problems.is_empty() {
                println!("artgen: {} 与 art/ 一致", artgen::OUT_DIR);
                Ok(ExitCode::SUCCESS)
            } else {
                for p in &problems {
                    eprintln!("artgen: {p}");
                }
                eprintln!("artgen: 请运行 `cargo run --release -p artgen -- build` 并提交结果");
                Ok(ExitCode::FAILURE)
            }
        }
        ["preview", id] => {
            preview(&root, id)?;
            Ok(ExitCode::SUCCESS)
        }
        _ => bail!("{}", usage()),
    }
}

fn print_summary(s: &artgen::Summary) {
    println!(
        "artgen: {} 个角色，{} 个部件文件，{} 个片段，{} 帧（去重后 {} 张图）",
        s.characters, s.parts, s.clips, s.frames, s.unique
    );
    for (name, w, h) in &s.atlases {
        println!("  {name}.png {w} × {h}");
    }
}

/// 自查用：默认位姿（放大）与每个片段每个状态的帧条，写到 `tmp/artgen/`。
fn preview(root: &Path, id: &str) -> Result<()> {
    let palette = Palette::load(&root.join("art/palette.ron"))?;
    let rules = Rules::load(&root.join("assets"))?;
    let ch = artgen::load_character(&root.join("art/characters").join(id), &palette)?;
    let out = root.join("tmp/artgen");
    std::fs::create_dir_all(&out)?;
    let rest = artgen::render_rest(&ch, &palette)?;
    save(
        &scaled(&[rest], PREVIEW_SCALE)?,
        &out.join(format!("{id}_rest.png")),
    )?;
    for clip in &ch.clips {
        let duration = artgen::clip_duration(&rules, id, clip)?;
        let ticks = frame_ticks(&clip.frames, duration)?;
        for state in ch.rig.output_states() {
            let frames = artgen::render_clip(&ch, &palette, clip, &state, &ticks)?;
            let suffix = if state.name.is_empty() {
                String::new()
            } else {
                format!("_{}", state.name)
            };
            let path = out.join(format!("{id}{suffix}_{}.png", clip.name));
            save(&scaled(&frames, 2)?, &path)?;
            println!("{}（{} 帧，tick {ticks:?}）", path.display(), frames.len());
        }
    }
    Ok(())
}

/// 横向拼接并按整数倍放大（最近邻），背景为浅色便于观察轮廓。
fn scaled(frames: &[Image], scale: u32) -> Result<tiny_skia::Pixmap> {
    let (w, h) = frames.first().map(|f| (f.w, f.h)).context("没有帧")?;
    let mut sheet = tiny_skia::Pixmap::new(w * scale * frames.len() as u32, h * scale)
        .context("预览尺寸为 0")?;
    sheet.fill(tiny_skia::Color::from_rgba8(238, 236, 226, 255));
    for (i, f) in frames.iter().enumerate() {
        let size = tiny_skia::IntSize::from_wh(f.w, f.h).context("帧尺寸")?;
        let src = tiny_skia::Pixmap::from_vec(f.data.clone(), size).context("帧数据")?;
        let paint = tiny_skia::PixmapPaint {
            quality: tiny_skia::FilterQuality::Nearest,
            ..tiny_skia::PixmapPaint::default()
        };
        let t = tiny_skia::Transform::from_scale(scale as f32, scale as f32)
            .post_translate((i as u32 * w * scale) as f32, 0.0);
        sheet.draw_pixmap(0, 0, src.as_ref(), &paint, t, None);
    }
    Ok(sheet)
}

fn save(p: &tiny_skia::Pixmap, path: &Path) -> Result<()> {
    std::fs::write(path, p.encode_png()?).with_context(|| format!("写入 {}", path.display()))
}
