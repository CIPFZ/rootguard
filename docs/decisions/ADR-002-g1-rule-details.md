# ADR-002：G1 规则层的实现细节与 SPEC 未写清之处

- 编号：ADR-002
- 标题：G1 规则层的实现细节与 SPEC 未写清之处
- 日期：2026-10-13
- 状态：已接受

## 背景

实现 SPEC.md 第 4 ～ 10 章与 16.1 测试时，遇到下列未定义、无法字面实现或彼此有出入的地方。
逐条列出候选并选择最简单、且不违反 1.2 知识产权约束、3.1 依赖规则与引擎确定性规则的方案。

## 决策

### A. 数据与类型

1. **内容 id 为定长内联字符串**（`ShortStr`，≤ 23 字节 ASCII，`Copy`，按字典序比较）。组件与事件
   需要 `Copy`（SPEC 5.1），`String` 不满足；`&'static str` 需要泄漏内存或全局驻留表（全局可变状态，
   AGENT 4.5 禁止）。`Placeholder.label` 同样用 `ShortStr`（SPEC 5.1 写作 `&'static str`），颜色用 `u32`
   十六进制（SPEC 5.1 已允许）。
2. **关卡 id 格式**：SPEC 4.5 要求“所有 id 仅含 `[a-z0-9_]`”，但 4.4 / 10.2 的关卡 id 为 `1-3`。
   决策：字符集规则只约束防御者 / 弹丸 / 敌人 id（三者共享一个唯一性命名空间）；关卡 id 必须形如
   `章-序`（两段十进制数字），在关卡之间唯一。
3. **全局规则数值进 RON**：SPEC 6 / 8.2 / 9 / 10.1 中的玩法数值（Intro 180、Dew 上限 9990、天空 Dew
   420 / 570 / 180 / 25 / 1.0、过期 900、提前开波 40% / 300、横幅 300、Chill ×0.5 / ×2、Heavy 推距 ×0.5）
   放在 `assets/data/rules.ron`，代码不出现这些数字（AGENT 3.3）。空间定义（2.2 / 2.5 / 7.2 的坐标、
   半宽、偏移、Dew 点击半径、占位尺寸）集中在 `rootguard-core/src/grid.rs`。
4. **新增数据字段**：`Producer` 增加可选 `bonus_every` / `bonus_amount`（Sunlantern 每 4 次 +25，
   合并为一次产出 50）；`Overlay(repair_below_num, repair_below_den)`（2/3）；`Blast.radius_cells`（3 × 3
   即 1）；`Chomper.reach_cells`（右侧 1 格）；敌人特性 `Vault(ticks)`、`Sprint(threshold_percent, factor)`。
5. **`Placement::Any`** v1 无内容使用，按 `Ground` 处理。
6. **音量总线**：`UiAction::SetVolume.bus` 若使用引擎 `AudioBus` 会让 core 依赖 `trellia-audio`（违反 R1）。
   决策：core 自有 `VolumeBus { Sfx, Music }`，由 app 映射。

### B. 调度与事件

7. **FixedUpdate 调度**：SPEC 6 允许“场景调用”或“插件注册”二选一。选择场景调用：
   `BattleScene::fixed_update` 调用 `battle_tick`；四个 core 插件只注册事件与资源。这样不在对局中时
   规则系统不会运行，headless 测试也可直接以 `App::step_ticks` 驱动 `battle_tick`。
8. **事件只用于通知呈现层**；模拟内部的跨系统数据走资源队列（`DamageQueue`、`PendingOps`、`DeathLog`）。
   原因：引擎事件按帧翻页，一帧 0 ～ 5 个 tick，用 `iter()` 读事件会在多个 tick 中重复处理。
9. **`UiAction` 的分流**：`ui_action_system` 取走全部 `UiAction`，消费 6 个对局动作（选卡、取消、铲子、
   放置、铲除、收集），其余场景动作原样放回事件队列，由 app 场景在 Update 中消费。
10. **`level_tick` 只计 `Playing` 的 tick**（Intro 与暂停不计）；脚本解的时间轴即 `level_tick`。

### C. 规则细节

11. **新防御者下一 tick 起生效**：放置时 `reserve_entity` + 延迟插入（tick 末），因此“放置后 N tick”
    从放置 tick 的下一 tick 开始计。Tuberblast 在放置后第 900 tick 武装并在同一 tick 检查触发。
    同一 tick 内对同一格的重复放置由本地集合拦截。
12. **Shooter 计时**：放置时即“就绪”；冷却每 tick 递减（与有无目标无关），就绪且有目标时发射并重置。
    `volley_gap_ticks == 0` 时整轮同 tick 发出。
13. **伤害溢出**：护甲吸收不完的伤害继续落到主生命（SPEC 2.4“护甲层 → 主生命”顺序结算）。
14. **Sprint 阈值**：SPEC 8.1 写“hp < 50%”，16.1 测试写“hp ≤ 5 后速度 0.675”（10 的 50% 即 5）。
    两者在 hp = 5 时矛盾；以验收测试为准取“主生命 ≤ 50%”，阈值在数据中（`threshold_percent`），
    已在 `docs/progress.md` 提请负责人确认。
15. **Vault**：跨越耗时 40 tick（数据 `Vault(ticks: 40)`，SPEC 未给出）；落点为被跨越防御者左侧格中心
    （`cell_center.x - 100`）。被跨越目标是“就绪的 Snapmaw”时不跨越而停下，由 Snapmaw 在同 tick 吞食
    （SPEC 8.1“吞食优先于跨越”）。
16. **Snapmaw 范围**：敌人碰撞区间与 `[cell_left, cell_right + 100 × reach_cells]` 相交即“进入”；
    咀嚼计时结束的 tick 恢复就绪，下一 tick 起可再吞。同 tick 多个 Snapmaw 不会选中同一目标。
17. **Gust**：推后 x 上限 1260，但不会把原本就在 1260 右侧的敌人往左拉；跨越中的敌人起止点一并平移。
18. **Hose 清场不触发 Carrier 衍生**（否则衍生物出生在失败线左侧，下一 tick 必然判负，违背“保险”语义）。
19. **Gourdshell 修复**：恢复现有覆盖层为满血（同一实体），扣费并发 `DefenderPlaced`。
20. **铲子**：`ShovelAt` 不要求处于铲子模式（动作本身已明确）；成功铲除后退出铲子模式。
21. **弹丸越界**：x > 1300 销毁（逻辑宽度之外，覆盖全部敌人生成位置 1240 ～ 1279）。
22. **波次生成**：一波的全部敌人在该波开始的 tick 一次生成（x 抖动由 `GameRng` 提供）。
    提前开波把剩余计时缩短到“距上一波满 300 tick”，flag 波横幅在剩余 ≤ 300 tick 时出现。
23. **Producer** 首次产出在放置后满一个周期；产出位置偏移 `gen_range(-30..=30)`（两轴独立）。
24. **啃食计时**用已经过 tick 数（`Biter.elapsed`），Chill 时阈值按规则倍增，离开啃食状态清零。

## 后果

- 规则层不依赖事件的帧语义，`step_ticks` 与真实帧驱动结果一致。
- 第 14 条需要负责人确认；若改为严格小于，只需改 `sprinter_husk.enemy.ron` 与对应测试的阈值判断。

## 替代方案

- （1）`DefenderId(u16)` 索引：存档、日志与快照需要再查表转换，且注册表变化会改变索引，否决。
- （7）插件注册 FixedUpdate 系统：每个系统需判断“是否在对局中”，且与场景暂停语义重复，否决。
- （11）同 tick 立即生效：放置当 tick 即计时，“放置后 900 tick 武装”需要在计时器上 +1 补偿，否决。
