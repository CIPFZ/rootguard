//! # rootguard-app
//!
//! 职责：组装 Trellia App（插件顺序见 SPEC.md 3.2）、场景（SPEC.md 3.4）、egui UI、
//! 占位 / 皮肤渲染、音效触发、存档与设置。入口为 `src/main.rs`。
//!
//! Feature：`devtools`（默认开启）——引擎调试面板与 Rootguard 调试窗口（SPEC.md 11.5）。
//!
//! 不负责：任何规则判定。模拟只响应 `UiAction` 事件（SPEC.md 11.1），视觉效果不影响模拟。

#![forbid(unsafe_code)]

mod scenes;

pub use scenes::TitleScene;

use trellia::prelude::*;

/// 逻辑分辨率（SPEC.md 2.2，引擎 `GfxPlugin` 固定）。
pub const LOGICAL_SIZE: Vec2 = Vec2::new(1280.0, 720.0);

/// 窗口标题。
pub const TITLE: &str = "Rootguard";

/// 组装 App。插件顺序（SPEC.md 3.2）：`AssetsPlugin` → `GfxPlugin` → `AudioPlugin`
/// → 游戏内容插件 → `ScenePlugin` → `DevtoolsPlugin`（feature `devtools`）。
#[must_use]
pub fn build_app() -> App {
    let mut app = App::new();
    app.add_plugin(AssetsPlugin::default())
        .add_plugin(GfxPlugin::new(LOGICAL_SIZE))
        .add_plugin(AudioPlugin)
        .add_plugin(ScenePlugin::new(TitleScene));
    #[cfg(feature = "devtools")]
    app.add_plugin(DevtoolsPlugin::default());
    app
}
