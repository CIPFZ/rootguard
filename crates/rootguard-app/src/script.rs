//! 脚本解（SPEC 16.2 `every_level_has_scripted_solution`，AGENT 3.4）：`tests/scripts/<id>.ron`。
//!
//! 一个脚本是"`level_tick` → 对局动作列表"的时间表，回放时在对应 tick 发出 `UiAction`，
//! 与鼠标操作走同一通道（SPEC 11.1）。格式见 ADR-003。

use rootguard_core::events::UiAction;
use rootguard_core::{DefenderId, LevelId};
use serde::{Deserialize, Serialize};
use trellia::prelude::Vec2;

/// 脚本中的一个动作（对局 `UiAction` 的可序列化子集）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScriptAction {
    /// `UiAction::SelectCard`。
    Select(usize),
    /// `UiAction::DeselectCard`。
    Deselect,
    /// `UiAction::PlaceAt`。
    Place(u8, u8),
    /// `UiAction::ShovelAt`。
    Shovel(u8, u8),
    /// `UiAction::CollectDewAt`。
    Collect(f32, f32),
}

impl ScriptAction {
    /// 转为 `UiAction`。
    #[must_use]
    pub fn to_ui(&self) -> UiAction {
        match *self {
            ScriptAction::Select(i) => UiAction::SelectCard(i),
            ScriptAction::Deselect => UiAction::DeselectCard,
            ScriptAction::Place(row, col) => UiAction::PlaceAt { row, col },
            ScriptAction::Shovel(row, col) => UiAction::ShovelAt { row, col },
            ScriptAction::Collect(x, y) => UiAction::CollectDewAt(Vec2::new(x, y)),
        }
    }

    /// 由对局 `UiAction` 转换；场景动作返回 `None`。
    #[must_use]
    pub fn from_ui(action: &UiAction) -> Option<Self> {
        Some(match *action {
            UiAction::SelectCard(i) => ScriptAction::Select(i),
            UiAction::DeselectCard => ScriptAction::Deselect,
            UiAction::PlaceAt { row, col } => ScriptAction::Place(row, col),
            UiAction::ShovelAt { row, col } => ScriptAction::Shovel(row, col),
            UiAction::CollectDewAt(p) => ScriptAction::Collect(p.x, p.y),
            _ => return None,
        })
    }
}

/// 某一 tick 发出的动作。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Step")]
pub struct Step {
    /// 发出时的 `level_tick`（在下一 tick 生效）。
    pub tick: u64,
    /// 动作。
    pub actions: Vec<ScriptAction>,
}

/// 一关的脚本解。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Script")]
pub struct Script {
    /// 关卡。
    pub level: LevelId,
    /// 选卡（`Forced` 关为空）。
    pub cards: Vec<DefenderId>,
    /// 回放上限（`level_tick`），超过仍未胜利即失败。
    pub max_ticks: u64,
    /// 时间表，按 `tick` 升序。
    pub steps: Vec<Step>,
}

impl Script {
    /// 解析 RON。
    ///
    /// # Errors
    ///
    /// 语法或结构错误时返回带位置的错误。
    pub fn parse(text: &str) -> Result<Self, ron::error::SpannedError> {
        rootguard_core::data::parse_ron(text)
    }

    /// 序列化为 RON（每步一行）。
    ///
    /// # Errors
    ///
    /// 序列化失败时返回错误。
    pub fn to_ron(&self) -> Result<String, ron::Error> {
        let cfg = ron::ser::PrettyConfig::new()
            .depth_limit(2)
            .separate_tuple_members(false)
            .struct_names(true);
        ron::ser::to_string_pretty(self, cfg)
    }
}

/// 回放游标：按 `level_tick` 依次取出动作。
#[derive(Clone, Debug)]
pub struct Replay {
    script: Script,
    next: usize,
}

impl Replay {
    /// 从脚本开始回放。
    #[must_use]
    pub fn new(script: Script) -> Self {
        Self { script, next: 0 }
    }

    /// 取出 `tick` 时应发出的全部动作（跳过已过期的步骤）。
    pub fn due(&mut self, tick: u64) -> Vec<UiAction> {
        let mut out = Vec::new();
        while let Some(step) = self.script.steps.get(self.next) {
            if step.tick > tick {
                break;
            }
            if step.tick == tick {
                out.extend(step.actions.iter().map(ScriptAction::to_ui));
            }
            self.next += 1;
        }
        out
    }

    /// 脚本。
    #[must_use]
    pub fn script(&self) -> &Script {
        &self.script
    }
}
