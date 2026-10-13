# ADR-003：G2 对局场景、HUD 与脚本解的实现细节

- 编号：ADR-003
- 标题：G2 对局场景、HUD 与脚本解的实现细节
- 日期：2026-10-13
- 状态：已接受

## 背景

实现 SPEC.md 11.1 / 11.2 / 12.2 与 16.2 时遇到以下未写清之处。

## 决策

1. **`UiAction` 收件箱**：引擎事件只存活到下一帧末，而窗口模式下一帧可能执行 0 个 tick，
   在 Update 中发出的对局动作可能在被 FixedUpdate 读到前就被丢弃。`BattlePlugin` 在 `PreUpdate`
   注册 `buffer_ui_actions`，把对局动作移入 `UiInbox` 资源，`ui_action_system` 先读收件箱再读事件；
   场景动作放回事件队列，由当前场景在 Update 中取走。headless 下（每帧恰好 1 tick）行为不变。
2. **HUD 的视觉画在 `DrawList`**（Dew 计数、卡槽、冷却遮罩、波次进度、横幅），随视口 letterbox 缩放，
   headless 可检查；卡槽点击由 `BattleScene::update` 按 SPEC 11.1 翻译为 `SelectCard`。铲子与暂停为
   egui 按钮（SPEC 11.2“egui”），按 `Viewport` 把逻辑矩形映射为 egui 点后放置；按钮只发 `UiAction`。
3. **Dew 画在 HUD 之上**（`Layer::OVERLAY + 1`）：天空 Dew 从 y = -40 落下会经过 HUD 区域，需可见可点。
4. **行遮挡**：E-3 尚未合入游戏所用的引擎 rev，占位渲染按 SPEC 12.2 的回退方案每行一个 `Layer(WORLD + row)`。
5. **G2 的结算**：Outcome / Pause 场景属于 G3。G2 由 `BattleScene` 自绘胜负覆盖层（回车重开、Esc 回标题），
   `PauseToggle` 暂不处理。
6. **命令行**：`--level <id>`（`Choose` 关未选卡时默认卡组为注册表顺序的全部防御者，由 `start_battle`
   截取前 `card_slots` 张）、`--seed <n>`、`--autoplay <script.ron>`。资产根目录由游戏解析：
   `TRELLIA_ASSETS` → 可执行文件同级 → 当前目录 → 编译期工作区 `assets/`（直接运行 `target/` 下的二进制时）。
7. **脚本解格式** `tests/scripts/<id>.ron`：`Script(level, cards, max_ticks, steps: [Step(tick, actions)])`，
   `tick` 为发出时的 `level_tick`（下一 tick 生效），动作为 `Select / Deselect / Place / Shovel / Collect`
   （对局 `UiAction` 的可序列化子集；`UiAction` 本身含 `Vec2`，不为它引入 glam 的 serde feature）。
   脚本由测试目录内的启发式机器人录制（`ROOTGUARD_RECORD=1`），回放测试只读 RON。
   `--autoplay` 在 FixedUpdate 中把到期动作直接放入 `UiInbox`，与 headless 回放逐 tick 一致。
8. **SPEC 16.2 `level_1_1_scripted_win` 的“第 60 tick”** 指 `level_tick`（ADR-002 第 10 条）；
   测试同时每 tick 收集全部 Dew（否则 Dew 不足以“每次 ready 且 Dew 够时”放置）。

## 后果

- 对局动作不会因帧率抖动丢失；场景动作仍是“当帧消费”。
- E-3 合入后只需把 `render::row_layer` 换成 `*_sorted` 调用。

## 替代方案

- （1）场景在 Update 中把对局动作直接写入 `PendingOps`：绕过 `ui_action_system` 的选卡校验，否决。
- （2）卡槽也用 egui 按钮：egui 坐标不随 letterbox 缩放的字号难以与画面一致，且 headless 无法点击，否决。
