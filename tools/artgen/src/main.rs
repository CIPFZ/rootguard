//! `artgen` 命令行（ART-SPEC 9，ADR-013）。
//!
//! ```text
//! cargo run --release -p artgen -- build            # 生成 assets/skins/garden/
//! cargo run --release -p artgen -- check            # 只比较，不写盘；不一致时退出码 1
//! cargo run --release -p artgen -- preview <id>     # 自查图写到 tmp/artgen/（不入库）
//! cargo run --release -p artgen -- review [--partial] # A1 审核表写到 docs/art/
//! cargo run --release -p artgen -- strip <out.png> <id:[底层+]片段[:状态][@n]>...  # 帧条：逐帧合成在战场底图上
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
    "用法: artgen [--root <仓库根>] <build | check | preview <角色 id> | review [--partial] | strip <out.png> <id:片段[:状态]>...>"
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
        ["review"] | ["review", "--partial"] => {
            for path in artgen::review::review(&root, args.len() == 2)? {
                println!("artgen: 写出 {path}");
            }
            Ok(ExitCode::SUCCESS)
        }
        ["strip", out, specs @ ..] if !specs.is_empty() => {
            strip(&root, Path::new(out), specs)?;
            Ok(ExitCode::SUCCESS)
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
    if s.readability_failing > 0 {
        println!(
            "  注意：{} 个角色主色与格子底色的明度差低于 ART-SPEC 8.1 的下限，见 {}（ADR-015）",
            s.readability_failing,
            artgen::READABILITY_REPORT
        );
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

/// 帧条（A2 每类的证据，裁决七第 4 节）：每个 `id:片段[:状态]` 一行，逐帧合成在战场底图 `bg_base` 的一格上，
/// 锚点放法与游戏相同（地面角色脚底 = 格子中心 + 46，弹丸 / Dew / 特效在格子中心），2 倍放大。
fn strip(root: &Path, out: &Path, specs: &[&str]) -> Result<()> {
    use rootguard_core::Grid;
    /// 与游戏 `skin::FOOT_OFFSET_Y` 相同。
    const FOOT_OFFSET_Y: f32 = 46.0;
    const GAP: u32 = 6;
    let palette = Palette::load(&root.join("art/palette.ron"))?;
    let rules = Rules::load(&root.join("assets"))?;
    let bg = artgen::bg::render(root, &palette)?.context("没有 art/bg/，无法合成帧条")?;
    let mut rows: Vec<Vec<Image>> = Vec::new();
    for spec in specs {
        // `@n`：每 n 帧取 1 帧（长片段的帧条）
        let (spec, step) = match spec.split_once('@') {
            Some((s, n)) => (s, n.parse::<usize>().context("@ 后应为帧间隔")?.max(1)),
            None => (*spec, 1),
        };
        let parts: Vec<&str> = spec.split(':').collect();
        let (id, clip_name, state_name) = match parts.as_slice() {
            [id, clip] => (*id, *clip, ""),
            [id, clip, state] => (*id, *clip, *state),
            _ => bail!("帧条参数应为 id:片段[:状态]，实际 {spec}"),
        };
        let ch = artgen::load_character(&root.join("art/characters").join(id), &palette)?;
        // `底层+叠层`：叠层每帧画在底层同一 tick 的帧上（如生产者的 idle + produce 两层，ADR-016 第 2 节）
        let (base_name, clip_name) = match clip_name.split_once('+') {
            Some((b, o)) => (Some(b), o),
            None => (None, clip_name),
        };
        let clip = ch
            .clips
            .iter()
            .find(|c| c.name == clip_name)
            .with_context(|| format!("{id} 没有片段 {clip_name}"))?;
        let state = ch
            .rig
            .output_states()
            .into_iter()
            .find(|s| s.name == state_name)
            .with_context(|| format!("{id} 没有状态 {state_name:?}"))?;
        let ticks = frame_ticks(&clip.frames, artgen::clip_duration(&rules, id, clip)?)?;
        let mut frames = artgen::render_clip(&ch, &palette, clip, &state, &ticks)?;
        if let Some(base_name) = base_name {
            let base = ch
                .clips
                .iter()
                .find(|c| c.name == base_name)
                .with_context(|| format!("{id} 没有片段 {base_name}"))?;
            let base_len = artgen::clip_duration(&rules, id, base)?.max(1);
            let mut start = 0;
            for (f, t) in frames.iter_mut().zip(&ticks) {
                let under = ch
                    .parts
                    .render(&ch.rig.pose(base, &state, start % base_len), &palette)?;
                let size = tiny_skia::IntSize::from_wh(f.w, f.h).context("帧")?;
                let mut p = tiny_skia::Pixmap::from_vec(under.data, size).context("帧")?;
                let top = tiny_skia::Pixmap::from_vec(f.data.clone(), size).context("帧")?;
                p.draw_pixmap(
                    0,
                    0,
                    top.as_ref(),
                    &tiny_skia::PixmapPaint::default(),
                    tiny_skia::Transform::identity(),
                    None,
                );
                f.data = p.take();
                start += t;
            }
        }
        let frames: Vec<Image> = frames.into_iter().step_by(step).collect();
        let (ax, ay) = ch.rig.kind.anchor();
        let center = Grid::cell_center(2, 4);
        let ground = !matches!(
            ch.rig.kind,
            artgen::rig::Kind::Projectile | artgen::rig::Kind::Dew | artgen::rig::Kind::Fx(..)
        );
        let foot = if ground {
            center.y + FOOT_OFFSET_Y
        } else {
            center.y
        };
        let mut row = Vec::new();
        for f in frames {
            let x0 = (center.x - ax * f.w as f32).round() as i32;
            let y0 = (foot - ay * f.h as f32).round() as i32;
            let mut tile = tiny_skia::Pixmap::new(f.w, f.h).context("帧尺寸")?;
            let bgp = tiny_skia::Pixmap::from_vec(
                bg.data.clone(),
                tiny_skia::IntSize::from_wh(bg.w, bg.h).context("底图")?,
            )
            .context("底图")?;
            tile.draw_pixmap(
                -x0,
                -y0,
                bgp.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                tiny_skia::Transform::identity(),
                None,
            );
            let fp = tiny_skia::Pixmap::from_vec(
                f.data.clone(),
                tiny_skia::IntSize::from_wh(f.w, f.h).context("帧")?,
            )
            .context("帧")?;
            tile.draw_pixmap(
                0,
                0,
                fp.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                tiny_skia::Transform::identity(),
                None,
            );
            row.push(Image {
                w: f.w,
                h: f.h,
                data: tile.take(),
            });
        }
        rows.push(row);
    }
    let width = rows
        .iter()
        .map(|r| r.iter().map(|i| i.w + GAP).sum::<u32>() + GAP)
        .max()
        .unwrap_or(1);
    let height = rows
        .iter()
        .map(|r| r.iter().map(|i| i.h).max().unwrap_or(0) + GAP)
        .sum::<u32>()
        + GAP;
    let mut sheet = tiny_skia::Pixmap::new(width, height).context("帧条尺寸")?;
    sheet.fill(tiny_skia::Color::from_rgba8(238, 236, 226, 255));
    let mut y = GAP;
    for row in &rows {
        let mut x = GAP;
        for img in row {
            let p = tiny_skia::Pixmap::from_vec(
                img.data.clone(),
                tiny_skia::IntSize::from_wh(img.w, img.h).context("帧")?,
            )
            .context("帧")?;
            sheet.draw_pixmap(
                x as i32,
                y as i32,
                p.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                tiny_skia::Transform::identity(),
                None,
            );
            x += img.w + GAP;
        }
        y += row.iter().map(|i| i.h).max().unwrap_or(0) + GAP;
    }
    let size = tiny_skia::IntSize::from_wh(sheet.width(), sheet.height()).context("帧条")?;
    let img = Image {
        w: size.width(),
        h: size.height(),
        data: sheet.take(),
    };
    save(&scaled(&[img], 2)?, out)?;
    println!("artgen: 写出 {}", out.display());
    Ok(())
}
