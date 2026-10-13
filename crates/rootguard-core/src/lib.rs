//! # rootguard-core
//!
//! 职责：Rootguard 的规则与模拟层——组件、资源、事件、FixedUpdate 系统、
//! 防御者 / 敌人 / 弹丸 / 关卡的数据模型（SPEC.md 第 4 ～ 6 章）。全部逻辑可在 headless 下测试。
//!
//! 依赖边界（SPEC.md 3.1 R1 / R2）：只依赖 `trellia-core`、`serde`、`ron`、`log`、`thiserror`；
//! 禁止出现任何呈现类型。渲染与音效由 `rootguard-app` 响应本 crate 发出的事件完成。
//!
//! 用法：添加 [`ContentPlugin`]、[`BattlePlugin`]、[`EconomyPlugin`]、[`WavePlugin`]，
//! 以 [`start_battle`] 开局，每个 FixedUpdate tick 调用一次 [`battle_tick`]（SPEC 6）。
//!
//! 不负责：资产文件读取（`rootguard-content`）、场景、UI、渲染、音频、存档（`rootguard-app`）。

#![forbid(unsafe_code)]

pub mod battle;
pub mod components;
pub mod data;
pub mod events;
pub mod grid;
pub mod ids;
pub mod registry;
pub mod resources;
pub mod snapshot;
pub mod systems;

pub use battle::{BattleError, end_battle, start_battle};
pub use grid::Grid;
pub use ids::{DefenderId, EnemyId, LevelId, ProjectileId, ShortStr};
pub use registry::Content;
pub use systems::battle_tick;

use trellia_core::{App, Plugin};

use events::{
    BattleEnded, DefenderPlaced, DefenderRemoved, DewCollected, DewSpawned, EnemyArmorBroken,
    EnemyDamaged, EnemyDied, EnemySpawned, HoseTriggered, ProjectileFired, ProjectileHit, UiAction,
    WaveStarted,
};
use resources::{DamageQueue, DeathLog, PendingOps};

/// SPEC 3.3 `ContentPlugin`：插入 `DefenderRegistry` / `ProjectileRegistry` / `EnemyRegistry` /
/// `LevelRegistry` / `Rules` / `Grid` 资源。内容由 `rootguard-content` 加载并校验。
pub struct ContentPlugin {
    content: Content,
}

impl ContentPlugin {
    /// 以已校验的内容创建。
    #[must_use]
    pub fn new(content: Content) -> Self {
        Self { content }
    }
}

impl Plugin for ContentPlugin {
    fn name(&self) -> &'static str {
        "ContentPlugin"
    }

    fn build(&self, app: &mut App) {
        let c = self.content.clone();
        app.insert_resource(c.defenders)
            .insert_resource(c.projectiles)
            .insert_resource(c.enemies)
            .insert_resource(c.levels)
            .insert_resource(c.rules)
            .insert_resource(Grid);
    }
}

/// SPEC 3.3 `BattlePlugin`：注册对局事件与 `UiAction`，插入内部队列资源。
///
/// FixedUpdate 系统不在此注册，由 `BattleScene::fixed_update` 调用 [`battle_tick`]（ADR-002）。
pub struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn name(&self) -> &'static str {
        "BattlePlugin"
    }

    fn build(&self, app: &mut App) {
        let events = &mut app.world_mut().events;
        events.register::<UiAction>();
        events.register::<DefenderPlaced>();
        events.register::<DefenderRemoved>();
        events.register::<EnemyDamaged>();
        events.register::<EnemyArmorBroken>();
        events.register::<EnemyDied>();
        events.register::<ProjectileFired>();
        events.register::<ProjectileHit>();
        events.register::<HoseTriggered>();
        events.register::<BattleEnded>();
        app.insert_resource(DamageQueue::default())
            .insert_resource(PendingOps::default())
            .insert_resource(DeathLog::default());
    }
}

/// SPEC 3.3 `EconomyPlugin`：注册 Dew 事件。
pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn name(&self) -> &'static str {
        "EconomyPlugin"
    }

    fn build(&self, app: &mut App) {
        let events = &mut app.world_mut().events;
        events.register::<DewSpawned>();
        events.register::<DewCollected>();
    }
}

/// SPEC 3.3 `WavePlugin`：注册波次事件。
pub struct WavePlugin;

impl Plugin for WavePlugin {
    fn name(&self) -> &'static str {
        "WavePlugin"
    }

    fn build(&self, app: &mut App) {
        let events = &mut app.world_mut().events;
        events.register::<EnemySpawned>();
        events.register::<WaveStarted>();
    }
}

/// 一次性添加四个核心插件（`ContentPlugin` → `BattlePlugin` → `EconomyPlugin` → `WavePlugin`）。
pub fn add_core_plugins(app: &mut App, content: Content) {
    app.add_plugin(ContentPlugin::new(content))
        .add_plugin(BattlePlugin)
        .add_plugin(EconomyPlugin)
        .add_plugin(WavePlugin);
}
