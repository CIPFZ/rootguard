# ADR-016：A2 动画的驱动方式（游戏侧）

- 编号：ADR-016
- 标题：A2 动画的驱动方式（游戏侧）
- 日期：2026-10-16
- 状态：已接受；补充 ADR-014（计时类片段进度驱动），按 A2 各类逐节追加

## 背景

裁决（七）第 4 节：A2 的动画一律按 ART-SPEC 3.1 的时长绑定与进度驱动。ADR-014 已规定 `attack` / `produce` /
`arm` / `fuse` 由计时器进度驱动、`eat` 由 `Biter` 相位驱动（ADR-012 后续条目）。ART-SPEC 3.1 另有几种“事件
触发 / 状态进入”的片段与若干纯呈现行为，需要定下它们由什么状态决定。原则不变：只读模拟组件，不改模拟。

## 决策

### 1. 守卫 attack 类（2026-10-16）

1. **双籽荚两枚弹丸**：第 k 枚在射手计时器重置后 `k × volley_gap_ticks` 生成（SPEC 7.2），`fire` 事件放在片段
   tick `0` 与 `volley_gap_ticks`（6）。`artgen` 校验 `fire` 的 tick 恰为 `k × volley_gap_ticks`（读取数据），
   数据改了而片段没跟上即拒绝生成。ART-SPEC 4.5 写的“78 与 84”按 ADR-014 第 5 条同理换算。
2. **咬夹草**：`Chomper` 处于 `Chewing` 且咀嚼计时器已过 tick < `snap` 片段长（12）时播 `snap`，相位 = 已过
   tick（吞食就是计时器重置的那个 tick）；之后循环 `chew`（时钟）；`Ready` 时 `idle`。张嘴角度上下荚瓣合计 44°
   （裁决七：≤ 45°）。
3. **荆棘垫**：格内有存活敌人（与模拟 Spikes 相同的 `Grid::enemy_overlaps_cols` 判定）时播 `trigger`，相位 =
   Spikes 计时器已过 tick，`prick` 在 0 即造成伤害的 tick；否则 `idle`（打盹）。`artgen` 把 `trigger` 加入必需
   绑定（`spikes.interval_ticks`）。
4. **被啃抖动**：防御者受伤闪白期间（6 tick）左右交替抖 2 px（ART-SPEC 4.3 / 4.9）。鼓腹枯壳啃食时设计卡写
   “守卫抖 3 px”，现统一 2 px（呈现层拿不到伤害来源）。
5. 测试：`chomper_and_spikes_clips_follow_state`（1-9，咬夹草 snap / chew / idle、荆棘垫 trigger / idle 及相位）。

### 2. produce 类（2026-10-16）

1. **两层绘制**：设计卡要求生产者同时有时钟驱动的 idle（摇摆）与进度驱动的 produce（露珠长大 / 灯笼点亮），
   单张片段要同时表达两种节奏只能把 1440 tick 逐帧画满（每个生产者数百帧）。改为：身体播 `<id>_idle`（时钟，
   不含进度相关部件），其上叠画 `<id>_produce`（只含露珠 / 光晕 / 哈欠或三盏灯笼，相位 = 生产计时器已过 tick），
   同锚点、同 sort_y、后提交。ADR-014 的 `produce` 进度驱动不变，只是从“替换身体”改为“叠层”；`default` 皮肤
   没有叠层片段，行为不变。`rest` 片段（完整位姿）只供审核表与头像。
2. **`pop` 在片段时间 0**：露水在生产计时器到期并重置的 tick 生成，同 ADR-014 第 5 条。
3. **日灯笼的金色一轮**：下一次生产是额外产量（`(produced + 1) % bonus_every == 0`）时叠层改用
   `produce_gold`（三盏变金色，ART-SPEC 4.12）。“落两颗”是模拟本身的产量，不在动画里画。
4. **露杯的露珠**在叠层里不随杯身摇摆；为此杯身摇摆只有 ±1.2°（顶部偏移约 1 px）。
5. 测试：`producer_overlay_follows_timer`（1-10，露杯与日灯笼：身体 idle、叠层相位 = 计时器、金色一轮）。

### 3. fuse 类（2026-10-16）

1. `fuse`（炽球 / 风铃花）与 `arm`（块茎雷）按 ADR-014 由引信 / 武装计时器驱动；块茎雷武装后循环 `armed`（时钟）。
2. **`burst` 事件放在片段末帧起点（54）**：引信到期的同一 tick 防御者被移除（`DefenderRemoved { cause: Consumed }`），
   片段时间永远到不了 60。爆炸 / 风环特效与音效由移除事件触发（特效类，第 6 节），`burst` 帧只作为数据标记。
3. 风铃花钟口椭圆单独成部件，不随加速旋转的花瓣转动。
4. 块茎雷：引信芽加高 30%、多卷半圈（裁决七第 1 节），块茎整体下沉 4 px 以守住 1.4 的贴地类 48 px。
5. 测试：`fuse_and_arm_clips_follow_timers`（1-10）。

### 4. Husk walk + eat（2026-10-16）

1. **walk / sprint 由走过的距离驱动**：片段时间 = ((1280 − x) ÷ 步幅 的小数部分) × 片段长，步幅 = `Walker.base_speed`
   × 片段长（ART-SPEC 3.1“基础速度 0.30 时一周期 12 px”）。脚不打滑；Chill 速度减半时步频自动减半（ART-SPEC 3.3
   “walk 速度按 StatusEffects 减半”）；停步、被推（风铃花）时相位跟着位置走。取代 ADR-007 第 2 条的时钟相位。
2. **sprint**：`Walker.base_speed` 高于数据中的基础速度（Sprint 特性已把它 × 1.5）时播 `sprint`。
3. **vault**：`WalkState::Vaulting { t }` 时播 `vault`，相位 = `t / Vaulter.ticks × 片段长`；时长绑定
   `Rule("enemies.vault_husk.vault.ticks")`（数据 40）。ART-SPEC 5.5 写 30 tick，与数据冲突，以数据为准。
   跨越中的水平位置由模拟线性移动，弧线高度只在动画里。
4. **spawn**：呈现层记录每个敌人首次出现的 tick，生成后 `spawn` 片段长度内播放 `spawn`（小芽怪）。
5. **状态前缀**：跨越落地后 `vaulted_`；护甲 hp ≤ 1/2 max 时 `cracked_`；护甲没了 `bare_`（沿用 default 皮肤的命名）；
   皮肤缺少带前缀的片段时回退无前缀的。A2 的“护甲破碎替换”因此由这里完成（第 5 类只剩守卫受损分级与破碎特效）。
6. 啃食时一臂扶在守卫侧面：去鸟化后头坐在前肩上，原 40° 的扶臂从下巴下伸出像根棍子，改为 18 ～ 24°。
7. 测试：`husk_clips_follow_movement_and_state`（回放 1-10，walk / eat / cracked / bare / sprint / vault / vaulted /
   spawn 全部出现且与模拟一致，行走相位逐帧等于位置公式）。

### 5. 护甲破碎与受损阶段（2026-10-16）

1. **特效实体**（`rootguard-app/src/fx.rs`，ART-SPEC 7“特效实体由呈现层在 Update 中根据事件生成，寿命以 tick 计，
   不进入模拟层”）：`fx::capture` 注册在 `GamePlugin` 中、先于 `SfxPlugin`，因此在 `sfx_system` 取走事件之前读到；
   特效记在 `RenderState.fx`（随对局场景创建 / 销毁），按 `fixed_tick − 开始 tick` 取帧（暂停时静止），片段播完即删。
   引擎事件双缓冲（本帧 + 下一帧可读）：未被 `sfx_system` 取走的事件会被读两次，`capture` 记住上一帧生成过的
   事件键并跳过。特效画在 `Layer::FX`；占位模式不画。
2. `artgen` 新增：`art/fx/<id>/`（ART-SPEC 9.1）与 `Fx(w, h)` 类别（图集 `garden_fx`）；关键帧新增不透明度 `op`
   （不随父部件传递），用于淡出。特效引用对应角色的专属色时在调色板为该特效登记同值专属色。
3. **护甲破碎**：`EnemyArmorBroken` → 枝冠 `armor_break_twig`、陶盆 `armor_break_pot`（按敌人 id 区分两种 Helm）、
   门板 `armor_break_plank`、葫芦 `armor_break_gourd`，位置在头顶 / 胸前 / 腹部；破碎后的外形由状态前缀（第 4 节）
   切换。敌人与护甲在同一 tick 结算死亡时实体已移除，不播破碎特效（干屑 `crumble` 在特效类）。
4. **防御者受损阶段**（ART-SPEC 2.4）：皮肤有 `<id>_cracked_idle` 时三级（> 2/3 完好、> 1/3 `cracked`、其余
   `split`：树皮墙、葫芦壳），否则两级（≤ 1/2 `hurt`：石籽荚）。葫芦壳也闪白并抖动（4.9）。
5. 测试：`armor_break_spawns_effects`（1-10 回放；全部 10 关脚本解里门板从未破碎——都死于无视 Shield 的伤害，
   因此对一只门板枯壳注入一次破碎事件）、`defender_damage_states`。

### 6. 特效（2026-10-16）

ART-SPEC 7 全表加 4.1 / 4.4 的口部特效，均为 `art/fx/<id>/` 并由 `fx::capture` 按事件生成：

| 事件 | 特效 | 位置 / 尺寸 |
|---|---|---|
| `ProjectileFired` | `muzzle_dust`（石子）/ `muzzle_frost`（冰石） | 发射者格子 + 弹丸出口偏移 |
| `ProjectileHit` | `hit_puff` / `hit_frost` | 目标左缘、弹丸高度；目标同 tick 死亡时用 `EnemyDied.x` |
| `EnemyDied` | `crumble`；小芽怪 `sprout_fold`；被吞（同 tick `EnemyDamaged::Chomp`）不播 | 脚底上方 |
| `EnemyDamaged::Spikes` | `plant_dust` 缩小（4.6 被扎脚下小尘） | 敌人脚底 |
| `DefenderPlaced` | `plant_dust`；防御者本身 1.2 → 1.0 弹性缩放 10 tick | 格子脚底 |
| `DefenderRemoved` | Killed → `wilt`；Shovel → 残影（idle 第 0 帧上升淡出 12 tick）；Consumed → `tuber_burst` / `ember_wave`（300 × 345）/ `gust_ring`（从格子左缘拉伸到 x = 1260） | |
| `DewSpawned(Producer)` | `dew_pop` | 露水位置 |
| `HoseTriggered` | `hose_burst`（拉伸到 x 235 ～ 1160） | 车道中心 |
| Chill（状态，非事件） | `frost_crown` 循环，画在 Husk 头顶 | 每种 Husk 头顶高度表 |

1. 命中特效需要弹丸种类，`ProjectileHit` 不带：`capture` 在 `ProjectileFired`（带 id）时记下“弹丸实体 → id”，
   弹丸消失后删除。
2. 咬夹草的 `snap` 叶齿残影与 `chew` 结束小气泡（4.10）未做：没有对应事件（咀嚼结束只是状态变回 Ready），
   效果很小；记录为未实现。
3. 测试：`battle_effects_follow_events`（1-10：十类特效与 Chill 霜晶）、`constructed_effects`（石子口部 / 命中、
   风环拉伸、铲除残影、放置缩放——1-10 脚本解没用到双籽荚、风铃花与铲子）。

### 7. 音效（2026-10-16）

1. ART-SPEC 12 的 20 个新音效由 `scripts/gen_audio.py` 合成（只新增文件，已有音频未重新编码），登记 `SOURCES.md`，
   SPEC 13 表按 ART-SPEC 12“允许的改动清单”补行。触发方式见 SPEC 13 表：模拟事件或片段事件帧。
2. **片段事件帧 → 音效**（ART-SPEC 3.2）：`gulp`（咬夹草 snap 末帧）、`thud`（鼓腹枯壳 walk 每步）、`prick`（荆棘垫
   trigger 第 0 帧 = 扎刺伤害的 tick）、`vault`（跳栏开始）与原有 `eat_hit`。呈现层对全部角色计算当前片段与相位，
   与上一帧比较后发引擎 `AnimationEvent`；新片段的第 0 帧事件只在确实从第 0 帧开始时发（敌人中途走进荆棘垫不算）。
3. **事件帧固定取 garden 片段**（缺片段回退 default），不论渲染模式与所选皮肤：音效时机属于美术规格的事件帧，
   不应随设置改变（ADR-010“与渲染模式无关”的推广）。`default` 皮肤的 eat 片段与 garden 同长、`eat_hit` 都在末帧，
   原有啃食音效不变。
4. **不能取走的事件**（`EnemyDamaged`、`EnemyArmorBroken`、`DewSpawned`，呈现层也要读）：`sfx_system` 只读并记住
   上一帧的事件键去重（双缓冲）。生产的额外产量是同一 tick 的第二个 `DewSpawned`，按生产者去重；日灯笼第
   `bonus_every` 次（`produced % bonus_every == 0`）换 `sfx.lantern`。
5. 冰石命中 `sfx.hit_frost`：`sfx_system` 在 `ProjectileFired` 时记下弹丸是否冰石，命中时查表（`ProjectileHit` 不带
   弹丸 id）；门板破碎没有新音效（5.6 只写命中木板用 `sfx.hit_armor`）。
6. 测试：`art_phase_sfx_mapped`（1-10，16 种新音效；只用冰石时不出现普通发射 / 命中；铃声远少于滴落）、
   `constructed_sfx_mapped`（咬夹草 snap 与 gulp 一一对应、风铃花 gust、葫芦壳 shell_crack）。波形图
   `docs/screenshots/a2-sfx.png`（自上而下：drip、wood_crack、shoot_frost、hit_frost、prick、tuber_burst、ember_burst、
   shell_crack、snap、gulp、gust、lantern、wilt、twig_snap、pot_break、vault、seeds_spill、thud、gourd_break、sprout_down）。
   听感未经人耳确认（本环境无音频设备）。

## 后果

- 新片段都由模拟状态唯一决定，暂停、回放、切换渲染模式都不会让动画与模拟错位。
