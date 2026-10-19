//! 存档与设置（SPEC 14）：`progress.ron`，原子写，损坏时备份并以默认进度启动。

use std::path::{Path, PathBuf};

use rootguard_core::data::Rules;
use rootguard_core::resources::{Progress, Settings};
use rootguard_core::{DefenderId, LevelId};
use trellia::prelude::*;

/// 存档文件名。
pub const FILE_NAME: &str = "progress.ron";
/// 备份文件名（读取失败时）。
pub const BACKUP_NAME: &str = "progress.ron.bak";
/// 覆盖存档目录的环境变量（测试用）。
pub const SAVE_DIR_ENV: &str = "ROOTGUARD_SAVE_DIR";

/// 资源：存档位置与启动时是否已有存档。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaveSlot {
    /// 存档目录；`None` 表示无法确定目录，只在内存中保存。
    pub dir: Option<PathBuf>,
    /// 启动时是否读到了有效存档（标题“继续”按钮）。
    pub existed: bool,
}

impl SaveSlot {
    /// 存档文件路径。
    #[must_use]
    pub fn file(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join(FILE_NAME))
    }
}

/// 存档目录：`ROOTGUARD_SAVE_DIR` → `ProjectDirs::from("", "", "rootguard")` 的 data 目录。
#[must_use]
pub fn save_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(SAVE_DIR_ENV) {
        return Some(PathBuf::from(dir));
    }
    directories::ProjectDirs::from("", "", "rootguard").map(|p| p.data_dir().to_path_buf())
}

/// 新存档：已解锁初始卡牌，默认设置。
#[must_use]
pub fn default_progress(rules: Option<&Rules>) -> Progress {
    Progress {
        version: Progress::VERSION,
        cleared: Default::default(),
        unlocked: rules
            .map(|r| r.starting_cards.iter().copied().collect())
            .unwrap_or_default(),
        settings: Settings::default(),
    }
}

/// 读取存档。文件不存在 → 默认进度；v1 → 迁移到 v2（[`Progress::migrate_v1`]）；损坏或其他版本 → 备份为
/// `.bak`、`log::warn!` 并返回默认进度。
/// 返回 `(进度, 是否读到有效存档)`。
#[must_use]
pub fn load(dir: &Path, rules: Option<&Rules>) -> (Progress, bool) {
    let path = dir.join(FILE_NAME);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return (default_progress(rules), false);
        }
        Err(e) => {
            log::warn!("读取存档失败 {}: {e}，以默认进度启动", path.display());
            return (default_progress(rules), false);
        }
    };
    match rootguard_core::data::parse_ron::<Progress>(&text) {
        Ok(p) if p.version == Progress::VERSION => (p, true),
        // v1 → v2：默认渲染模式改为花园皮肤（裁决十）；下次写入时以 v2 保存
        Ok(p) if p.version == 1 => (p.migrate_v1(), true),
        Ok(p) => {
            backup(
                dir,
                &format!("版本 {} 不符（需要 {}）", p.version, Progress::VERSION),
            );
            (default_progress(rules), false)
        }
        Err(e) => {
            backup(dir, &e.to_string());
            (default_progress(rules), false)
        }
    }
}

fn backup(dir: &Path, reason: &str) {
    let from = dir.join(FILE_NAME);
    let to = dir.join(BACKUP_NAME);
    match std::fs::rename(&from, &to) {
        Ok(()) => log::warn!(
            "存档无效（{reason}），已备份为 {}，以默认进度启动",
            to.display()
        ),
        Err(e) => log::warn!("存档无效（{reason}），备份失败: {e}，以默认进度启动"),
    }
}

/// 原子写入：临时文件 + rename（SPEC 14）。
///
/// # Errors
///
/// 目录创建、序列化、写入或改名失败时返回描述错误的字符串。
pub fn write(dir: &Path, progress: &Progress) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("创建目录 {}: {e}", dir.display()))?;
    let text =
        ron::ser::to_string_pretty(progress, ron::ser::PrettyConfig::new().struct_names(true))
            .map_err(|e| format!("序列化存档: {e}"))?;
    let tmp = dir.join(format!("{FILE_NAME}.tmp"));
    std::fs::write(&tmp, text).map_err(|e| format!("写入 {}: {e}", tmp.display()))?;
    let path = dir.join(FILE_NAME);
    std::fs::rename(&tmp, &path).map_err(|e| format!("改名为 {}: {e}", path.display()))
}

/// 把 World 中的 `Progress` 写入存档；失败只 `log::warn!`。
pub fn save(world: &World) {
    let (Some(slot), Some(progress)) = (
        world.resources.get::<SaveSlot>(),
        world.resources.get::<Progress>(),
    ) else {
        return;
    };
    let Some(dir) = &slot.dir else {
        return;
    };
    if let Err(e) = write(dir, progress) {
        log::warn!("保存进度失败: {e}");
    }
}

/// 通关：记录关卡、解锁卡牌并保存（SPEC 14 写入时机 `BattleEnded(Won)`）。返回本次新解锁的卡牌。
pub fn record_win(world: &mut World, level: LevelId, unlocks: &[DefenderId]) -> Vec<DefenderId> {
    let Some(progress) = world.resources.get_mut::<Progress>() else {
        return Vec::new();
    };
    progress.cleared.insert(level);
    let fresh: Vec<DefenderId> = unlocks
        .iter()
        .copied()
        .filter(|id| progress.unlocked.insert(*id))
        .collect();
    save(world);
    fresh
}

/// 存档位置。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SaveLocation {
    /// `ROOTGUARD_SAVE_DIR` 或系统 data 目录（[`save_dir`]）。
    #[default]
    Default,
    /// 指定目录（测试）。
    Dir(PathBuf),
}

/// 存档插件：读取存档并插入 `Progress` / `SaveSlot`（须在内容插件之后添加，以取得初始卡牌）。
pub struct SavePlugin(pub SaveLocation);

impl Plugin for SavePlugin {
    fn name(&self) -> &'static str {
        "SavePlugin"
    }

    fn build(&self, app: &mut App) {
        let rules = app.world().resources.get::<Rules>().cloned();
        let dir = match &self.0 {
            SaveLocation::Default => save_dir(),
            SaveLocation::Dir(d) => Some(d.clone()),
        };
        let (progress, existed) = match &dir {
            Some(d) => load(d, rules.as_ref()),
            None => {
                log::warn!("无法确定存档目录，进度只保存在内存中");
                (default_progress(rules.as_ref()), false)
            }
        };
        app.insert_resource(progress)
            .insert_resource(SaveSlot { dir, existed });
    }
}
