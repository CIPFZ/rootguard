//! Rootguard 入口：`cargo run --release -p rootguard-app [-- --level 1-4 --seed 104]`。

#![forbid(unsafe_code)]

fn main() {
    let args = rootguard_app::CliArgs::parse(std::env::args().skip(1));
    let cfg = trellia::RunConfig {
        title: rootguard_app::TITLE.into(),
        logical_size: rootguard_app::LOGICAL_SIZE,
        font_path: rootguard_app::cjk_font_path(),
        ..trellia::RunConfig::default()
    };
    if let Err(e) = trellia::run_egui(args.build_app(), cfg) {
        eprintln!("rootguard: {e:#}");
        std::process::exit(1);
    }
}
