//! # rootguard-content
//!
//! 职责：从 `assets/data/` 加载防御者 / 弹丸 / 敌人 / 关卡 / 全局规则 RON，按 SPEC.md 4.5 校验，
//! 构建只读注册表（[`Content`]）。`rootguard-app` 在启动时调用 [`load`]，成功后添加
//! `rootguard-core` 的 `ContentPlugin`，失败则记为启动错误（SPEC.md 3.3 / 4）。
//!
//! 依赖边界（SPEC.md 3.1）：只依赖 `rootguard-core` 与 `trellia-assets`（及通用库，ADR-001）。
//!
//! 不负责：规则与模拟（`rootguard-core`）、渲染与 UI（`rootguard-app`）。

#![forbid(unsafe_code)]

mod validate;

use std::fmt;
use std::path::{Path, PathBuf};

use rootguard_core::Content;
use rootguard_core::data::{DefenderDef, EnemyDef, LevelDef, ProjectileDef, Rules};
use rootguard_core::registry::{
    DefenderRegistry, EnemyRegistry, LevelRegistry, ProjectileRegistry,
};
use trellia_assets::{AssetError, AssetServer};

pub use validate::validate;

/// 数据目录（相对资产根目录）。
pub const DATA_DIR: &str = "data";

/// 一条校验错误：文件路径 + 字段名 + 原因（SPEC 4.5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// 文件路径（相对资产根目录）。
    pub path: String,
    /// 字段名（嵌套字段用 `.` 与 `[i]` 表示）。
    pub field: String,
    /// 原因。
    pub reason: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: 字段 `{}`: {}", self.path, self.field, self.reason)
    }
}

/// 内容加载失败。
#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    /// 文件读取或 RON 解析失败（信息含完整路径）。
    #[error(transparent)]
    Asset(#[from] AssetError),
    /// 目录列举失败。
    #[error("读取目录失败 {path}: {source}")]
    Dir {
        /// 目录。
        path: PathBuf,
        /// 底层错误。
        source: std::io::Error,
    },
    /// 校验失败（全部错误）。
    #[error("内容校验失败:\n{}", .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"))]
    Invalid(Vec<ValidationError>),
}

/// 一个已解析、尚未校验的数据文件。
#[derive(Debug, Clone)]
pub struct Loaded<T> {
    /// 相对资产根目录的路径。
    pub path: String,
    /// 定义。
    pub def: T,
}

/// 解析后、校验前的全部原始数据。
#[derive(Debug, Clone)]
pub struct RawContent {
    /// 防御者。
    pub defenders: Vec<Loaded<DefenderDef>>,
    /// 弹丸。
    pub projectiles: Vec<Loaded<ProjectileDef>>,
    /// 敌人。
    pub enemies: Vec<Loaded<EnemyDef>>,
    /// 关卡。
    pub levels: Vec<Loaded<LevelDef>>,
    /// 全局规则。
    pub rules: Loaded<Rules>,
}

/// 列出 `data/<sub>/` 下以 `suffix` 结尾的文件（相对资产根目录，按文件名排序）。
fn list(server: &AssetServer, sub: &str, suffix: &str) -> Result<Vec<String>, ContentError> {
    let dir = server.root().join(DATA_DIR).join(sub);
    let entries = std::fs::read_dir(&dir).map_err(|source| ContentError::Dir {
        path: dir.clone(),
        source,
    })?;
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.ends_with(suffix))
        .collect();
    names.sort();
    Ok(names
        .into_iter()
        .map(|n| format!("{DATA_DIR}/{sub}/{n}"))
        .collect())
}

fn load_all<T: serde::de::DeserializeOwned>(
    server: &AssetServer,
    sub: &str,
    suffix: &str,
) -> Result<Vec<Loaded<T>>, ContentError> {
    list(server, sub, suffix)?
        .into_iter()
        .map(|path| {
            let def = server.load_ron(&path)?;
            Ok(Loaded { path, def })
        })
        .collect()
}

/// 读取并解析全部数据文件（不校验）。
///
/// # Errors
///
/// 目录不存在、文件读取或 RON 解析失败时返回 [`ContentError`]（含文件路径）。
pub fn load_raw(server: &AssetServer) -> Result<RawContent, ContentError> {
    let rules_path = format!("{DATA_DIR}/rules.ron");
    Ok(RawContent {
        defenders: load_all(server, "defenders", ".defender.ron")?,
        projectiles: load_all(server, "projectiles", ".projectile.ron")?,
        enemies: load_all(server, "enemies", ".enemy.ron")?,
        levels: load_all(server, "levels", ".level.ron")?,
        rules: Loaded {
            def: server.load_ron(&rules_path)?,
            path: rules_path,
        },
    })
}

/// 由已校验的原始数据构建注册表。
#[must_use]
pub fn build(raw: RawContent) -> Content {
    let mut defenders = DefenderRegistry::default();
    for d in raw.defenders {
        defenders.insert(d.def.id, d.def);
    }
    let mut projectiles = ProjectileRegistry::default();
    for p in raw.projectiles {
        projectiles.insert(p.def.id, p.def);
    }
    let mut enemies = EnemyRegistry::default();
    for e in raw.enemies {
        enemies.insert(e.def.id, e.def);
    }
    Content {
        defenders,
        projectiles,
        enemies,
        levels: LevelRegistry::new(raw.levels.into_iter().map(|l| l.def).collect()),
        rules: raw.rules.def,
    }
}

/// 加载、校验并构建全部内容（SPEC 4）。
///
/// # Errors
///
/// 任一文件读取 / 解析失败，或任一校验规则不满足时返回 [`ContentError`]。
pub fn load(server: &AssetServer) -> Result<Content, ContentError> {
    let raw = load_raw(server)?;
    let errors = validate(&raw);
    if !errors.is_empty() {
        return Err(ContentError::Invalid(errors));
    }
    Ok(build(raw))
}

/// 以指定资产根目录加载（测试与工具用）。
///
/// # Errors
///
/// 同 [`load`]。
pub fn load_from(root: impl AsRef<Path>) -> Result<Content, ContentError> {
    load(&AssetServer::with_root(root.as_ref()))
}
