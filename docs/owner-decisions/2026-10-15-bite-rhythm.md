# 负责人裁决 2026-10-15（三）：啃食节奏对齐与 tag 前准备

> 发给 Rootguard 实现者。不改 SPEC。本文件只处理你汇报中提出的啃食音效节奏问题，并安排 tag 前的最后一步。

## 1. 啃食音效节奏：以数据对齐，不加限流

你的判断正确：`*_eat` 片段总长 30 tick（10 + 10 + 10），`eat_hit` 每秒 2 次；`bite_interval_ticks = 60`，伤害每秒 1 次。音效比伤害快一倍是数据不一致，不是音量问题。

决定：**把 eat 片段总长改为 60 tick，与 `bite_interval_ticks` 一致**，`eat_hit` 仍在第 2 帧。建议帧时长 `[20, 10, 30]`（抬头 → 咬下 → 停顿），具体由你定，但总和必须等于 60，且这个约束要写进 `gen_skins.py` 并由测试断言（读取 `rules`/敌人数据中的 `bite_interval_ticks`，断言每个 eat 片段 `sum(ticks) == bite_interval_ticks`）。

这样做的附带收益：Chill 下引擎 `Animator` 速度减半、`chill_bite_interval_factor = 2`，动画与伤害在冰冻时仍然对齐，无需任何额外代码。

不要做的事：

- 不要给 `sfx.bite` 加全局限流或降音量来掩盖节奏问题。现有"同一 tick 同一音效最多 2 次"（SPEC 13）保持不变。
- 不要把音效改为由 `DamageRequest`/`bite_system` 触发。SPEC 12.3 的设计意图就是"音效跟动画事件帧"，问题在数据不在机制。

验收：`bite_sfx_on_eat_hit` 仍通过；新增 `eat_clip_matches_bite_interval`；golden 不变（动画时长不进入模拟层快照，若发生变化立即停下）。一次提交：`fix(assets): eat 片段总长对齐 bite_interval_ticks（60），eat_hit 每秒一次`。

多只敌人同时啃食时的整体响度，等负责人实机试听后再说；若仍偏吵，届时另行裁决。

## 2. CHANGELOG 日期

负责人打 tag 的当天会通知你"今天打 tag"。收到后你把 `[0.1.0] - Unreleased` 改为当天日期（`YYYY-MM-DD`），提交 `docs(release): 0.1.0 发布日期`，然后停下。tag 由负责人打。

## 3. 其他确认

- 你把裁决文件从根目录移到 `docs/owner-decisions/` 并修掉文件名前导空格：正确，以后负责人放错位置照此处理。
- `tmp/venv/` 保留：同意。请在 `scripts/subset_font.py` 顶部注释写明如何重建该 venv（`python3 -m venv tmp/venv && tmp/venv/bin/pip install fonttools`），避免下一位实现者困惑。
- CI 截图失败原因"推测为缺 X11 / EGL 运行库"：你补齐依赖后通过且把日志写入步骤摘要，处理方式合格；文档中如实标注"推测"也正确。

## 4. 完成第 1 节后

回到 AGENT.md 第 9 章第 3 项，停下等待 `docs/levels.md` 的主观难度列。
