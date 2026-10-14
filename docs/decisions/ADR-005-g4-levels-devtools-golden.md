# ADR-005：G4 关卡生成、调试窗口与确定性快照

- 编号：ADR-005
- 标题：G4 关卡生成、调试窗口与确定性快照
- 日期：2026-10-13
- 状态：已接受（负责人裁决 2026-10-14：[`docs/owner-decisions/2026-10-14-g4-levels.md`](../owner-decisions/2026-10-14-g4-levels.md)；
  按 SPEC v1.1 的返工方法与结果见 ADR-006 与下文“裁决后的重做结果”）

## 背景

实现 SPEC.md 10.2（1-4 ～ 1-10）、11.5（调试扩展）与 16.2（`determinism_golden_1_4`）时的未定之处。

## 决策

1. **波次初稿用脚本生成**（`scripts/gen_levels.py`，设计期运行一次，输出提交为 RON 数据），
   保证每波威胁与预算逐一相等；之后的调整记录在 `docs/levels.md`。
2. **flag**：只有最后一波为 flag（SPEC 4.5 的最低要求）；中段 flag 在第一次调整中取消。
3. **车道逐步打开**：第 1 波 `Fixed(2)`，第 2 波 `Fixed(1)` / `Fixed(3)`，之后 `Spread` / `Random`。
4. **脚本解的卡组**必须是进入该关时已解锁的卡牌（初始卡牌 + 之前各关 `unlock_on_win`），
   `every_level_has_scripted_solution` 同时检查这一点。
5. **调试窗口**（feature `devtools`）与引擎面板一起由 F1 开关；Dew +100、跳到下一波、立即胜负
   直接修改 `BattleState` / `WaveState`。这是开发工具，不经 `UiAction`（SPEC 11.1 的枚举中没有调试动作），
   发布构建（`--no-default-features`）中不存在。
6. **golden 快照** = 引擎 `HeadlessRunner::snapshot()`（`Transform2D`）+ `rootguard_core::snapshot::write`
   （对局资源与全部模拟组件，按实体 id 升序、小端序）。回放 30000 帧（每帧 1 tick），1-4 于 17597 tick
   胜利后对局停止推进，快照反映最终状态。debug 与 release 构建逐字节一致。

## 后果

- golden 首次生成后只读；任何改变 1-4 模拟结果的改动（规则、数据、脚本）都需先向负责人说明。

## 替代方案

- （5）为调试动作扩展 `UiAction`：改变 SPEC 11.1 规定的枚举，否决。

## 裁决后的重做结果（2026-10-14）

负责人判定 1-10 不可解的根因是 SPEC 10.2 预算公式后期增长过快，SPEC v1.1 增加预算上限阶梯（10.2）
与脚本解 Hose ≤ 1（10.3）。本 ADR 的第 1 ～ 4 条与第 6 条保持不变；`gen_levels.py` 增加
`--tier / --cap / --flag-cap`。重做结果（详见 `docs/levels.md`）：

| 关 | 档位 | 脚本解 tick | Hose（重做前 → 后） |
|---|---|---|---|
| 1-3 | —（只重录脚本解，ADR-006） | 14254 | 2 → 1 |
| 1-7 | 1（上限未触发，波次不变） | 21954 | 2 → 0 |
| 1-8 | 1 | 24137 | 2 → 0 |
| 1-9 | 2（第 1 档无合格解） | 30565 | 5 → 1 |
| 1-10 | 1 | 31731 | 无解 → 1 |

1-4 的 golden 快照未变。通关解锁调整为 1-9 → snapmaw + sunlantern、1-10 → gustbloom。
