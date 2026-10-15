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

## 后果

- 新片段都由模拟状态唯一决定，暂停、回放、切换渲染模式都不会让动画与模拟错位。
