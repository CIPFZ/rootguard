# ADR-005：G4 关卡生成、调试窗口与确定性快照

- 编号：ADR-005
- 标题：G4 关卡生成、调试窗口与确定性快照
- 日期：2026-10-13
- 状态：已接受（1-9 / 1-10 的难度待负责人裁决）

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
