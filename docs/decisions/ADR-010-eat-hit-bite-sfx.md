# ADR-010：`eat_hit` 动画事件与啃食音效 `sfx.bite`

- 编号：ADR-010
- 标题：`eat_hit` 动画事件与啃食音效 `sfx.bite`
- 日期：2026-10-14
- 状态：已接受；补充 ADR-007 后果第 2 条（`eat_hit` 无消费者）

## 背景

负责人裁决 `docs/owner-decisions/2026-10-14-g5-closure.md` 第 3 步与 AGENT.md 第 9 章第 2 项：`SfxPlugin` 在
`AnimationEvent { event: "eat_hit" }` 时播放新音效 `sfx.bite`，模拟层不改，golden 不变。

引擎的 `AnimationEvent` 只由引擎 `animation_system` 对挂有 `Animator` + `Sprite` 的实体发送；而 ADR-007 第 1 条
为保证“两种模式切换无差异”，皮肤不在模拟实体上挂组件、也不建呈现实体，帧由 `fixed_tick` 推导。
因此游戏里不存在任何会发出 `AnimationEvent` 的东西，需要决定由谁、在什么条件下发出。

## 决策

1. **呈现层发出引擎 `AnimationEvent`**：`render::track_visuals`（`BattleScene::update`，暂停时不调用）中的
   `emit_anim_events` 对每个处于 `WalkState::Eating` 的存活敌人，按绘制时相同的规则选片段
   （`<id>_eat`，护甲破损后 `<id>_bare_eat`）与相位（`fixed_tick + 实体 id × 7`），用
   `skin::events_between` 找出本帧推进的 tick 中“进入带事件的帧”，以引擎 `AnimationEvent { entity, event }`
   发送——与引擎 `Animator` 的事件语义相同，只是帧的来源不同（ADR-007 后果第 2 条预留的做法）。
2. **与渲染模式无关**：只要皮肤资产已加载就发送，占位模式下也有啃食音效，两种模式的音频一致；
   皮肤加载失败时（SPEC 12.1 尽力而为）没有该音效，其余音效不受影响。
3. **补发上限**：一帧最多处理 5 个 tick（引擎一帧最多推进的 FixedUpdate 数）；进入对局的第一帧只记录 tick；
   暂停恢复后不补发暂停期间的事件。
4. **`SfxPlugin` 消费**：`sfx_system` 取走 `AnimationEvent`，`event == "eat_hit"` 映射为 `sfx.bite`，
   沿用 SPEC 13“同一 tick 同一音效最多 2 次”的上限。游戏中没有其他 `AnimationEvent` 消费者。
5. **音效**：`scripts/gen_audio.py` 新增 `sfx_bite`（两下短促的滤波噪声 + 下滑正弦，约 0.09 s，RMS 约为
   发射 / 命中音效的 1/3）；脚本新增按名称只生成指定文件的参数，避免重新编码使无关文件变动。登记 `assets/SOURCES.md`，
   SPEC 13 音效表补一行。
6. **测试**：`bite_sfx_on_eat_hit`（`tests/flow.rs`，默认占位模式，1-1 中路最右格放石籽荚：开始啃食前无
   `sfx.bite`，啃食期间至少 3 次）与单元测试 `events_between_reports_entered_event_frames`。

## 后果

- 模拟层与 core 事件未改，`determinism_golden_1_4` 与 `skin_switching_keeps_golden_1_4` 不变。
- 音效节奏跟随动画而非模拟中的实际啃食：eat 片段每 30 tick 循环一次（每只啃食中的敌人约每秒 2 次），
  模拟中的啃食间隔为 60 tick（Chill 时 120）。敌群同时啃食时音效较密，听感需负责人在实机确认；
  若需要调整，优先改 `gen_skins.py` 中 eat 片段的帧长或 `sfx_bite` 的音量，不改模拟。
- 呈现层的“当前是否在啃食”取本帧状态，开始 / 停止啃食的那一帧最多有 5 tick 的误差（纯音效，可接受）。

## 替代方案

- （1）在敌人进入 / 离开 `Eating` 时给模拟实体挂引擎 `Animator` + `Sprite`：改变 hecs 原型与查询顺序，
  违背 ADR-007 第 1 条，否决。
- （1）在 core 的 `bite_system` 中发“已啃食”事件并据此播放：需要改模拟层，违背裁决“模拟层不改”，否决。
- （2）只在皮肤模式下发送：同一局面的音频随渲染模式变化，否决。
