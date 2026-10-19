//! 事件（SPEC 5.3）与 UI 动作（SPEC 11.1）。全部为纯数据（`Clone + Debug`）。
//!
//! 事件只用于通知呈现层（渲染、音效、场景切换、存档）；模拟内部的跨系统数据走资源队列
//! （`DamageQueue`、`DeathLog` 等），不依赖事件双缓冲的帧语义（ADR-002）。

use trellia_core::{Entity, Vec2};

use crate::components::Cell;
use crate::data::ArmorKind;
use crate::ids::{DefenderId, EnemyId, LevelId, ProjectileId};

/// 防御者被移除的原因（SPEC 5.3）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoveCause {
    /// 被铲除。
    Shovel,
    /// 被啃死。
    Killed,
    /// 自身消耗（Mine / Blast / Gust 触发后）。
    Consumed,
}

/// 伤害来源（SPEC 5.3 / 8.3）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DamageSource {
    /// 弹丸。
    Projectile,
    /// 地刺。
    Spikes,
    /// 地雷。
    Mine,
    /// 爆炸。
    Blast,
    /// 吞食（即死）。
    Chomp,
    /// Hose Burst（即死）。
    Hose,
    /// 敌人啃食（目标为防御者）。
    Bite,
}

/// 对局结果（SPEC 5.3 `BattleEnded`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// 胜利。
    Won,
    /// 失败。
    Lost,
}

/// Dew 来源（SPEC 5.3）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DewSource {
    /// 天空掉落。
    Sky,
    /// 生产者。
    Producer(Entity),
}

/// 防御者已放置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefenderPlaced {
    /// 实体。
    pub entity: Entity,
    /// 防御者 id。
    pub id: DefenderId,
    /// 所在格。
    pub cell: Cell,
}

/// 防御者已移除。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefenderRemoved {
    /// 实体。
    pub entity: Entity,
    /// 防御者 id。
    pub id: DefenderId,
    /// 所在格。
    pub cell: Cell,
    /// 原因。
    pub cause: RemoveCause,
}

/// 敌人已生成。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemySpawned {
    /// 实体。
    pub entity: Entity,
    /// 敌人 id。
    pub id: EnemyId,
    /// 车道。
    pub lane: u8,
}

/// 敌人受到伤害。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyDamaged {
    /// 实体。
    pub entity: Entity,
    /// 伤害量（即死来源为结算前的剩余生命）。
    pub amount: i32,
    /// 来源。
    pub source: DamageSource,
}

/// 敌人护甲破损。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyArmorBroken {
    /// 实体。
    pub entity: Entity,
    /// 护甲种类。
    pub kind: ArmorKind,
}

/// 敌人死亡。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnemyDied {
    /// 实体。
    pub entity: Entity,
    /// 敌人 id。
    pub id: EnemyId,
    /// 车道。
    pub lane: u8,
    /// 死亡位置 x。
    pub x: f32,
}

/// 弹丸已发射。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectileFired {
    /// 弹丸实体。
    pub entity: Entity,
    /// 发射者。
    pub from: Entity,
    /// 弹丸 id。
    pub id: ProjectileId,
}

/// 弹丸命中。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectileHit {
    /// 弹丸实体。
    pub entity: Entity,
    /// 目标。
    pub target: Entity,
    /// 命中时目标是否带护甲（音效 `sfx.hit_armor`，SPEC 13）。
    pub armored: bool,
}

/// Dew 已生成。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DewSpawned {
    /// 实体。
    pub entity: Entity,
    /// 来源。
    pub source: DewSource,
}

/// Dew 已收集。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DewCollected {
    /// 数值。
    pub amount: i32,
    /// 收集位置（HUD 飘字动画起点）。
    pub pos: Vec2,
}

/// 一波开始生成。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaveStarted {
    /// 波次下标（从 0 计）。
    pub index: usize,
    /// 是否为大波。
    pub flag: bool,
}

/// Hose Burst 触发。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoseTriggered {
    /// 车道。
    pub lane: u8,
}

/// 对局结束。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BattleEnded {
    /// 结果。
    pub outcome: Outcome,
}

/// 音量总线（SPEC 11.1 `SetVolume.bus`）。模拟层自有类型，由 `rootguard-app` 映射到引擎总线（ADR-002）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VolumeBus {
    /// 音效。
    Sfx,
    /// 音乐。
    Music,
}

/// 界面语言（SPEC 11.6）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Lang {
    /// 简体中文（默认）。
    #[default]
    Zh,
    /// 英文。
    En,
}

/// 渲染模式（SPEC 12.1）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SkinMode {
    /// 占位渲染（默认，必须完整可玩）。
    #[default]
    Placeholder,
    /// 原创皮肤（用哪套由 [`SkinName`] 决定）。
    Skinned,
}

/// `SkinMode::Skinned` 使用的皮肤（ART-SPEC 0）：`assets/skins/<dir>/`。
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum SkinName {
    /// G5 的几何皮肤 `default`（SPEC 12.3）。
    Default,
    /// 美术阶段的 `garden` 皮肤（ART-SPEC）；`SkinMode::Skinned` 的默认（裁决八第 3 节第 7 项）。
    #[default]
    Garden,
}

impl SkinName {
    /// 全部皮肤。
    pub const ALL: [Self; 2] = [Self::Default, Self::Garden];

    /// 资产目录名（`assets/skins/<dir>/`）。
    #[must_use]
    pub fn dir(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Garden => "garden",
        }
    }
}

/// UI 与模拟之间的唯一接口（SPEC 11.1）。鼠标、按钮、快捷键与测试都只产生它。
///
/// 前 6 个变体为对局动作，由 `ui_action_system` 在 FixedUpdate 中消费；其余为场景动作，
/// 由 `rootguard-app` 的场景在 Update 中消费（ADR-002）。
#[derive(Clone, Debug, PartialEq)]
pub enum UiAction {
    /// 选中第 i 张卡。
    SelectCard(usize),
    /// 取消选卡与铲子。
    DeselectCard,
    /// 切换铲子模式。
    ToggleShovel,
    /// 在格子放置当前选中的卡。
    PlaceAt {
        /// 行。
        row: u8,
        /// 列。
        col: u8,
    },
    /// 铲除格子顶层防御者。
    ShovelAt {
        /// 行。
        row: u8,
        /// 列。
        col: u8,
    },
    /// 在逻辑坐标处收集 Dew。
    CollectDewAt(Vec2),
    /// 暂停 / 继续。
    PauseToggle,
    /// 重开当前关。
    Restart,
    /// 进入下一关。
    NextLevel,
    /// 返回标题。
    BackToTitle,
    /// 返回选关。
    BackToLevelSelect,
    /// 开始关卡（进入选卡或直接对局）。
    StartLevel(LevelId),
    /// 确认选卡并开始对局。
    ConfirmSeeds(Vec<DefenderId>),
    /// 设置音量。
    SetVolume {
        /// 总线。
        bus: VolumeBus,
        /// 音量 0..1。
        volume: f32,
    },
    /// 设置语言。
    SetLanguage(Lang),
    /// 设置渲染模式。
    SetSkin(SkinMode),
    /// 设置皮肤模式使用的皮肤（ART-SPEC 0）。
    SetSkinName(SkinName),
}

impl UiAction {
    /// 是否为对局动作（由模拟层消费）。
    #[must_use]
    pub fn is_battle_action(&self) -> bool {
        matches!(
            self,
            UiAction::SelectCard(_)
                | UiAction::DeselectCard
                | UiAction::ToggleShovel
                | UiAction::PlaceAt { .. }
                | UiAction::ShovelAt { .. }
                | UiAction::CollectDewAt(_)
        )
    }
}
