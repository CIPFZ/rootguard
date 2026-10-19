# Rootguard 游戏规格说明书（v1）

> 状态：v1.3，2026-10-19（v1.3：12.1 默认渲染模式改为 Skinned；v1.1：10.2 预算上限、10.3 脚本解质量、8.1 Sprint 阈值、解锁顺序；v1.2：10.3 适用范围澄清、16.2 `audio_events_mapped` 允许交叉淡化、新增第 20 章 Phase 1 收尾与难度校准）
> 适用范围：Rootguard Phase 1（白天花园，10 关）
> 读者：负责从 0 实现游戏的 AI 开发者与项目负责人
> 前置：Trellia 引擎 v1（https://github.com/CIPFZ/trellia，M5 已验收）
> 配套文档：`AGENT.md`（AI 开发者工作手册）

---

## 0. 阅读指南与硬性约定

关键词含义：**必须（MUST）** 不满足即不合格；**禁止（MUST NOT）** 出现即不合格；**应当（SHOULD）** 默认遵守，偏离需 ADR；**可以（MAY）** 可选。

全局约定：

1. 本文档是游戏需求的唯一来源。引擎的行为以引擎仓库的 `SPEC.md` 与 `docs/decisions/ADR-*.md` 为准，本文档第 15 章列出的"引擎补充需求"是唯一允许修改引擎的理由。
2. 所有数值以本文档第 7 ～ 10 章的表格为准，并同步落入 RON 数据文件。代码中禁止出现与表格重复的魔法数字。
3. 第三方依赖：继承引擎白名单，游戏层额外允许 `directories`（存档目录）。新增其他依赖必须 ADR。
4. 所有 crate `#![forbid(unsafe_code)]`；CI 同引擎（fmt、clippy `-D warnings`、test、doc）。
5. 自然语言中文，标识符英文。游戏内文本通过 `i18n` 表（第 11.6 节）管理，默认简体中文，备用英文。

---

## 1. 定位、边界与知识产权

### 1.1 一句话定位

Rootguard 是一款原创的车道式植物塔防游戏：玩家在 5 行 × 9 列的花园里放置各类防御植物，抵御从右侧涌来的 Husk（枯壳怪），守住左侧的温室。玩法机制属于塔防类型的通用设计，角色、名称、美术、音乐、文案全部原创。

### 1.2 知识产权硬约束

1. 禁止使用任何第三方游戏的角色名、角色造型、图标、音乐、音效、字体、UI 风格或文案，也禁止"仿制"（改色、镜像、像素化等）。
2. 本文档中的所有名称（Rootguard、Husk、Dew、Dewcup、Pebble Pod 等）为本项目原创。实现者在新增内容时同样必须原创命名，禁止使用业界知名角色名的直译或谐音。
3. 美术方向（第 12 章）必须与任何既有商业作品有显著区分：扁平几何风，所有角色由基本形状组合而成。
4. 音频必须自制（合成器 / 程序化生成）或 CC0 素材，逐个登记在 `assets/SOURCES.md`。
5. 实现者在任何阶段若不确定某项内容是否越界，按越界处理并换一个方案。

### 1.3 Phase 1 范围

| 包含 | 不包含（Phase 2+，仅第 18 章记录） |
|---|---|
| 白天花园场景，5 行 × 9 列 | 夜晚、水域、屋顶、迷雾等场景 |
| 12 种防御者、8 种敌人（+1 种衍生） | 更多角色、Boss |
| 冒险模式 10 关，线性解锁 | 生存、小游戏、无尽 |
| 选卡（卡槽 6 ～ 8） | 商店、货币、升级 |
| Dew 经济、Hose Burst 车道保险 | 图鉴、成就 |
| 进度存档、音量设置 | 云存档、多存档位 |
| 占位渲染模式 + 原创皮肤模式 | 动态天气、昼夜 |
| 中文 / 英文 | 其他语言 |

---

## 2. 玩法总则

### 2.1 核心循环

1. 关卡开始，玩家持有若干 Dew，卡槽内有若干防御者卡牌。
2. 天空周期性掉落 Dew；Dewcup 定期生产 Dew。点击收集。
3. 消耗 Dew 选择卡牌并点击格子放置防御者；卡牌进入冷却。
4. Husk 按波次从右侧生成，沿车道向左行走，遇到防御者停下啃食。
5. 防御者按各自规则攻击、阻挡、生产。
6. Husk 到达车道最左端时触发该车道的 Hose Burst（一次性清场）；若已用过则失败。
7. 所有波次生成完毕且场上无 Husk 时胜利，解锁下一关与新卡牌。

### 2.2 空间定义

逻辑分辨率 1280 × 720（引擎 `GfxPlugin` 固定）。

| 项 | 值 |
|---|---|
| 网格原点（左上） | (260, 90) |
| 格子尺寸 | 100 × 115 |
| 列中心 x | 310 + 100 × col，col ∈ [0, 8] |
| 行中心 y | 147.5 + 115 × row，row ∈ [0, 4] |
| 行底部 y（sort_y） | 205 + 115 × row |
| 网格右边界 | x = 1160 |
| 敌人生成 x | 1240 + `GameRng.gen_range(0..40)` |
| 敌人失败线 | x < 250 |
| Hose Burst 停放 x | 235 |
| 顶部 HUD 区域 | y ∈ [0, 80] |
| 底部提示区域 | y ∈ [680, 720] |

网格之外的坐标仅用于 HUD 与敌人入场。不激活的车道（第 10 章 `lanes`）绘制为暗色且不可放置、不生成敌人。

### 2.3 时间定义

- 固定 60 tick/s（引擎）。本文档一切时长单位为 tick，括号内的秒数仅供阅读。
- 关卡内时间 `level_tick` 从关卡开始计 0，暂停不计。
- 速度单位：逻辑像素 / tick。

### 2.4 伤害与生命模型

- 所有生命、护甲、伤害为整数（`i32`）。
- 敌人有主生命 `hp`，可选护甲层（Helm、Shield、Drum，第 8 章）。伤害按"护甲层 → 主生命"顺序结算；每种伤害来源在第 8.3 节标明是否无视护甲。
- 防御者有生命 `hp`；Gourdshell 作为覆盖层优先承伤。
- 啃食：敌人停在防御者所在格右边界时，每 `bite_interval` tick 对其造成 `bite_damage`。
- 死亡在伤害结算的同一 tick 判定；死亡实体在该 tick 末通过 `commands.despawn` 移除，死亡事件同 tick 发出。

### 2.5 车道与碰撞

一切碰撞为同车道一维区间判定：

- 防御者占据 `[cell_left, cell_right]`（宽 100）。
- 敌人碰撞区间 `[x - 20, x + 20]`。
- 弹丸碰撞区间 `[x - 6, x + 6]`，命中条件：敌人区间与弹丸区间相交且敌人 `x ≥ 弹丸起始 x`。
- 敌人"到达"防御者：敌人 `x - 20 ≤ cell_right` 且敌人 `x > cell_left`。
- 同一 tick 多个候选目标时，取 `x` 最小者（最靠左），再按 `Entity` id 升序。

---

## 3. 架构

### 3.1 仓库与 crate

游戏为独立仓库 `rootguard/`，workspace 包含：

| crate | 职责 | 允许依赖 |
|---|---|---|
| `rootguard-core` | 规则、组件、系统、关卡数据模型、模拟层（headless 可测） | `trellia::core`（即 `trellia-core`）、`serde`、`ron`、`log`、`thiserror` |
| `rootguard-content` | 内置内容：防御者 / 敌人 / 关卡 RON 的加载、校验与注册表 | `rootguard-core`、`trellia-assets` |
| `rootguard-app` | 场景、UI、渲染系统、音频触发、存档、入口 `main.rs` | 全部 trellia crate、`rootguard-core`、`rootguard-content`、`directories` |

依赖规则（CI 以 `scripts/check_deps.sh` 检查，复用引擎脚本结构）：

- R1 `rootguard-core` 禁止依赖 `trellia-gfx` / `trellia-audio` / `trellia-scene` / `egui` / `eframe`。
- R2 `rootguard-core` 中禁止出现 `Sprite`、`DrawList`、`AudioQueue` 等呈现类型；渲染与音效通过事件（第 6.4 节）在 `rootguard-app` 中响应。
- R3 引擎依赖通过 `[workspace.dependencies] trellia = { git = "https://github.com/CIPFZ/trellia", rev = "<sha>" }` 锁定。禁止 path 依赖、禁止 fork 后改源码。

### 3.2 Trellia 引擎用法速查（来自引擎实际 API，ADR-001 ～ 007）

- 入口：`trellia::prelude::*`，`run_egui(app, RunConfig)` / `run_headless(app)`。
- 插件顺序必须为：`AssetsPlugin` → `GfxPlugin::new(Vec2::new(1280.0, 720.0))` → `AudioPlugin` → 游戏内容插件 → `ScenePlugin::new(initial)` → `DevtoolsPlugin`（可选 feature）。原因见引擎报告第 3.7 条。
- 系统签名 `fn(&mut World)`，通过 `app.add_system(Stage, system_fn)` 注册；`World` 字段 `entities` / `resources` / `events` / `commands` 公开。
- 启动期错误：`app.record_startup_error(err)`，后端在首帧前检查。
- 场景：`trait Scene: Send + Sync + 'static`，方法 `on_enter / on_exit / on_pause / on_resume / fixed_update / update / draw / ui`；切换通过 `SceneCommands` 资源。UI 中发出的命令下一帧生效。
- 退出：置 `AppExit.requested = true`。
- 资产加载需临时 `remove` 再 `insert` 多个 `Assets<T>`（引擎报告第 3.8 条，第 15 章 E-2 要求补一个辅助函数）。
- 形状命令 `sort_y` 恒为 0（引擎报告第 3.9 条），第 15 章 E-3 要求补齐；在 E-3 落地前占位渲染按行分层（第 12.2 节）。
- headless 不能点击 egui 控件（引擎报告第 3.5 条）；本游戏所有 UI 动作必须同时可由 `UiAction` 事件触发（第 11.1 节）。

### 3.3 插件划分（`rootguard-core` 提供）

| Plugin | 注册内容 |
|---|---|
| `ContentPlugin` | `DefenderRegistry`、`EnemyRegistry`、`LevelRegistry` 资源（由 `rootguard-content` 加载后插入） |
| `BattlePlugin` | 全部对局事件、`BattleState` 资源模板、FixedUpdate 系统（第 6 章顺序） |
| `EconomyPlugin` | Dew 掉落、生产、收集、卡牌冷却 |
| `WavePlugin` | 波次调度、生成、胜负判定 |

`rootguard-app` 提供 `RenderPlugin`（占位 / 皮肤渲染）、`SfxPlugin`（事件 → `AudioQueue`）、`SavePlugin`、`UiPlugin`。

### 3.4 场景

| Scene | 说明 | 覆盖层语义 |
|---|---|---|
| `TitleScene` | 标题，开始 / 继续 / 设置 / 退出 | 栈底 |
| `LevelSelectScene` | 10 关列表，显示解锁状态 | replace |
| `SeedSelectScene` | 选卡（1-4 起），显示已解锁卡牌 | replace |
| `BattleScene` | 对局 | replace；拥有全部对局实体 |
| `PauseScene` | 暂停菜单 | push；`blocks_update_below = true`，`blocks_draw_below = false` |
| `OutcomeScene` | 胜利 / 失败，下一关 / 重试 / 返回 | push，同上 |
| `SettingsScene` | 音量、语言、皮肤 / 占位切换 | push |

对局实体在 `BattleScene::on_exit` 中全部 despawn（通过 `BattleTag` 组件查询）。

---

## 4. 数据格式（RON）

全部位于 `assets/data/`。启动时由 `rootguard-content` 全量加载并校验，任一失败即 `record_startup_error`。

### 4.1 防御者 `defenders/*.defender.ron`

```ron
Defender(
    id: "pebble_pod",
    cost: 100,
    cooldown_ticks: 450,
    hp: 6,
    placement: Ground,            // Ground | Overlay（可放在已有防御者上）| Any
    behaviors: [
        Shooter(
            interval_ticks: 85,
            volley: 1,
            volley_gap_ticks: 0,
            projectile: "pebble",
        ),
    ],
    tags: [],                     // 自由字符串，供敌人特殊规则匹配，如 "spiky"
    placeholder: (color: 0x4CAF50, label: "P"),
)
```

`behaviors` 枚举（第 7.2 节详述）：`Shooter`、`Producer`、`Wall`、`Spikes`、`Mine`、`Blast`、`Overlay`、`Chomper`、`Gust`。一个防御者可组合多个，但 v1 内容中每个只用 1 个（Twinpod 用 `Shooter { volley: 2 }`）。

### 4.2 弹丸 `projectiles/*.projectile.ron`

```ron
Projectile(
    id: "frost_pebble",
    speed: 5.0,
    damage: 1,
    pierce: false,
    on_hit: [Chill(ticks: 600)],  // 可选状态效果
    placeholder: (color: 0x90CAF9, radius: 7.0),
)
```

### 4.3 敌人 `enemies/*.enemy.ron`

```ron
Enemy(
    id: "clay_husk",
    hp: 10,
    speed: 0.30,
    bite_damage: 1,
    bite_interval_ticks: 60,
    armor: Some(Helm(hp: 28)),    // None | Helm | Shield | Drum
    traits: [],                   // Vault | Sprint | Carrier(child: "sproutling") | Heavy
    threat: 2,                    // 波次强度计算用
    placeholder: (color: 0xA1887F, label: "C"),
)
```

### 4.4 关卡 `levels/*.level.ron`

```ron
Level(
    id: "1-3",
    seed: 103,
    lanes: [1, 2, 3],
    start_dew: 50,
    sky_dew: true,
    card_slots: 6,
    cards: Forced(["pebble_pod", "dewcup", "barkwall"]),   // 或 Choose
    unlock_on_win: ["frostpod"],
    waves: [
        Wave(delay_ticks: 1500, flag: false, spawns: [(enemy: "husk", count: 1, lane: Random)]),
        Wave(delay_ticks: 1500, flag: false, spawns: [(enemy: "husk", count: 2, lane: Random)]),
        Wave(delay_ticks: 1200, flag: true,  spawns: [(enemy: "husk", count: 3, lane: Spread), (enemy: "clay_husk", count: 1, lane: Random)]),
    ],
)
```

`lane` 枚举：`Random`（在激活车道中均匀随机）、`Spread`（count 个依次分配到不同激活车道，从随机起点轮转）、`Fixed(row)`。

### 4.5 校验规则

- 所有 id 全局唯一且仅含 `[a-z0-9_]`。
- 关卡引用的敌人 / 卡牌 id 必须存在；`Fixed(row)` 必须在 `lanes` 内；`waves` 非空；`flag` 波至少 1 个且最后一波必须为 `flag`。
- `card_slots ∈ [4, 8]`；`Forced` 列表长度 ≤ `card_slots`。
- 数值下限：`hp ≥ 1`、`speed > 0`、`cost ≥ 0`、`cooldown_ticks ≥ 1`、`interval_ticks ≥ 1`。
- 校验失败信息必须包含文件路径与字段名。

---

## 5. 组件与资源（`rootguard-core`）

### 5.1 组件

| 组件 | 字段 | 说明 |
|---|---|---|
| `BattleTag` | — | 对局实体标记，退出对局时整体清理 |
| `Lane` | `row: u8` | 所在车道 |
| `Cell` | `row: u8, col: u8` | 防御者所在格 |
| `Transform2D` | 引擎 | 位置（敌人 / 弹丸 / Dew 用；防御者由 `Cell` 推导并写入以便渲染） |
| `DefenderRef` | `id: DefenderId` | 指向注册表 |
| `EnemyRef` | `id: EnemyId` | 同上 |
| `Health` | `hp: i32, max: i32` | |
| `Armor` | `kind: ArmorKind, hp: i32` | Helm / Shield / Drum；破损时 `commands.remove::<Armor>` |
| `OverlayOf` | `host: Entity` | Gourdshell 指向宿主 |
| `Shielded` | `overlay: Entity` | 宿主指向 Gourdshell |
| `StatusEffects` | `chill_ticks: u32, stun_ticks: u32` | 常驻；`chill` 速度与啃食间隔减半效果 |
| `Walker` | `base_speed: f32, state: WalkState` | `Walking` / `Eating(target)` / `Vaulting{from,to,t}` / `Swallowed` |
| `Biter` | `damage: i32, interval: u32, timer: Timer` | |
| `Shooter` | `timer: Timer, volley_left: u8, volley_timer: Timer, projectile: ProjectileId` | |
| `Producer` | `timer: Timer, amount: i32` | |
| `Spikes` | `timer: Timer, damage: i32` | |
| `Mine` | `arm: Timer, armed: bool, damage: i32` | |
| `Blast` | `fuse: Timer, damage: i32, radius_cells: u8` | |
| `Chomper` | `state: ChompState, chew: Timer, bite: Timer` | |
| `Gust` | `fuse: Timer, push_px: f32` | |
| `Vaulter` | `used: bool` | |
| `Sprinter` | — | 标记：生命 ≤ 50% 时速度 ×1.5（一次性，由 `Walker.base_speed` 改写） |
| `Carrier` | `child: EnemyId` | |
| `Heavy` | — | 免疫吞食；Gust 推距减半 |
| `ProjectileBody` | `damage: i32, speed: f32, pierce: bool, origin_x: f32, hit: SmallVec<Entity>`（无 smallvec 时用 `Vec`） | |
| `DewPickup` | `amount: i32, land_y: f32, expire: Timer, landed: bool` | |
| `Placeholder` | `color: Color, label: &'static str, size: Vec2` | 占位渲染（`Color` 来自 `trellia_core` 不可用时，用 `u32` hex，由 app 转换） |

组件均为纯数据，`Copy` 或小 `Clone`，`StatusEffects` 常驻而非加减组件（引擎 SPEC 4.3.4）。

### 5.2 资源

| 资源 | 内容 |
|---|---|
| `BattleState` | `level: LevelId, level_tick: u64, dew: i32, phase: Intro / Playing / Won / Lost, selected_card: Option<usize>, shovel_mode: bool` |
| `CardTray` | `cards: Vec<CardSlot { defender: DefenderId, cooldown: Timer, ready: bool }>` |
| `WaveState` | `next_wave: usize, wave_timer: Timer, alive_from_wave: i32（当前波生成的总 hp 与剩余 hp）, all_spawned: bool, banner: Option<Timer>` |
| `LaneState` | `hose_available: [bool; 5], active: [bool; 5]` |
| `Grid` | 纯函数集合：`cell_center(row, col)`, `cell_at(pos) -> Option<(row, col)>`, `lane_at_y`, `sort_y(row)` |
| `DefenderRegistry` / `EnemyRegistry` / `ProjectileRegistry` / `LevelRegistry` | 由 RON 加载的只读表，id → 定义 |
| `Progress` | 已通关关卡集合、已解锁卡牌集合、设置（音量、语言、皮肤） |
| `UiActions` | `Events<UiAction>` 的别名用法见 11.1 |

### 5.3 事件（纯数据，`Clone + Debug`）

| 事件 | 发出者 | 消费者 |
|---|---|---|
| `DefenderPlaced { entity, id, cell }` | 放置系统 | 音效、渲染初始化 |
| `DefenderRemoved { entity, id, cell, cause: Shovel / Killed / Consumed }` | 多处 | 音效、特效 |
| `EnemySpawned { entity, id, lane }` | 波次系统 | 渲染初始化 |
| `EnemyDamaged { entity, amount, source: Projectile / Spikes / Mine / Blast / Chomp / Hose }` | 伤害系统 | 受伤闪白、音效 |
| `EnemyArmorBroken { entity, kind }` | 伤害系统 | 音效、换皮肤帧 |
| `EnemyDied { entity, id, lane, x }` | 伤害系统 | Carrier 衍生、波次统计、音效 |
| `ProjectileFired { entity, from, id }` | 射手系统 | 音效 |
| `ProjectileHit { entity, target }` | 碰撞系统 | 特效、音效 |
| `DewSpawned { entity, source: Sky / Producer(Entity) }` | 经济系统 | 音效 |
| `DewCollected { amount }` | 经济系统 | 音效、HUD 动画 |
| `WaveStarted { index, flag }` | 波次系统 | 横幅、音效 |
| `HoseTriggered { lane }` | 车道系统 | 音效、特效 |
| `BattleEnded { outcome: Won / Lost }` | 波次系统 | 场景切换、存档 |
| `UiAction` | UI / 测试 | 对应处理系统（11.1） |

---

## 6. 模拟系统（FixedUpdate 顺序）

`BattleScene::fixed_update` 中按以下固定顺序调用（或由 `BattlePlugin` 以此顺序注册；二选一，不可混用）。只有 `BattleState.phase == Playing` 时执行第 2 ～ 14 步。

| # | 系统 | 职责 |
|---|---|---|
| 1 | `intro_system` | `phase == Intro` 时倒计时 180 tick（显示"准备"横幅）后进入 `Playing` |
| 2 | `ui_action_system` | 消费 `UiAction`：选卡、取消、铲子、放置请求、暂停请求 |
| 3 | `placement_system` | 校验并执行放置 / 铲除，扣 Dew，启动卡牌冷却，发 `DefenderPlaced / Removed` |
| 4 | `economy_system` | 天空 Dew 掉落计时、Producer 计时生产、Dew 下落与过期、点击收集、卡牌冷却推进 |
| 5 | `wave_system` | 波次计时与生成（含 `Spread / Random` 车道分配）、波次横幅计时 |
| 6 | `status_system` | `StatusEffects` 计时递减 |
| 7 | `walker_system` | 敌人状态机：行走 / 啃食 / 跨越 / 被吞；碰撞判定"到达防御者"；Sprinter 触发 |
| 8 | `defender_behavior_system` | Shooter / Spikes / Mine / Blast / Chomper / Gust 的计时与触发，发 `ProjectileFired` 等；Mine 与 Blast 直接产生伤害请求 |
| 9 | `projectile_system` | 弹丸位移、越界销毁、命中判定，生成伤害请求 |
| 10 | `bite_system` | 啃食计时，对防御者（优先 Gourdshell）产生伤害请求 |
| 11 | `damage_system` | 统一结算本 tick 全部伤害请求（内部 `Vec<DamageRequest>` 资源，按 `(target, seq)` 顺序）：护甲 → 生命；发 `EnemyDamaged / ArmorBroken / Died`、`DefenderRemoved` |
| 12 | `death_system` | Carrier 衍生子敌人；Chomper 吞食完成；despawn 命令 |
| 13 | `lane_defense_system` | 敌人越过失败线：Hose Burst 可用则触发（该车道全部敌人立即死亡，`source: Hose`），否则 `phase = Lost` |
| 14 | `outcome_system` | `all_spawned && 无存活敌人 && 无待衍生` → `phase = Won`；发 `BattleEnded` |

伤害请求模型：`DamageRequest { target: Entity, amount: i32, source, ignore_armor: bool, seq: u32 }`，由第 8 ～ 10 步 push，第 11 步统一结算后清空。这样同一 tick 内多来源伤害顺序确定。

---

## 7. 防御者（12 种）

### 7.1 总表

| id | 名称（中 / 英） | 费用 | 冷却 | 生命 | 行为 | 解锁 |
|---|---|---|---|---|---|---|
| `pebble_pod` | 石籽荚 / Pebble Pod | 100 | 450 (7.5s) | 6 | Shooter：每 85 tick 发射 `pebble` | 初始 |
| `dewcup` | 露杯 / Dewcup | 50 | 450 | 6 | Producer：每 1440 tick (24s) 产 25 Dew | 1-1 |
| `barkwall` | 树皮墙 / Barkwall | 50 | 1800 (30s) | 72 | Wall | 1-2 |
| `frostpod` | 霜籽荚 / Frostpod | 175 | 450 | 6 | Shooter：每 85 tick 发射 `frost_pebble`（命中 Chill 600 tick） | 1-3 |
| `twinpod` | 双籽荚 / Twinpod | 200 | 450 | 6 | Shooter：每 85 tick 连发 2 枚 `pebble`，间隔 6 tick | 1-4 |
| `thornmat` | 荆棘垫 / Thornmat | 100 | 450 | 不可被啃（敌人直接走过） | Spikes：每 60 tick 对其格内所有敌人造成 1 伤害 | 1-5 |
| `tuberblast` | 块茎雷 / Tuberblast | 25 | 1800 | 6 | Mine：放置后 900 tick (15s) 武装；武装后敌人进入其格时爆炸，该格内敌人受 90 伤害（无视护甲），自身消耗 | 1-6 |
| `emberbulb` | 炽球 / Emberbulb | 150 | 3000 (50s) | 6 | Blast：放置后 60 tick 引爆，以其格为中心 3 × 3 格内敌人受 90 伤害（无视护甲），自身消耗 | 1-7 |
| `gourdshell` | 葫芦壳 / Gourdshell | 125 | 1800 | 80 | Overlay：放在已有防御者上，优先承伤；可重复放置以修复（仅当 hp < 2/3 max） | 1-8 |
| `snapmaw` | 咬夹草 / Snapmaw | 150 | 450 | 6 | Chomper：敌人进入其格或右侧相邻格时吞食（Heavy 免疫），咀嚼 2520 tick (42s) 期间不可再吞；吞食对目标即死 | 1-9 |
| `gustbloom` | 风铃花 / Gustbloom | 100 | 1800 | 6 | Gust：放置后 60 tick 引爆，将其所在车道全部敌人向右推 150 px（Heavy 减半），自身消耗 | 1-10（Phase 1 收官奖励） |
| `sunlantern` | 日灯笼 / Sunlantern | 125 | 450 | 6 | Producer：每 1440 tick 产 25 Dew，且每产 4 次额外 +25（简化的"成长"） | 1-9 |

解锁含义：通关该关后进入可选卡池。1-1 ～ 1-3 为 `Forced` 卡组教学关。

### 7.2 行为定义

- **Shooter**：若同车道右侧（`enemy.x > cell_right - 10`）且 `x ≤ 1240` 存在存活敌人，则计时推进并在触发时生成弹丸；无目标时计时保持在"就绪"状态（即一旦出现目标立即发射，之后按间隔）。弹丸生成位置 `(cell_center.x + 30, cell_center.y - 20)`。`volley > 1` 时后续弹丸按 `volley_gap_ticks` 顺序发射。
- **Producer**：计时到期生成 `DewPickup` 于其格附近（偏移 `GameRng` 在 ±30 px 内），`landed = true`，过期 900 tick。
- **Wall**：无主动行为。
- **Spikes**：对其格区间内所有敌人每 `interval` 造成伤害，无视护甲为 false；敌人不停下啃食它（`walker_system` 判定"到达"时忽略 `tags` 含 `spiky` 的防御者）。
- **Mine**：`arm` 到期前被啃食会正常死亡；武装后敌人区间与其格相交即触发。
- **Blast**：引信期间可被啃食但不会死亡（hp 不扣）；到期结算。
- **Overlay**：放置目标格必须有非 Overlay 防御者且无 Gourdshell 或 Gourdshell hp < 2/3；放置即建立 `OverlayOf / Shielded` 双向引用；宿主被铲除时 Gourdshell 一并移除。
- **Chomper**：状态 `Ready → Chewing(timer) → Ready`；吞食目标为"进入其格或右侧相邻格"且非 Heavy 的 x 最小敌人；目标置 `Swallowed` 并在同 tick 死亡。
- **Gust**：推送通过修改 `Transform2D.pos.x += push`（并同步 `prev_pos`，避免插值拖影）；越过生成 x 的敌人钳到 1260。

---

## 8. 敌人（8 种 + 1 衍生）

### 8.1 总表

| id | 名称 | 生命 | 护甲 | 速度 | 啃食 | 特性 | 威胁 | 首次出现 |
|---|---|---|---|---|---|---|---|---|
| `husk` | 枯壳怪 / Husk | 10 | — | 0.30 | 1 / 60 tick | — | 1 | 1-1 |
| `twig_husk` | 枝冠枯壳 / Twig Husk | 10 | Helm 10 | 0.30 | 1 / 60 | — | 2 | 1-2 |
| `clay_husk` | 陶盔枯壳 / Clay Husk | 10 | Helm 28 | 0.30 | 1 / 60 | — | 4 | 1-4 |
| `sprinter_husk` | 疾行枯壳 / Sprinter Husk | 10 | — | 0.45 | 1 / 60 | Sprint（hp ≤ 50% 时速度 ×1.5，即 hp ≤ 5） | 2 | 1-5 |
| `vault_husk` | 跳栏枯壳 / Vault Husk | 17 | — | 0.50 | 1 / 60 | Vault（首次遇到防御者时跨越到其左侧格；Snapmaw 吞食优先于跨越） | 3 | 1-6 |
| `shield_husk` | 门板枯壳 / Shield Husk | 10 | Shield 55（仅正面弹丸；Spikes / Mine / Blast 直接打主生命） | 0.30 | 1 / 60 | — | 4 | 1-7 |
| `carrier_husk` | 背篓枯壳 / Carrier Husk | 20 | — | 0.25 | 1 / 60 | Carrier：死亡时在原地生成 `sproutling` | 3 | 1-8 |
| `drum_husk` | 鼓腹枯壳 / Drum Husk | 30 | Drum 60 | 0.22 | 2 / 60 | Heavy | 6 | 1-9 |
| `sproutling` | 小芽怪 / Sproutling（衍生） | 5 | — | 0.60 | 1 / 60 | — | 1 | 随 carrier |

### 8.2 状态效果

| 效果 | 来源 | 效果 | 时长 | 叠加 |
|---|---|---|---|---|
| Chill | `frost_pebble` 命中 | 速度 ×0.5，啃食间隔 ×2 | 600 tick | 刷新为 600，不累加 |
| Stun | （v1 无来源，字段预留给 Phase 2 数据，不实现逻辑） | — | — | — |

Chill 下的速度与间隔计算在 `walker_system` / `bite_system` 读取 `StatusEffects` 时即时计算，不改写基础值。

### 8.3 伤害与护甲规则

| 来源 | 对 Helm | 对 Shield | 对 Drum |
|---|---|---|---|
| 弹丸 | 先扣护甲 | 先扣护甲 | 先扣护甲 |
| Spikes | 先扣护甲 | 无视（直接主生命） | 先扣护甲 |
| Mine / Blast | 无视 | 无视 | 无视 |
| Chomper | 即死（Heavy 免疫） | 即死 | 免疫 |
| Hose | 即死 | 即死 | 即死 |

护甲归零时 `commands.remove::<Armor>` 并发 `EnemyArmorBroken`。Shield 的"仅正面"在 v1 中等价于"弹丸"，因为所有弹丸都从左向右飞。

---

## 9. 经济

| 项 | 值 |
|---|---|
| 关卡起始 Dew | `Level.start_dew`（教学关 50 ～ 150，常规 50） |
| 天空 Dew | `sky_dew == true` 时，首次 420 tick 后掉落，之后每 `570 + gen_range(0..180)` tick 一次，数值 25，落点随机激活格上方（x 在网格列范围内，从 y = -40 落到目标格中心 y，速度 1.0 px/tick），落地后 900 tick 过期 |
| Dewcup / Sunlantern | 见第 7 章 |
| 收集 | 点击 `DewPickup` 碰撞圆（半径 36）即 `dew += amount`，实体立即移除，发 `DewCollected`；收集后的飘向 HUD 动画为纯视觉（`rootguard-app` 用 Tween 实现），不影响模拟 |
| Dew 上限 | 9990 |
| 卡牌状态 | `ready = cooldown.finished() && dew ≥ cost`；放置后 `cooldown.reset()`，关卡开始时所有卡牌冷却完成 |
| 铲子 | 免费；点击格子移除该格顶层（Gourdshell 优先，否则宿主）；不退 Dew |

---

## 10. 波次与关卡

### 10.1 波次调度

- `wave_timer` 从关卡 `Playing` 起计 `waves[0].delay_ticks`。
- 每波生成完毕后，下一波计时 `delay_ticks`；但若当前波生成的敌人剩余总 hp ≤ 该波初始总 hp 的 40%，立即（提前）开始下一波（最低间隔 300 tick）。
- `flag` 波：生成前 300 tick 显示"大波来袭"横幅（`WaveState.banner`），并额外发 `WaveStarted { flag: true }`。
- 最后一波全部死亡且无待衍生 → 胜利。

### 10.2 关卡表（Phase 1）

| 关 | 车道 | 起始 Dew | 卡 | 波数 | 新敌人 | 通关解锁 | 意图 |
|---|---|---|---|---|---|---|---|
| 1-1 | [2] | 150 | Forced [pebble_pod] | 4 | husk | dewcup | 学会放置与收集 |
| 1-2 | [1,2,3] | 100 | Forced [pebble_pod, dewcup] | 6 | twig_husk | barkwall | 多车道与经济 |
| 1-3 | [1,2,3] | 50 | Forced [pebble_pod, dewcup, barkwall] | 7 | — | frostpod | 阻挡 |
| 1-4 | [0..4] | 50 | Choose, slots 6 | 8 | clay_husk | twinpod | 五车道 |
| 1-5 | [0..4] | 50 | Choose, 6 | 9 | sprinter_husk | thornmat | 速度威胁 |
| 1-6 | [0..4] | 50 | Choose, 6 | 10 | vault_husk | tuberblast | 跨越对策 |
| 1-7 | [0..4] | 50 | Choose, 7 | 10 | shield_husk | emberbulb | 高护甲 |
| 1-8 | [0..4] | 50 | Choose, 7 | 11 | carrier_husk | gourdshell | 衍生 |
| 1-9 | [0..4] | 50 | Choose, 8 | 12 | drum_husk | snapmaw, sunlantern | 重型 |
| 1-10 | [0..4] | 75 | Choose, 8 | 14 | 全部混合 | gustbloom | 阶段终局 |

每关的具体 `waves` 由实现者按以下预算生成并写入 RON（不是运行时生成）：第 k 波（从 1 计）总威胁 ≈ `ceil(base × k^1.15)`，`base` 从 1-1 的 1 递增到 1-10 的 3；`flag` 波威胁 ×2；新敌人首次出现在该关第 3 波或之后；每关 `seed = 100 + 关序号`。实现者在 `docs/levels.md` 中记录每关的威胁曲线表。

**预算上限（v1.1 修订，2026-10-14）**：上式在后期增长过快，因此对每波预算施加上限：非 flag 波 ≤ `cap`，flag 波 ≤ `flag_cap`。上限按以下阶梯取值，从第 1 档开始；某关在当前档位下写不出满足 10.3 的脚本解时，该关（且仅该关）降一档重新生成，无需再向负责人提问；三档用尽仍不可解才停下提问。

| 档 | cap | flag_cap |
|---|---|---|
| 1 | 40 | 70 |
| 2 | 35 | 60 |
| 3 | 30 | 50 |

已有脚本解且满足 10.3 的关卡不受本修订影响，不必重新生成；1-4 的 golden 快照保持不变。

### 10.3 脚本解的质量要求

`every_level_has_scripted_solution` 除"回放必胜"外还必须满足：脚本解全程触发 Hose Burst **不超过 1 次**。Hose 是给玩家失误用的保险，不是设计预算的一部分；机器人需要多条保险才能通过的关卡，真人必输。本条对全部 10 关成立；任何关卡的脚本解 Hose 超过 1 次都必须重录，预算本就低于上限的关卡只重录脚本解、不改关卡数据（如 1-3，ADR-006）。测试 `scripted_solution_uses_at_most_one_hose` 对每关断言此条。

---

## 11. UI 与交互

### 11.1 UiAction（UI 与模拟的唯一接口）

```rust
pub enum UiAction {
    SelectCard(usize), DeselectCard, ToggleShovel,
    PlaceAt { row: u8, col: u8 }, ShovelAt { row: u8, col: u8 },
    CollectDewAt(Vec2),
    PauseToggle, Restart, NextLevel, BackToTitle, BackToLevelSelect,
    StartLevel(LevelId), ConfirmSeeds(Vec<DefenderId>),
    SetVolume { bus: AudioBus, volume: f32 }, SetLanguage(Lang), SetSkin(SkinMode),
}
```

鼠标点击网格的翻译（`BattleScene::update` 中，非 FixedUpdate）：若点击命中 `DewPickup` → `CollectDewAt`；否则若 `shovel_mode` → `ShovelAt`；否则若 `selected_card.is_some()` → `PlaceAt`；点击卡槽 → `SelectCard`；右键 / Esc → `DeselectCard` 或 `PauseToggle`。所有 egui 按钮也只发 `UiAction`。headless 测试通过 `world.events.send(UiAction)` 驱动一切流程（规避引擎报告 3.5 条）。

### 11.2 对局 HUD（egui，`Scene::ui`）

- 左上：Dew 计数（图标 + 数字）。
- 顶部中间：卡槽（`card_slots` 个），每张显示占位色块 / 皮肤图、费用、冷却遮罩（自上而下消退）、不可用时置灰；选中高亮。
- 右上：铲子按钮；暂停按钮。
- 顶部右侧：波次进度条（已生成波 / 总波，flag 波有标记）。
- 底部中间：横幅文字（"准备"、"大波来袭"、"最后一波"）。
- 鼠标悬停格子且已选卡：在格子绘制半透明预览（可放置为绿、不可为红）。

### 11.3 其他场景 UI

- Title：标题文字、"继续"（有存档时）、"新游戏"、"设置"、"退出"。
- LevelSelect：10 个按钮，未解锁置灰；显示已通关标记。
- SeedSelect：左侧已解锁卡牌网格，右侧已选卡槽；"开始"需至少 1 张；`Forced` 关直接跳过本场景。
- Pause：半透明遮罩，"继续"、"重新开始"、"设置"、"返回标题"。
- Outcome：胜利显示解锁的新卡；"下一关" / "重试" / "返回"。
- Settings：音效 / 音乐滑杆、语言切换、皮肤 / 占位切换；实时生效并写入存档。

### 11.4 快捷键

Esc 暂停 / 取消选卡；数字 1 ～ 8 选卡；空格 继续；F1 调试面板（devtools feature）。全部走 `UiAction`。

### 11.5 Devtools 扩展（仅 devtools feature）

在引擎调试面板之外，游戏提供 `RootguardDebugWindow`：Dew +100、跳到下一波、立即胜利 / 失败、显示每个敌人 hp 与状态、`DebugDraw` 绘制格子边界与敌人碰撞区间。

### 11.6 文本

`assets/data/i18n/{zh,en}.ron`，`HashMap<String, String>`；代码通过 `t("key")` 取用。缺失 key 显示 key 本身并 `log::warn!` 一次。所有防御者 / 敌人名称与描述 key 形如 `defender.pebble_pod.name`。

中文显示依赖引擎补充 E-1；E-1 落地前以英文运行，中文表仍需齐备。

---

## 12. 渲染

### 12.1 两种模式

`SkinMode::Placeholder`（必须完整可玩）与 `SkinMode::Skinned`。**v1.3：默认为 `Skinned`（皮肤 `garden`）**；Placeholder 保留为设置中的可选项与 CI / 测试的基准。切换只影响 `rootguard-app` 的渲染系统，模拟层无感知。自动化验收仍以 Placeholder 为准；Skinned 允许部分实体回退占位。

### 12.2 占位渲染

- 背景：草地分层色带（激活车道亮绿、非激活暗绿）、格子细线、左侧温室为蓝灰矩形、右侧入口为深色竖条。
- 防御者：`Placeholder.color` 实心圆角矩形 80 × 90 + 白色 `label`；Gourdshell 为宿主外围描边环；生命 < 50% 时加红色描边。
- 敌人：实心矩形 50 × 90 + `label`；护甲以头顶小矩形表示（Helm 棕、Shield 前方竖条、Drum 腹部圆）；Chill 时 tint 偏蓝；受伤后 6 tick 内 tint 白。
- 弹丸：圆。Dew：黄色圆 + 微光描边。
- 行遮挡：在 E-3 落地前，每行使用独立 `Layer(WORLD.0 + row)`，行内按提交顺序；E-3 落地后改用 `sort_y`。
- 全部 `Transform2D` 用 `lerp(time.alpha)` 插值绘制。

### 12.3 皮肤渲染（原创美术方向）

- 风格：扁平几何、粗轮廓、低饱和暖色调；每个角色由 ≤ 6 个基本形状组合，禁止任何拟人五官以外的细节模仿既有作品。
- 每个防御者：idle 4 帧、attack（若有）3 帧；每个敌人：walk 4 帧、eat 3 帧（`eat_hit` 事件帧）、armor 破损替换图。
- 图集与动画走引擎 RON 格式；在 `assets/skins/default/`。
- 美术由实现者以程序化方式生成（Rust 或 Python 脚本写 PNG，脚本随仓库提交）或由 AI 图像工具生成后人工审核；来源登记在 `assets/SOURCES.md`。

---

## 13. 音频

| 事件 | 音效 key |
|---|---|
| `ProjectileFired` | `sfx.shoot` |
| `ProjectileHit` | `sfx.hit`（护甲为 `sfx.hit_armor`） |
| `DefenderPlaced` | `sfx.plant` |
| `DefenderRemoved(Shovel)` | `sfx.dig` |
| `DewCollected` | `sfx.dew` |
| `EnemyDied` | `sfx.husk_down` |
| `WaveStarted(flag)` | `sfx.flag_wave` |
| `HoseTriggered` | `sfx.hose` |
| `AnimationEvent { event: "eat_hit" }`（敌人 eat 片段的啃食帧，SPEC 12.3） | `sfx.bite`（收尾期补充，ADR-010） |
| `DewSpawned`（生产者产出；ART-SPEC 4.2 / 4.12） | `sfx.drip`（美术阶段，ART-SPEC 12） |
| `DefenderRemoved(Killed)` 树皮墙（4.3） | `sfx.wood_crack`（美术阶段，ART-SPEC 12） |
| `ProjectileFired` 冰石（4.4） | `sfx.shoot_frost`（美术阶段，ART-SPEC 12） |
| `ProjectileHit` 冰石（6 章） | `sfx.hit_frost`（美术阶段，ART-SPEC 12） |
| 荆棘垫 `trigger` 片段事件帧 `prick`（4.6） | `sfx.prick`（美术阶段，ART-SPEC 12） |
| `DefenderRemoved(Consumed)` 块茎雷（4.7） | `sfx.tuber_burst`（美术阶段，ART-SPEC 12） |
| `DefenderRemoved(Consumed)` 炽球（4.8） | `sfx.ember_burst`（美术阶段，ART-SPEC 12） |
| `DefenderRemoved(Killed)` 葫芦壳（4.9） | `sfx.shell_crack`（美术阶段，ART-SPEC 12） |
| `EnemyDamaged(Chomp)`（4.10） | `sfx.snap`（美术阶段，ART-SPEC 12） |
| 咬夹草 `snap` 片段事件帧 `gulp`（4.10） | `sfx.gulp`（美术阶段，ART-SPEC 12） |
| `DefenderRemoved(Consumed)` 风铃花（4.11） | `sfx.gust`（美术阶段，ART-SPEC 12） |
| `DewSpawned` 日灯笼第 `bonus_every` 次（4.12） | `sfx.lantern`（美术阶段，ART-SPEC 12） |
| `DefenderRemoved(Killed)` 其他守卫（4.13） | `sfx.wilt`（美术阶段，ART-SPEC 12） |
| `EnemyArmorBroken` 枝冠（5.2） | `sfx.twig_snap`（美术阶段，ART-SPEC 12） |
| `EnemyArmorBroken` 陶盆（5.3） | `sfx.pot_break`（美术阶段，ART-SPEC 12） |
| 跳栏 `vault` 片段事件帧（5.5） | `sfx.vault`（美术阶段，ART-SPEC 12） |
| `EnemyDied` 背篓枯壳（与 `sfx.husk_down` 同时，5.7） | `sfx.seeds_spill`（美术阶段，ART-SPEC 12） |
| 鼓腹枯壳 `walk` 片段事件帧 `thud`（5.8） | `sfx.thud`（美术阶段，ART-SPEC 12） |
| `EnemyArmorBroken` 葫芦（5.8） | `sfx.gourd_break`（美术阶段，ART-SPEC 12） |
| `EnemyDied` 小芽怪（代替 `sfx.husk_down`，5.9） | `sfx.sprout_down`（美术阶段，ART-SPEC 12） |
| `BattleEnded(Won/Lost)` | `sfx.win` / `sfx.lose` |
| 场景 | Title / Battle / Outcome 各一首循环 BGM |

全部音频自制（建议用 Rust 脚本程序化合成简单波形并写 WAV → OGG）或 CC0，登记来源。`SfxPlugin` 在 `Stage::Update` 消费事件写入 `AudioQueue`；同一 tick 同一音效最多触发 2 次。

---

## 14. 存档与设置

- 路径：`directories::ProjectDirs::from("", "", "rootguard")` 的 data 目录下 `progress.ron`。
- 内容：`Progress { version: u32, cleared: BTreeSet<LevelId>, unlocked: BTreeSet<DefenderId>, settings: { sfx: f32, music: f32, lang: Lang, skin: SkinMode } }`。
- 写入时机：`BattleEnded(Won)`、设置变更。原子写（临时文件 + rename）。
- 读取失败（损坏 / 版本不符）：备份为 `progress.ron.bak`，以默认进度启动，`log::warn!`。
- 环境变量 `ROOTGUARD_SAVE_DIR` 覆盖目录（测试用）。

---

## 15. 引擎补充需求（唯一允许修改引擎的清单）

以下条目来自引擎 v1 报告第 4 节。实现者在游戏开发中按需向引擎仓库提交 PR，每条对应一个 ADR 与测试；游戏仓库通过更新 `rev` 消费。

| 编号 | 需求 | 触发时机 |
|---|---|---|
| E-1 | 带 LICENSE 的开源 CJK 字体子集 + `RunConfig.font_path` / 后端 `setup_fonts` | G3（HUD 中文） |
| E-2 | `World` 上的资产加载辅助 `load_atlas_into(world, rel)` / `load_clip_into` / `load_audio_into` | G1 |
| E-3 | `DrawList::rect_sorted / circle_sorted / text_sorted` 带 `sort_y` 参数 | G2 |
| E-4 | `HeadlessRunner` 向 egui 注入指针事件（可选；本游戏以 `UiAction` 规避，仅当需要测试 egui 布局时） | 按需 |
| E-5 | 音乐交叉淡化 `AudioRequest::CrossfadeMusic { clip, ticks }` | G5 |

禁止在游戏仓库内复制引擎代码或以 patch 方式绕过。

---

## 16. 测试

### 16.1 规则单元测试（`rootguard-core`，headless，无渲染）

| 测试 | 断言 |
|---|---|
| `pebble_kills_husk_in_10_hits` | 1 发 1 伤，第 10 发命中后 `EnemyDied` |
| `clay_husk_armor_then_hp` | 28 + 10 = 38 发致死，第 28 发后 `EnemyArmorBroken` |
| `frost_halves_speed_and_bite` | Chill 期间位移为基础一半，啃食间隔 120 |
| `shield_ignored_by_spikes` | Thornmat 对 shield_husk 直接扣主生命 |
| `mine_arms_after_900_ticks` | 899 tick 时敌人踩上不爆，900 后爆且伤害 90 无视护甲 |
| `emberbulb_hits_3x3` | 3 × 3 内敌人受伤，外侧不受 |
| `gourdshell_absorbs_first` | 宿主 hp 不变直到 Gourdshell 归零 |
| `snapmaw_swallows_and_chews` | 吞食即死；2520 tick 内不再吞；Heavy 不被吞 |
| `vault_husk_jumps_once` | 第一次遇墙跨越到左侧格，第二次正常啃食 |
| `carrier_spawns_sproutling` | 死亡同 tick 原地生成 |
| `sprinter_speeds_up_below_half` | hp ≤ 5 后速度 0.675 |
| `gust_pushes_and_clamps` | 推 150，Heavy 75，上限 1260 |
| `hose_once_per_lane` | 首次越线清车道，第二次失败 |
| `wave_early_start_at_40_percent` | 剩余 hp ≤ 40% 时下一波提前，且 ≥ 300 tick 间隔 |
| `dew_cap_and_expire` | 上限 9990；落地 900 tick 后消失 |
| `card_ready_requires_cooldown_and_dew` | 两条件缺一不可 |
| `placement_rules` | 非激活车道 / 已占格 / Overlay 规则 / Dew 不足均拒绝且不扣费 |
| `targeting_leftmost_then_entity_id` | 同格两敌人时命中 x 小者 |
| `damage_order_deterministic` | 同 tick 多来源结算顺序与 seq 一致 |

### 16.2 流程与确定性（`rootguard-app` 集成，`HeadlessRunner` + `UiAction`）

| 测试 | 断言 |
|---|---|
| `level_1_1_scripted_win` | 固定脚本（第 60 tick 放 pebble_pod 于 (2,0)，之后每次 ready 且 Dew 够时在 (2,1..4) 放）在 20000 tick 内 `BattleEnded(Won)` |
| `level_1_1_idle_lose` | 不操作时触发 Hose 后再次越线 → `Lost` |
| `every_level_loads_and_validates` | 10 关 RON 全部通过校验 |
| `every_level_has_scripted_solution` | 每关附一个 `tests/scripts/<id>.ron` 操作脚本（tick → UiAction 列表），回放必胜；这是关卡可玩性的最低保证 |
| `scripted_solution_uses_at_most_one_hose` | 每关脚本解回放中 `HoseTriggered` 事件 ≤ 1 次（10.3） |
| `determinism_golden_1_4` | 1-4 脚本回放 30000 tick，`snapshot()` 与 `tests/golden/level_1_4.snap` 一致 |
| `save_roundtrip` | 通关后进度写入，重启读取一致；损坏文件走备份路径 |
| `ui_flow_title_to_battle` | `UiAction` 序列 StartLevel → ConfirmSeeds 后 `SceneStack::top_name() == "Battle"` |
| `audio_events_mapped` | 一局中 `take_audio_log` 含 `sfx.shoot`、`sfx.dew`、`sfx.plant`；首次 BGM 为 `PlayMusic`，之后的场景切换为 `CrossfadeMusic`（E-5 落地后，ADR-009） |
| `placeholder_mode_draws_all_entities` | 对局中 DrawList 的 Rect/Circle 数 ≥ 实体数 |

### 16.3 性能

Release 下 1-10 最大波（≈ 40 敌人、45 防御者、100 弹丸）帧耗时 < 4 ms；以 devtools 面板读数为准，记录到 `docs/progress.md`。

---

## 17. 里程碑与验收

| 里程碑 | 交付 | 验收 |
|---|---|---|
| G0 骨架 | workspace、三 crate 空壳、引擎 `rev` 锁定、`check_deps.sh`、CI、ADR 模板、`assets/data` 目录与 i18n 空表、`docs/progress.md` | 构建通过；依赖规则检查生效 |
| G1 核心规则 | `rootguard-core` 全部组件 / 资源 / 事件 / 系统；`rootguard-content` 加载校验；12 防御者 + 9 敌人 RON；E-2 | 16.1 全部通过；`every_level_loads_and_validates`（此时关卡可为临时 1 ～ 2 个） |
| G2 可玩对局 | `BattleScene` + 占位渲染 + HUD + `UiAction` + 鼠标翻译；关卡 1-1 ～ 1-3；E-3 | 手动完整玩通 1-3；`level_1_1_scripted_win / idle_lose`；`placeholder_mode_draws_all_entities` |
| G3 完整流程 | Title / LevelSelect / SeedSelect / Pause / Outcome / Settings；存档；i18n；E-1；音效与 BGM | `ui_flow_title_to_battle`、`save_roundtrip`、`audio_events_mapped`；中文 HUD 显示正确 |
| G4 全部关卡 | 1-4 ～ 1-10 RON 与脚本解；`docs/levels.md` 威胁曲线；devtools 扩展 | `every_level_has_scripted_solution`、`determinism_golden_1_4`；手动通关全部 10 关并记录每关用时与感受 |
| G5 皮肤与打磨 | `Skinned` 模式美术（程序化生成）、Dew 飘落 Tween、受伤闪白、横幅动画、E-5 交叉淡化、三平台打包脚本 | 两种模式切换无差异；性能 16.3 达标；`docs/game-v1-report.md` |

---

## 18. Phase 2+ 备忘（不实现）

夜晚场景与墓碑生成点、水域车道与浮台类防御者、屋顶斜面弹道、更多护甲与特殊移动方式（挖掘、跳跃、飞行）、Boss 关、生存 / 无尽模式、商店与升级、图鉴与成就、手柄支持、更多语言。以上均需新的游戏 spec 版本，禁止在 Phase 1 代码中预埋。

---

## 19. 术语表

| 术语 | 含义 |
|---|---|
| Dew | 游戏内资源，用于放置防御者 |
| Husk | 敌人统称 |
| 防御者 | 玩家放置的植物单位 |
| Hose Burst | 每车道一次的保险清场 |
| flag 波 | 大波，带横幅与双倍威胁 |
| 威胁 | 敌人的强度权重，用于波次预算 |
| 占位渲染 | 几何图形 + 字母标签的渲染模式 |
| 脚本解 | 一个能必胜该关的 `UiAction` 时间表，用于自动化验证可玩性 |

---

## 20. Phase 1 收尾与难度校准（v1.2 新增）

G5 关闭后、Phase 2 spec 之前，项目处于**收尾期**。本章定义收尾期允许的改动范围与流程。

### 20.1 冻结规则

- 禁止新增防御者、敌人、关卡、场景、行为类型（属于 Phase 2）。
- 允许：修复缺陷、按 20.2 校准难度、补齐第 20.3 节发布清单、改进文档与工具。
- 任何 FixedUpdate 逻辑改动仍须保持 `tests/golden/level_1_4.snap` 不变；若必须改变（缺陷修复导致），停下提问。

### 20.2 难度校准流程（基于真人试玩反馈）

负责人在 `docs/levels.md` 填写"手动用时 / 主观难度"后，实现者按以下规则处理，每关一次提交：

| 负责人主观难度 | 处理 |
|---|---|
| 过难（多次失败或必须用 Hose） | 该关降一档（10.2 阶梯）重新生成并重录；已是第 3 档则允许把该关 `start_dew` +25 并记录 ADR |
| 偏难（险胜） | 波间隔 +300 tick，重录；不改预算 |
| 合适 | 不动 |
| 偏易 / 过易 | 不动（Phase 1 以"真人可通关"为唯一目标，不追求紧张感） |

校准后更新 `docs/levels.md` 对应行并保留调整历史；1-4 不得改动（golden）。

### 20.3 发布清单（v0.1.0）

- 项目许可证文件 `LICENSE`（由负责人选定，见 `docs/owner-decisions/`），`Cargo.toml` 各 crate 填写 `license` 字段，`package.sh` 把 `LICENSE` 一并打包。
- `CHANGELOG.md`，首个条目 `0.1.0`。
- Git tag `v0.1.0`，CI 以 tag 触发时上传三平台安装包为 Release 资产。
- `README.md` 补充：截图（占位 / 皮肤各一张，由 CI Xvfb 截图或负责人提供）、安装方法、操作说明、许可证说明。
- 负责人实机验证记录（macOS / Windows 启动、音频听感）写入 `docs/progress.md`；未验证项在 Release 说明中标注。

