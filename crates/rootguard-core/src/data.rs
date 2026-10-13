//! 数据模型（SPEC 第 4 章）：与 `assets/data/**/*.ron` 一一对应的定义类型与 RON 解析。
//!
//! 本模块只做反序列化；字段取值范围与引用完整性的校验在 `rootguard-content`（SPEC 4.5）。

use serde::{Deserialize, Serialize};

use crate::ids::{DefenderId, EnemyId, LevelId, ProjectileId, ShortStr};

/// 以与引擎 `AssetServer::load_ron` 相同的选项（`IMPLICIT_SOME`）解析 RON 文本。
///
/// # Errors
///
/// 语法或结构不符时返回带位置的 [`ron::error::SpannedError`]。
pub fn parse_ron<T: serde::de::DeserializeOwned>(
    text: &str,
) -> Result<T, ron::error::SpannedError> {
    ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
        .from_str(text)
}

/// 占位渲染外观（SPEC 4.1 / 4.3 `placeholder`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceholderDef {
    /// sRGB 十六进制颜色，如 `0x4CAF50`。
    pub color: u32,
    /// 白色标签文字（≤ 23 字节）。
    pub label: ShortStr,
}

/// 弹丸占位外观（SPEC 4.2）。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectilePlaceholderDef {
    /// sRGB 十六进制颜色。
    pub color: u32,
    /// 半径（逻辑像素）。
    pub radius: f32,
}

/// 防御者放置方式（SPEC 4.1）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Placement {
    /// 只能放在空格。
    Ground,
    /// 只能放在已有非覆盖层防御者之上。
    Overlay,
    /// v1 内容未使用；按 `Ground` 处理（ADR-002）。
    Any,
}

/// 防御者行为（SPEC 4.1 / 7.2）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Behavior {
    /// SPEC 7.2 Shooter：有目标时每 `interval_ticks` 发射 `volley` 枚弹丸。
    Shooter {
        /// 发射间隔（tick）。
        interval_ticks: u32,
        /// 每次连发数。
        volley: u8,
        /// 连发间隔（tick）。
        volley_gap_ticks: u32,
        /// 弹丸 id。
        projectile: ProjectileId,
    },
    /// SPEC 7.2 Producer：每 `interval_ticks` 生产 `amount` Dew；每 `bonus_every` 次额外 `bonus_amount`。
    Producer {
        /// 生产间隔（tick）。
        interval_ticks: u32,
        /// 每次产量。
        amount: i32,
        /// 每产几次触发一次额外产量；0 表示无。
        #[serde(default)]
        bonus_every: u32,
        /// 额外产量。
        #[serde(default)]
        bonus_amount: i32,
    },
    /// SPEC 7.2 Wall：无主动行为。
    Wall,
    /// SPEC 7.2 Spikes：每 `interval_ticks` 对其格内全部敌人造成 `damage`。
    Spikes {
        /// 间隔（tick）。
        interval_ticks: u32,
        /// 伤害。
        damage: i32,
    },
    /// SPEC 7.2 Mine：放置 `arm_ticks` 后武装，敌人进入其格即爆炸（无视护甲）。
    Mine {
        /// 武装时间（tick）。
        arm_ticks: u32,
        /// 伤害。
        damage: i32,
    },
    /// SPEC 7.2 Blast：`fuse_ticks` 后对以其格为中心、半径 `radius_cells` 的方阵造成伤害（无视护甲）。
    Blast {
        /// 引信（tick）。
        fuse_ticks: u32,
        /// 伤害。
        damage: i32,
        /// 半径（格）；1 表示 3 × 3。
        radius_cells: u8,
    },
    /// SPEC 7.2 Overlay：覆盖在已有防御者之上优先承伤；hp < `num/den` × max 时可重复放置修复。
    Overlay {
        /// 修复阈值分子。
        repair_below_num: i32,
        /// 修复阈值分母。
        repair_below_den: i32,
    },
    /// SPEC 7.2 Chomper：吞食其格或右侧 `reach_cells` 格内的敌人，之后咀嚼 `chew_ticks`。
    Chomper {
        /// 咀嚼时间（tick）。
        chew_ticks: u32,
        /// 向右的额外格数。
        reach_cells: u8,
    },
    /// SPEC 7.2 Gust：`fuse_ticks` 后把所在车道全部敌人向右推 `push_px`。
    Gust {
        /// 引信（tick）。
        fuse_ticks: u32,
        /// 推距（逻辑像素）。
        push_px: f32,
    },
}

/// 防御者定义（SPEC 4.1，`defenders/*.defender.ron`）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Defender")]
pub struct DefenderDef {
    /// 全局唯一 id。
    pub id: DefenderId,
    /// 费用（Dew）。
    pub cost: i32,
    /// 卡牌冷却（tick）。
    pub cooldown_ticks: u32,
    /// 生命。
    pub hp: i32,
    /// 放置方式。
    pub placement: Placement,
    /// 行为列表。
    pub behaviors: Vec<Behavior>,
    /// 自由标签，如 `"spiky"`。
    #[serde(default)]
    pub tags: Vec<String>,
    /// 占位外观。
    pub placeholder: PlaceholderDef,
}

impl DefenderDef {
    /// 是否带有某标签。
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// 是否为覆盖层（含 `Overlay` 行为）。
    #[must_use]
    pub fn is_overlay(&self) -> bool {
        self.behaviors
            .iter()
            .any(|b| matches!(b, Behavior::Overlay { .. }))
    }
}

/// 弹丸命中时的状态效果（SPEC 4.2 / 8.2）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnHit {
    /// Chill：持续 `ticks`，重复命中刷新为该值、不累加。
    Chill {
        /// 持续时间（tick）。
        ticks: u32,
    },
}

/// 弹丸定义（SPEC 4.2，`projectiles/*.projectile.ron`）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Projectile")]
pub struct ProjectileDef {
    /// 全局唯一 id。
    pub id: ProjectileId,
    /// 速度（逻辑像素 / tick）。
    pub speed: f32,
    /// 伤害。
    pub damage: i32,
    /// 是否穿透。
    pub pierce: bool,
    /// 命中效果。
    #[serde(default)]
    pub on_hit: Vec<OnHit>,
    /// 占位外观。
    pub placeholder: ProjectilePlaceholderDef,
}

/// 护甲种类（SPEC 2.4 / 8.3）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArmorKind {
    /// 头盔：弹丸与 Spikes 先扣护甲。
    Helm,
    /// 门板：只挡弹丸。
    Shield,
    /// 鼓腹：弹丸与 Spikes 先扣护甲。
    Drum,
}

/// 护甲定义（SPEC 4.3 `armor`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArmorDef {
    /// 头盔。
    Helm {
        /// 护甲值。
        hp: i32,
    },
    /// 门板。
    Shield {
        /// 护甲值。
        hp: i32,
    },
    /// 鼓腹。
    Drum {
        /// 护甲值。
        hp: i32,
    },
}

impl ArmorDef {
    /// 种类与护甲值。
    #[must_use]
    pub fn split(self) -> (ArmorKind, i32) {
        match self {
            ArmorDef::Helm { hp } => (ArmorKind::Helm, hp),
            ArmorDef::Shield { hp } => (ArmorKind::Shield, hp),
            ArmorDef::Drum { hp } => (ArmorKind::Drum, hp),
        }
    }
}

/// 敌人特性（SPEC 4.3 `traits` / 8.1）。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Trait {
    /// 首次遇到防御者时用 `ticks` 跨越到其左侧格。
    Vault {
        /// 跨越耗时（tick，ADR-002）。
        ticks: u32,
    },
    /// 主生命不高于 `threshold_percent`% 时基础速度 ×`factor`（一次性）。
    Sprint {
        /// 阈值百分比。
        threshold_percent: i32,
        /// 速度倍率。
        factor: f32,
    },
    /// 死亡时在原地生成 `child`。
    Carrier {
        /// 衍生敌人 id。
        child: EnemyId,
    },
    /// 免疫吞食，Gust 推距按规则缩减。
    Heavy,
}

/// 敌人定义（SPEC 4.3，`enemies/*.enemy.ron`）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Enemy")]
pub struct EnemyDef {
    /// 全局唯一 id。
    pub id: EnemyId,
    /// 主生命。
    pub hp: i32,
    /// 速度（逻辑像素 / tick）。
    pub speed: f32,
    /// 每次啃食伤害。
    pub bite_damage: i32,
    /// 啃食间隔（tick）。
    pub bite_interval_ticks: u32,
    /// 护甲层。
    #[serde(default)]
    pub armor: Option<ArmorDef>,
    /// 特性。
    #[serde(default)]
    pub traits: Vec<Trait>,
    /// 威胁值（波次预算）。
    pub threat: u32,
    /// 占位外观。
    pub placeholder: PlaceholderDef,
}

impl EnemyDef {
    /// 主生命 + 护甲值（波次统计用，SPEC 10.1）。
    #[must_use]
    pub fn total_hp(&self) -> i32 {
        self.hp + self.armor.map_or(0, |a| a.split().1)
    }

    /// 是否为 Heavy。
    #[must_use]
    pub fn is_heavy(&self) -> bool {
        self.traits.iter().any(|t| matches!(t, Trait::Heavy))
    }
}

/// 卡组规则（SPEC 4.4 `cards`）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardRule {
    /// 固定卡组，跳过选卡。
    Forced(Vec<DefenderId>),
    /// 玩家从已解锁卡牌中选择。
    Choose,
}

/// 生成车道（SPEC 4.4 `lane`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaneSpec {
    /// 在激活车道中均匀随机。
    Random,
    /// 依次分配到不同激活车道，从随机起点轮转。
    Spread,
    /// 固定行。
    Fixed(u8),
}

/// 一组生成（SPEC 4.4 `spawns` 元素）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnDef {
    /// 敌人 id。
    pub enemy: EnemyId,
    /// 数量。
    pub count: u32,
    /// 车道分配。
    pub lane: LaneSpec,
}

/// 一波（SPEC 4.4 / 10.1）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "Wave")]
pub struct WaveDef {
    /// 距上一波（或关卡开始）的延迟（tick）。
    pub delay_ticks: u32,
    /// 是否为大波。
    pub flag: bool,
    /// 生成列表。
    pub spawns: Vec<SpawnDef>,
}

/// 关卡定义（SPEC 4.4，`levels/*.level.ron`）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "Level")]
pub struct LevelDef {
    /// 关卡 id，如 `1-3`。
    pub id: LevelId,
    /// 随机种子。
    pub seed: u64,
    /// 激活车道。
    pub lanes: Vec<u8>,
    /// 起始 Dew。
    pub start_dew: i32,
    /// 是否有天空 Dew。
    pub sky_dew: bool,
    /// 卡槽数。
    pub card_slots: u8,
    /// 卡组规则。
    pub cards: CardRule,
    /// 通关解锁的卡牌。
    #[serde(default)]
    pub unlock_on_win: Vec<DefenderId>,
    /// 波次。
    pub waves: Vec<WaveDef>,
}

/// 经济规则（SPEC 9）。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EconomyRules {
    /// Dew 上限。
    pub dew_cap: i32,
    /// 首次天空 Dew 的 tick。
    pub sky_first_ticks: u32,
    /// 天空 Dew 基础间隔。
    pub sky_interval_ticks: u32,
    /// 天空 Dew 间隔抖动上限（不含）。
    pub sky_interval_jitter_ticks: u32,
    /// 天空 Dew 数值。
    pub sky_amount: i32,
    /// 天空 Dew 下落速度（逻辑像素 / tick）。
    pub sky_fall_speed: f32,
    /// 落地后过期时间（tick）。
    pub dew_expire_ticks: u32,
}

/// 波次规则（SPEC 10.1）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaveRules {
    /// 当前波剩余 hp 不高于初始的此百分比时提前开始下一波。
    pub early_start_percent: i32,
    /// 两波生成的最低间隔（tick）。
    pub min_gap_ticks: u32,
    /// flag 波生成前的横幅时长（tick）。
    pub flag_banner_ticks: u32,
}

/// 状态效果规则（SPEC 8.2）。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StatusRules {
    /// Chill 速度倍率。
    pub chill_speed_factor: f32,
    /// Chill 啃食间隔倍率。
    pub chill_bite_interval_factor: u32,
}

/// 全局规则（`assets/data/rules.ron`，ADR-002）。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Rules")]
pub struct Rules {
    /// "准备"倒计时（tick，SPEC 6 第 1 步）。
    pub intro_ticks: u32,
    /// 经济。
    pub economy: EconomyRules,
    /// 波次。
    pub waves: WaveRules,
    /// 状态效果。
    pub status: StatusRules,
    /// Gust 对 Heavy 的推距倍率。
    pub heavy_push_factor: f32,
}
