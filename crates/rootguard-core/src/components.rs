//! 组件（SPEC 5.1）：全部为纯数据，`Copy` 或小 `Clone`。

use trellia_core::{Entity, Timer, Vec2};

use crate::data::ArmorKind;
use crate::ids::{DefenderId, EnemyId, ProjectileId, ShortStr};

/// 对局实体标记；退出对局时按它整体清理（SPEC 3.4）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BattleTag;

/// 所在车道（SPEC 5.1）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lane {
    /// 行号 0..5。
    pub row: u8,
}

/// 防御者所在格（SPEC 5.1）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cell {
    /// 行号 0..5。
    pub row: u8,
    /// 列号 0..9。
    pub col: u8,
}

/// 指向防御者注册表（SPEC 5.1）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefenderRef {
    /// 防御者 id。
    pub id: DefenderId,
}

/// 指向敌人注册表（SPEC 5.1）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyRef {
    /// 敌人 id。
    pub id: EnemyId,
}

/// 指向弹丸注册表（命中效果查询用）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectileRef {
    /// 弹丸 id。
    pub id: ProjectileId,
}

/// 生命（SPEC 2.4 / 5.1）。`hp ≤ 0` 即视为死亡，实体在该 tick 末移除。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Health {
    /// 当前生命。
    pub hp: i32,
    /// 最大生命。
    pub max: i32,
}

impl Health {
    /// 满血。
    #[must_use]
    pub fn full(hp: i32) -> Self {
        Self { hp, max: hp }
    }

    /// 是否存活。
    #[must_use]
    pub fn alive(&self) -> bool {
        self.hp > 0
    }
}

/// 护甲层（SPEC 5.1 / 8.3）。归零时移除并发 `EnemyArmorBroken`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Armor {
    /// 种类。
    pub kind: ArmorKind,
    /// 剩余护甲值。
    pub hp: i32,
    /// 初始护甲值（呈现用）。
    pub max: i32,
}

/// Gourdshell 指向宿主（SPEC 5.1 / 7.2 Overlay）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayOf {
    /// 宿主防御者。
    pub host: Entity,
}

/// 宿主指向 Gourdshell（SPEC 5.1 / 7.2 Overlay）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shielded {
    /// 覆盖层实体。
    pub overlay: Entity,
}

/// 常驻状态效果（SPEC 5.1 / 8.2）。速度与啃食间隔在读取时即时计算，不改写基础值。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatusEffects {
    /// Chill 剩余 tick。
    pub chill_ticks: u32,
    /// Stun 剩余 tick（v1 无来源，字段预留）。
    pub stun_ticks: u32,
}

impl StatusEffects {
    /// 是否处于 Chill。
    #[must_use]
    pub fn chilled(&self) -> bool {
        self.chill_ticks > 0
    }
}

/// 敌人行走状态（SPEC 5.1 `WalkState`）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WalkState {
    /// 向左行走。
    Walking,
    /// 停下啃食目标防御者（宿主；伤害优先落在其覆盖层上）。
    Eating(Entity),
    /// 跨越中：x 从 `from` 线性移动到 `to`，已经过 `t` tick。
    Vaulting {
        /// 起点 x。
        from: f32,
        /// 终点 x。
        to: f32,
        /// 已经过 tick。
        t: u32,
    },
    /// 被吞食（同 tick 死亡）。
    Swallowed,
}

/// 敌人移动（SPEC 5.1）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Walker {
    /// 基础速度（逻辑像素 / tick）；Sprint 触发时一次性改写。
    pub base_speed: f32,
    /// 状态。
    pub state: WalkState,
}

/// 啃食（SPEC 5.1 / 2.4）。`elapsed` 为当前啃食周期已经过的 tick。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Biter {
    /// 每次伤害。
    pub damage: i32,
    /// 基础间隔（tick）；Chill 时按规则倍增。
    pub interval: u32,
    /// 当前周期已经过 tick。
    pub elapsed: u32,
}

/// 射手（SPEC 5.1 / 7.2 Shooter）。`timer` 已结束即"就绪"。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shooter {
    /// 发射冷却（单次计时器；结束 = 就绪）。
    pub timer: Timer,
    /// 每次连发数。
    pub volley: u8,
    /// 本轮连发剩余枚数。
    pub volley_left: u8,
    /// 连发间隔计时器。
    pub volley_timer: Timer,
    /// 弹丸 id。
    pub projectile: ProjectileId,
}

/// 生产者（SPEC 5.1 / 7.2 Producer）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Producer {
    /// 生产周期计时器。
    pub timer: Timer,
    /// 每次产量。
    pub amount: i32,
    /// 每产几次触发一次额外产量；0 表示无。
    pub bonus_every: u32,
    /// 额外产量。
    pub bonus_amount: i32,
    /// 已生产次数。
    pub produced: u32,
}

/// 地刺（SPEC 5.1 / 7.2 Spikes）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spikes {
    /// 伤害周期计时器。
    pub timer: Timer,
    /// 伤害。
    pub damage: i32,
}

/// 地雷（SPEC 5.1 / 7.2 Mine）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mine {
    /// 武装计时器。
    pub arm: Timer,
    /// 是否已武装。
    pub armed: bool,
    /// 伤害。
    pub damage: i32,
}

/// 爆炸（SPEC 5.1 / 7.2 Blast）。引信期间可被啃食但不掉血。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blast {
    /// 引信计时器。
    pub fuse: Timer,
    /// 伤害。
    pub damage: i32,
    /// 半径（格）。
    pub radius_cells: u8,
}

/// 吞食者状态（SPEC 7.2 Chomper）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChompState {
    /// 可以吞食。
    Ready,
    /// 咀嚼中（`Chomper.chew` 计时）。
    Chewing,
}

/// 吞食者（SPEC 5.1 / 7.2 Chomper）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chomper {
    /// 状态。
    pub state: ChompState,
    /// 咀嚼计时器。
    pub chew: Timer,
    /// 向右的额外格数。
    pub reach_cells: u8,
}

/// 风推（SPEC 5.1 / 7.2 Gust）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gust {
    /// 引信计时器。
    pub fuse: Timer,
    /// 推距（逻辑像素）。
    pub push_px: f32,
}

/// 跨越者（SPEC 5.1 / 8.1 Vault）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vaulter {
    /// 是否已跨越过。
    pub used: bool,
    /// 跨越耗时（tick）。
    pub ticks: u32,
}

/// 疾行者（SPEC 5.1 / 8.1 Sprint）：主生命不高于阈值时一次性改写 `Walker.base_speed`。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprinter {
    /// 阈值百分比。
    pub threshold_percent: i32,
    /// 速度倍率。
    pub factor: f32,
    /// 是否已触发。
    pub triggered: bool,
}

/// 携带者（SPEC 5.1 / 8.1 Carrier）：死亡时原地生成 `child`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Carrier {
    /// 衍生敌人 id。
    pub child: EnemyId,
}

/// 敌人不会停下啃食的防御者（SPEC 7.2 Spikes，由 `spiky` 标签导出）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NotBlocking;

/// 重型（SPEC 5.1 / 8.1 Heavy）：免疫吞食，Gust 推距按规则缩减。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Heavy;

/// 敌人所属波次（波次剩余 hp 统计用，SPEC 10.1）；衍生敌人不带此组件。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpawnedInWave(pub u16);

/// 弹丸（SPEC 5.1）。
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectileBody {
    /// 伤害。
    pub damage: i32,
    /// 速度（逻辑像素 / tick）。
    pub speed: f32,
    /// 是否穿透。
    pub pierce: bool,
    /// 生成时 x（只命中 `x ≥ origin_x` 的敌人，SPEC 2.5）。
    pub origin_x: f32,
    /// 已命中的敌人（穿透弹丸不重复命中）。
    pub hit: Vec<Entity>,
}

/// 可收集的 Dew（SPEC 5.1 / 9）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DewPickup {
    /// 数值。
    pub amount: i32,
    /// 落点 y。
    pub land_y: f32,
    /// 落地后的过期计时器。
    pub expire: Timer,
    /// 是否已落地。
    pub landed: bool,
}

/// 占位渲染外观（SPEC 5.1 / 12.2）。颜色为 sRGB 十六进制，由 `rootguard-app` 转换。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placeholder {
    /// sRGB 十六进制颜色。
    pub color: u32,
    /// 标签文字。
    pub label: ShortStr,
    /// 尺寸（以 `Transform2D.pos` 为中心）。
    pub size: Vec2,
}
