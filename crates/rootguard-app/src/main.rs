//! Rootguard 入口：`cargo run --release -p rootguard-app`。

#![forbid(unsafe_code)]

fn main() {
    let cfg = trellia::RunConfig {
        title: rootguard_app::TITLE.into(),
        logical_size: rootguard_app::LOGICAL_SIZE,
        ..trellia::RunConfig::default()
    };
    if let Err(e) = trellia::run_egui(rootguard_app::build_app(), cfg) {
        eprintln!("rootguard: {e:#}");
        std::process::exit(1);
    }
}
