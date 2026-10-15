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

## 后续（2026-10-15）：eat 相位由 `Biter` 计时器驱动（负责人裁决（四））

裁决 `docs/owner-decisions/2026-10-15-chill-bite-phase.md` 取代上文后果中的两条“Chill 下不对齐”“只同频不同相”。
落地时与裁决前提的差异与处理：

1. **`Biter` 没有 `Timer`**：组件是 `elapsed`（当前周期已经过 tick）+ `interval`，`bite_system` 每 tick
   `elapsed += 1`，达到有效间隔（Chill 时 `interval × chill_bite_interval_factor`）即伤害并归零。
   因此“`Biter.timer.progress()`”以 `elapsed / 有效间隔` 实现（`render::eat_elapsed`），**呈现层需要读取
   `StatusEffects` 与 `Rules`**（裁决假设的“无需读 `StatusEffects`”在本实现中不成立），算法与 `bite_system`
   相同（`render::bite_interval`）。模拟层未改。
2. **帧序 `[30, 20, 10]`，`eat_hit` 在末帧**：`gen_skins.py` 不改图像，只改片段引用顺序——姿态 0（抬头）30、
   姿态 2（俯身）20、姿态 1（前倾最深，咬合）10 + `eat_hit`。图集逐字节不变，13 个 `*_eat.anim.ron` 变化。
3. **事件判定**：`emit_anim_events` 记录每个啃食者上一帧的 (目标, `elapsed`)；`elapsed` 未减小时检查两相位之间
   进入的事件帧，减小（其间发生过一次啃食）时把本帧相位加一个片段长度；目标变化或刚开始啃食只记录不发送。
   不再按 `fixed_tick` 补发，暂停期间 `Biter` 不推进，恢复后不会补发。
4. **偏差**：音效在计时器剩余 10 tick（Chill 下 20 tick）时触发，即裁决允许的上限；未采用“检测计时器刚触发
   直接发声”的 0 偏差做法，以保持“音效跟动画事件帧”（裁决（三）、SPEC 12.3），且咬合姿态与音效同帧出现。
5. **测试**：`bite_sfx_on_eat_hit` 改为断言每次啃食伤害之前 10 tick（Chill 20 tick）内恰有一次 `sfx.bite`、
   Chill 下伤害间隔为规则值；新增 `eat_anim_phase_follows_biter_timer`（正常与 Chill 下逐帧断言相位 =
   `elapsed × 片段总长 / 有效间隔`，且末帧对应计时器最后一段）。golden 与 `skin_switching_keeps_golden_1_4` 不变。
6. ART-SPEC 3.1 其余计时类片段（`attack` / `produce` / `fuse` / `arm`）在美术阶段 A0 按同一机制推广。
