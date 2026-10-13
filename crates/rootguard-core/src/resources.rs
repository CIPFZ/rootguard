//! 资源（SPEC 5.2）与模拟内部队列。

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use trellia_core::{Entity, Timer, Vec2};

use crate::components::Cell;
use crate::events::{DamageSource, Lang, SkinMode};
use crate::grid::Grid;
use crate::ids::{DefenderId, EnemyId, LevelId};

/// 对局阶段（SPEC 5.2）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// "准备"倒计时。
    Intro,
    /// 进行中。
    Playing,
    /// 胜利。
    Won,
    /// 失败。
    Lost,
}

/// 对局状态（SPEC 5.2）。
#[derive(Clone, Debug, PartialEq)]
pub struct BattleState {
    /// 当前关卡。
    pub level: LevelId,
    /// 关卡内 tick（从 0 计，Intro 与暂停不计入 `Playing` 之外的步骤）。
    pub level_tick: u64,
    /// 当前 Dew。
    pub dew: i32,
    /// 阶段。
    pub phase: Phase,
    /// 选中的卡槽下标。
    pub selected_card: Option<usize>,
    /// 是否处于铲子模式。
    pub shovel_mode: bool,
    /// Intro 倒计时。
    pub intro: Timer,
}

/// 一张卡（SPEC 5.2 `CardSlot`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardSlot {
    /// 防御者 id。
    pub defender: DefenderId,
    /// 冷却（单次计时器；结束即冷却完成）。
    pub cooldown: Timer,
    /// `cooldown.finished() && dew ≥ cost`（SPEC 9），每 tick 由经济系统刷新。
    pub ready: bool,
}

/// 卡槽（SPEC 5.2）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CardTray {
    /// 卡牌，按卡槽顺序。
    pub cards: Vec<CardSlot>,
}

/// 波次状态（SPEC 5.2 / 10.1）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaveState {
    /// 下一波下标（等于波数即全部生成完毕）。
    pub next_wave: usize,
    /// 距下一波生成的计时器。
    pub wave_timer: Timer,
    /// 最近生成的一波的初始总 hp（主生命 + 护甲）。
    pub wave_hp_total: i32,
    /// 最近生成的一波的剩余总 hp。
    pub wave_hp_left: i32,
    /// 是否已全部生成。
    pub all_spawned: bool,
    /// flag 波横幅计时器（显示中为 `Some`）。
    pub banner: Option<Timer>,
    /// 最近一波生成时的 `level_tick`。
    pub last_spawn_tick: Option<u64>,
    /// 波数。
    pub total_waves: usize,
}

/// 车道状态（SPEC 5.2）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaneState {
    /// 各车道 Hose Burst 是否可用。
    pub hose_available: [bool; Grid::ROWS as usize],
    /// 各车道是否激活。
    pub active: [bool; Grid::ROWS as usize],
}

impl LaneState {
    /// 由激活车道列表构造；非激活车道没有 Hose。
    #[must_use]
    pub fn new(lanes: &[u8]) -> Self {
        let mut active = [false; Grid::ROWS as usize];
        for &row in lanes {
            if let Some(slot) = active.get_mut(usize::from(row)) {
                *slot = true;
            }
        }
        Self {
            hose_available: active,
            active,
        }
    }

    /// 车道是否激活。
    #[must_use]
    pub fn is_active(&self, row: u8) -> bool {
        self.active.get(usize::from(row)).copied().unwrap_or(false)
    }

    /// 激活车道列表（升序）。
    #[must_use]
    pub fn active_rows(&self) -> Vec<u8> {
        (0..Grid::ROWS).filter(|&r| self.is_active(r)).collect()
    }
}

/// 一条伤害请求（SPEC 6）：第 8 ～ 10 步 push，第 11 步按 `(target, seq)` 统一结算后清空。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DamageRequest {
    /// 目标（敌人或防御者）。
    pub target: Entity,
    /// 伤害量。
    pub amount: i32,
    /// 来源。
    pub source: DamageSource,
    /// 是否无视护甲。
    pub ignore_armor: bool,
    /// 本 tick 内的提交序号。
    pub seq: u32,
}

/// 伤害请求队列（SPEC 6 `Vec<DamageRequest>` 资源）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DamageQueue {
    /// 本 tick 的请求。
    pub requests: Vec<DamageRequest>,
    next_seq: u32,
}

impl DamageQueue {
    /// 追加请求并分配序号。
    pub fn push(&mut self, target: Entity, amount: i32, source: DamageSource, ignore_armor: bool) {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.requests.push(DamageRequest {
            target,
            amount,
            source,
            ignore_armor,
            seq,
        });
    }

    /// 取走全部请求并按 `(target, seq)` 排序，序号归零。
    pub fn take_sorted(&mut self) -> Vec<DamageRequest> {
        let mut reqs = std::mem::take(&mut self.requests);
        reqs.sort_by_key(|r| (r.target.to_bits().get(), r.seq));
        self.next_seq = 0;
        reqs
    }
}

/// `ui_action_system` 解析出的、待后续系统执行的对局操作（SPEC 6 第 2 → 3 / 4 步）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PendingOps {
    /// 放置请求：(卡槽下标, 格子)。
    pub place: Vec<(usize, Cell)>,
    /// 铲除请求。
    pub shovel: Vec<Cell>,
    /// 收集请求（逻辑坐标）。
    pub collect: Vec<Vec2>,
}

/// 本 tick 死亡的敌人（`damage_system` 写入，`death_system` 消费）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Death {
    /// 实体。
    pub entity: Entity,
    /// 敌人 id。
    pub id: EnemyId,
    /// 车道。
    pub lane: u8,
    /// 死亡位置 x。
    pub x: f32,
    /// 是否生成衍生（Hose 清场不生成，ADR-002）。
    pub spawn_children: bool,
}

/// 本 tick 死亡列表（模拟内部队列，ADR-002）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeathLog(pub Vec<Death>);

/// 设置（SPEC 14）。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// 音效音量 0..1。
    pub sfx: f32,
    /// 音乐音量 0..1。
    pub music: f32,
    /// 语言。
    pub lang: Lang,
    /// 渲染模式。
    pub skin: SkinMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sfx: 1.0,
            music: 1.0,
            lang: Lang::default(),
            skin: SkinMode::default(),
        }
    }
}

/// 进度（SPEC 5.2 / 14）：已通关关卡、已解锁卡牌与设置。由 `rootguard-app` 负责读写文件。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Progress")]
pub struct Progress {
    /// 存档版本。
    pub version: u32,
    /// 已通关关卡。
    pub cleared: BTreeSet<LevelId>,
    /// 已解锁卡牌。
    pub unlocked: BTreeSet<DefenderId>,
    /// 设置。
    pub settings: Settings,
}

impl Progress {
    /// 当前存档版本。
    pub const VERSION: u32 = 1;
}
