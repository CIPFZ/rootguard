# ADR-012：eat 片段总长对齐 `bite_interval_ticks`

- 编号：ADR-012
- 标题：eat 片段总长对齐 `bite_interval_ticks`
- 日期：2026-10-15
- 状态：已接受；取代 ADR-010 后果第 2 条中“eat 片段每 30 tick 循环一次（约每秒 2 次）”

## 背景

负责人裁决 `docs/owner-decisions/2026-10-15-bite-rhythm.md` 第 1 节：eat 片段原为 10 + 10 + 10 = 30 tick，
`eat_hit` 啃食音效（ADR-010）每秒 2 次，而全部敌人的 `bite_interval_ticks = 60`，伤害每秒 1 次。
裁决认定这是数据不一致，要求片段总长改为 60 并由脚本与测试约束。

## 决策

1. **帧时长 `[20, 10, 30]`**（抬头 → 咬下 → 停顿），`eat_hit` 仍在第 2 帧；常量 `EAT_TICKS` / `EAT_HIT_FRAME`
   写在 `scripts/gen_skins.py`。脚本读取 `assets/data/enemies/*.enemy.ron` 的 `bite_interval_ticks`，
   任一敌人与 `sum(EAT_TICKS)` 不符即报错退出，不生成。
2. **测试 `eat_clip_matches_bite_interval`**（`tests/skin.rs`）：对每个敌人的 `<id>_eat` 与 `<id>_bare_eat`，
   断言片段总长 == 该敌人 `EnemyDef.bite_interval_ticks`，且每循环恰有 1 个 `eat_hit` 帧。
3. 重新生成后只有 13 个 `*_eat.anim.ron` 变化；图集 `skin.png` 逐字节不变。不加限流、不改音量、
   不改触发机制（裁决第 1 节“不要做的事”）。

## 后果

- 正常状态下 `sfx.bite` 与伤害同频（每只啃食中的敌人每秒 1 次）。模拟层未改，golden 不变。
- **Chill 下不对齐（与裁决的前提不符，已报告负责人）**：裁决称“Chill 下引擎 `Animator` 速度减半，动画与伤害
  仍然对齐，无需额外代码”。但本游戏不使用引擎 `Animator`（ADR-007 第 1 条）：皮肤帧由
  `fixed_tick + 实体 id × 7` 推导，Chill 只改变着色。因此 Chill 下伤害间隔为 120 tick，eat 动画与音效仍为
  60 tick，音效频率是伤害的 2 倍。按裁决未添加代码；若要对齐，需要呈现层按 Chill 状态缩放 eat 相位，
  属于需负责人决定的改动。
- **只同频不同相**：eat 相位按实体错开（ADR-007 第 2 条），与模拟中 `Biter.timer` 的到期时刻无关，
  `sfx.bite` 与实际扣血之间有固定的 0 ～ 59 tick 偏移。

## 替代方案

- 帧时长 `[20, 20, 20]`：节奏均匀但没有“咬下 → 停顿”的轻重，否决。
- 在测试中只断言“等于 60”：敌人数据将来若出现不同的啃食间隔，测试无法发现不一致，否决。
