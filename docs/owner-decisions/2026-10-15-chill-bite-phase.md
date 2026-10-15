# 负责人裁决 2026-10-15（四）：Chill 下啃食音效对齐、CHANGELOG 日期

> 发给 Rootguard 实现者。不改 SPEC。

## 1. Chill 下的啃食音效：以 `Biter` 计时器驱动啃食动画相位

你指出的两个问题（Chill 下频率再次失配；每只敌人 0 ～ 59 tick 的固定相位偏差）根因相同：啃食动画按游戏时钟自由运行，与真正决定伤害时刻的 `Biter.timer` 没有关系。两个可选方案都只修频率，不修相位。决定采用第三种，一次解决两者：

**渲染层把敌人处于 `Eating` 状态时的 eat 动画进度直接从 `Biter.timer.progress()` 推导**，而不是按游戏时钟推进：

- 进度 `p ∈ [0, 1)` 映射到 eat 片段的累计帧时长（总长 60 tick 的 `[20, 10, 30]` 保持不变即可，映射用 `p × 60` 落在哪一帧）。
- `eat_hit` 帧改到**最后一帧**（即 `[30, 20, 10]`，第 3 帧带 `eat_hit`），使音效在 `p` 进入最后 10 tick 时触发，与计时器归零造成伤害相差 ≤ 10 tick；若你能在渲染层拿到"本 tick 计时器刚触发"的信息，则直接在该帧触发，偏差为 0。
- Chill 下 `bite_system` 读取的间隔翻倍意味着 `progress()` 增长减半，动画与音效自动跟随，无需读 `StatusEffects`。
- 离开 `Eating` 状态时回到 walk 片段；进入时相位从 `progress()` 读出，不再错开。

约束：只改 `render.rs` / `sfx.rs` 与 `gen_skins.py` 的帧序，模拟层不动，golden 不变。`eat_clip_matches_bite_interval` 保留；`bite_sfx_on_eat_hit` 改为断言"音效触发 tick 与 `EnemyDamaged { source: Bite }` 的 tick 差 ≤ 10（Chill 下 ≤ 20）"，另加 `eat_anim_phase_follows_biter_timer`。若 `Timer::progress()` 在 Chill 下不是你预期的语义（例如计时器以 60 为周期、系统每两 tick 才推进一次），以实际实现为准写进 ADR-012 的后续条目，目标不变：音效与伤害同频同相。

一次提交：`fix(app): 啃食动画相位由 Biter 计时器驱动，音效与伤害同频同相（含 Chill）`。

## 2. CHANGELOG 日期

`0.1.0` 保持 `2026-10-14` 不改（tag 指向的提交即当日内容，差一天无实质影响）。规则固定为：**以后每个版本的日期 = 负责人通知你"今天打 tag"的当天**，由你在该通知后填写。

## 3. 完成第 1 节后

回到 AGENT.md 第 9 章第 3 项，等待 `docs/levels.md` 主观难度列。负责人已试玩并反馈"整体不错"，具体每关的数据稍后填入；在此之前不要自行校准任何关卡。
