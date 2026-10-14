//! # rootguard-app
//!
//! 职责：组装 Trellia App（插件顺序见 SPEC.md 3.2）、场景（SPEC.md 3.4）、HUD 与 egui 按钮、
//! 占位渲染（SPEC.md 12.2）与皮肤渲染（SPEC.md 12.3）、音效触发、存档与设置。入口为 `src/main.rs`。
//!
//! Feature：`devtools`（默认开启）——引擎调试面板与 Rootguard 调试窗口（SPEC.md 11.5）。
//!
//! 不负责：任何规则判定。模拟只响应 `UiAction` 事件（SPEC.md 11.1），视觉效果不影响模拟。

#![forbid(unsafe_code)]

#[cfg(feature = "devtools")]
pub mod devtools;
pub mod flow;
pub mod hud;
pub mod i18n;
pub mod render;
pub mod save;
pub mod scenes;
pub mod script;
pub mod sfx;
pub mod skin;

pub use scenes::{BattleScene, TitleScene};

use trellia::prelude::*;

/// 逻辑分辨率（SPEC.md 2.2，引擎 `GfxPlugin` 固定）。
pub const LOGICAL_SIZE: Vec2 = Vec2::new(1280.0, 720.0);

/// 窗口标题。
pub const TITLE: &str = "Rootguard";

/// 游戏内容插件（SPEC.md 3.2 / 3.3）：用 `AssetServer` 加载并校验 `assets/data/`，成功后添加
/// `ContentPlugin` / `BattlePlugin` / `EconomyPlugin` / `WavePlugin`，再加载文本表并添加
/// `SavePlugin` / `SfxPlugin` / `UiPlugin`；任一失败记为启动错误（SPEC.md 4）。
#[derive(Default)]
pub struct GamePlugin {
    /// 存档位置。
    pub save: save::SaveLocation,
}

impl Plugin for GamePlugin {
    fn name(&self) -> &'static str {
        "GamePlugin"
    }

    fn build(&self, app: &mut App) {
        let Some(server) = app.world().resources.get::<AssetServer>().cloned() else {
            // AssetsPlugin 已记录根目录错误
            return;
        };
        match rootguard_content::load(&server) {
            Ok(content) => rootguard_core::add_core_plugins(app, content),
            Err(e) => app.record_startup_error(e),
        }
        match i18n::I18n::load(&server) {
            Ok(table) => {
                app.insert_resource(table);
            }
            Err(e) => app.record_startup_error(e),
        }
        // 皮肤是可选增强（SPEC 12.1）：加载失败时警告并让 Skinned 模式回退占位渲染
        match skin::SkinAssets::load(app.world_mut(), &server) {
            Ok(assets) => {
                app.insert_resource(assets);
            }
            Err(e) => log::warn!("皮肤加载失败，皮肤模式将回退占位渲染：{e}"),
        }
        app.add_plugin(save::SavePlugin(self.save.clone()))
            .add_plugin(sfx::SfxPlugin)
            .add_plugin(flow::UiPlugin);
    }
}

/// 资产根目录：`TRELLIA_ASSETS` 已设置时交给引擎解析（返回 `None`）；否则依次尝试
/// 可执行文件同级 `assets/`、当前目录 `assets/`、编译期工作区 `assets/`（直接运行 `target/` 下的二进制时）。
#[must_use]
pub fn asset_root() -> Option<std::path::PathBuf> {
    if std::env::var_os("TRELLIA_ASSETS").is_some() {
        return None;
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.join("assets")));
    let cwd = std::env::current_dir().ok().map(|d| d.join("assets"));
    let workspace = Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets"));
    [exe_dir, cwd, workspace]
        .into_iter()
        .flatten()
        .find(|p| p.join("data").is_dir())
}

/// 以标题场景组装 App。
#[must_use]
pub fn build_app() -> App {
    build_app_with(TitleScene)
}

/// 以指定初始场景组装 App。插件顺序（SPEC.md 3.2）：`AssetsPlugin` → `GfxPlugin` → `AudioPlugin`
/// → 游戏内容插件 → `ScenePlugin` → `DevtoolsPlugin`（feature `devtools`）。
#[must_use]
pub fn build_app_with(initial: impl Scene) -> App {
    build_app_full(initial, save::SaveLocation::Default)
}

/// 以指定初始场景与存档位置组装 App（测试用独立存档目录）。
#[must_use]
pub fn build_app_full(initial: impl Scene, save: save::SaveLocation) -> App {
    let mut app = App::new();
    app.add_plugin(AssetsPlugin { root: asset_root() })
        .add_plugin(GfxPlugin::new(LOGICAL_SIZE))
        .add_plugin(AudioPlugin)
        .add_plugin(GamePlugin { save })
        .add_plugin(ScenePlugin::new(initial));
    #[cfg(feature = "devtools")]
    app.add_plugin(DevtoolsPlugin::default())
        .add_plugin(devtools::RootguardDevtoolsPlugin);
    app
}

/// 命令行参数（AGENT.md 7）：`--level <id>` 直接进入关卡，`--seed <n>` 覆盖关卡种子，
/// `--autoplay <script.ron>` 以脚本解自动对局（调试 / 演示）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CliArgs {
    /// 直接进入的关卡。
    pub level: Option<String>,
    /// 种子覆盖。
    pub seed: Option<u64>,
    /// 自动回放的脚本路径。
    pub autoplay: Option<std::path::PathBuf>,
}

impl CliArgs {
    /// 解析参数（不含程序名）；无法识别的参数记 `log::warn!` 后忽略。
    #[must_use]
    pub fn parse(args: impl IntoIterator<Item = String>) -> Self {
        let mut out = Self::default();
        let mut it = args.into_iter();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--level" => out.level = it.next(),
                "--seed" => out.seed = it.next().and_then(|s| s.parse().ok()),
                "--autoplay" => out.autoplay = it.next().map(Into::into),
                other => log::warn!("忽略未知参数 {other}"),
            }
        }
        out
    }

    /// 按参数组装 App：给出 `--level` 时直接以该关卡的对局场景启动（`Choose` 关使用默认卡组）。
    #[must_use]
    pub fn build_app(&self) -> App {
        if let Some(path) = &self.autoplay {
            match std::fs::read_to_string(path)
                .map_err(|e| e.to_string())
                .and_then(|t| script::Script::parse(&t).map_err(|e| e.to_string()))
            {
                Ok(s) => return build_app_with(BattleScene::with_autoplay(s)),
                Err(e) => log::warn!("无法读取脚本 {}: {e}，按普通方式启动", path.display()),
            }
        }
        match self
            .level
            .as_deref()
            .and_then(|s| rootguard_core::LevelId::new(s).ok())
        {
            Some(level) => build_app_with(BattleScene::new(level, Vec::new(), self.seed)),
            None => build_app(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_level_and_seed() {
        let args = CliArgs::parse(["--level", "1-4", "--seed", "104"].map(String::from));
        assert_eq!(args.level.as_deref(), Some("1-4"));
        assert_eq!(args.seed, Some(104));
        assert_eq!(CliArgs::parse(Vec::new()), CliArgs::default());
    }
}
