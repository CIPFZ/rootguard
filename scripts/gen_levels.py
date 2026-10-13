#!/usr/bin/env python3
"""关卡波次设计辅助（SPEC 10.2，AGENT 3.4）：按威胁预算填写 1-4 ～ 1-10 的 waves 并写出 RON。

预算：第 k 波 ceil(base × k^1.15)，base 从 1-1 的 1 线性增至 1-10 的 3；flag 波 ×2。
只有最后一波为 flag（中段 flag 在调整中取消，见 docs/levels.md）。新敌人不早于第 3 波。
车道逐步打开：第 1 波只走中路（Fixed(2)），第 2 波走 1、3 路，之后分散到全部车道。
本脚本只在设计期运行一次，输出提交为数据；之后的调整直接改 RON 并记录在 docs/levels.md。

    python3 scripts/gen_levels.py            # 写出 assets/data/levels/1-4 ～ 1-10
    python3 scripts/gen_levels.py --dry-run  # 只打印预算表
"""

import math
import os
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")

THREAT = {
    "husk": 1, "twig_husk": 2, "clay_husk": 4, "sprinter_husk": 2, "vault_husk": 3,
    "shield_husk": 4, "carrier_husk": 3, "drum_husk": 6,
}

# 关序号 → (波数, 新敌人, 起始 Dew, 卡槽, 通关解锁, 可用敌人池)
LEVELS = {
    4: (8, "clay_husk", 50, 6, ["twinpod"], ["husk", "twig_husk"]),
    5: (9, "sprinter_husk", 50, 6, ["thornmat"], ["husk", "twig_husk", "clay_husk"]),
    6: (10, "vault_husk", 50, 6, ["tuberblast"], ["husk", "twig_husk", "clay_husk", "sprinter_husk"]),
    7: (10, "shield_husk", 50, 7, ["emberbulb"], ["husk", "twig_husk", "clay_husk", "sprinter_husk", "vault_husk"]),
    8: (11, "carrier_husk", 50, 7, ["gourdshell"], ["husk", "twig_husk", "clay_husk", "sprinter_husk", "vault_husk", "shield_husk"]),
    9: (12, "drum_husk", 50, 8, ["snapmaw", "gustbloom"], ["husk", "twig_husk", "clay_husk", "sprinter_husk", "vault_husk", "shield_husk", "carrier_husk"]),
    10: (14, None, 75, 8, ["sunlantern"], ["husk", "twig_husk", "clay_husk", "sprinter_husk", "vault_husk", "shield_husk", "carrier_husk", "drum_husk"]),
}

INTENT = {
    4: "五车道", 5: "速度威胁", 6: "跨越对策", 7: "高护甲", 8: "衍生", 9: "重型", 10: "阶段终局",
}


def base(n):
    return 1 + 2 * (n - 1) / 9


# 中段 flag 只用于波数 ≥ MID_FLAG_MIN_WAVES 的关卡（SPEC 只要求最后一波为 flag）
MID_FLAG_MIN_WAVES = 99


def flags(waves):
    return {waves - 1} | ({math.ceil(waves / 2) - 1} if waves >= MID_FLAG_MIN_WAVES else set())


def budgets(n, waves):
    fl = flags(waves)
    return [math.ceil(base(n) * k ** 1.15) * (2 if k - 1 in fl else 1) for k in range(1, waves + 1)]


def fill(budget, pool, new, k):
    """返回 [(enemy, count)]，威胁总和恰为 budget。"""
    out = {}
    left = budget
    if new and k >= 3 and THREAT[new] <= left:
        c = max(1, round(budget * 0.25 / THREAT[new]))
        c = min(c, left // THREAT[new])
        out[new] = c
        left -= c * THREAT[new]
    # 其余：除 husk 外的敌人按威胁从高到低各分约 1/(n+1)，余数用 husk 补齐
    others = sorted((e for e in pool if e != "husk"), key=lambda e: -THREAT[e])
    if k >= 2:
        share = left / (len(others) + 2) if others else 0
        for e in others:
            c = int(share // THREAT[e])
            if c > 0 and c * THREAT[e] <= left:
                out[e] = out.get(e, 0) + c
                left -= c * THREAT[e]
    if left > 0:
        out["husk"] = out.get("husk", 0) + left
    return [(e, c) for e, c in out.items() if c > 0]


FIRST_DELAY = 3600
WAVE_DELAY = 2700
FLAG_DELAY = 3000


def delays(n, waves):
    fl = flags(waves)
    out = []
    for i in range(waves):
        if i == 0:
            out.append(FIRST_DELAY)
        elif i in fl:
            out.append(FLAG_DELAY)
        else:
            out.append(WAVE_DELAY)
    return out


def level_ron(n):
    waves, new, dew, slots, unlock, pool = LEVELS[n]
    full_pool = pool + ([new] if new else [])
    b = budgets(n, waves)
    fl = flags(waves)
    d = delays(n, waves)
    lines = [
        f"// SPEC 10.2 关卡 1-{n}：{INTENT[n]}。base {base(n):.3f}，预算 {b}（flag 波已 ×2，docs/levels.md）。",
        "// 由 scripts/gen_levels.py 按预算生成初稿，调整记录见 docs/levels.md。",
        "Level(",
        f'    id: "1-{n}",',
        f"    seed: {100 + n},",
        "    lanes: [0, 1, 2, 3, 4],",
        f"    start_dew: {dew},",
        "    sky_dew: true,",
        f"    card_slots: {slots},",
        "    cards: Choose,",
        "    unlock_on_win: [" + ", ".join(f'"{u}"' for u in unlock) + "],",
        "    waves: [",
    ]
    for k in range(1, waves + 1):
        spawns = fill(b[k - 1], full_pool, new, k)
        parts = []
        for j, (e, c) in enumerate(spawns):
            # 车道逐步打开：第 1 波只走中路，第 2 波走 1、3 路，之后分散到全部车道
            if k == 1:
                parts.append(f'(enemy: "{e}", count: {c}, lane: Fixed(2))')
            elif k == 2:
                lo, hi = c - c // 2, c // 2
                if lo:
                    parts.append(f'(enemy: "{e}", count: {lo}, lane: Fixed({1 if j % 2 == 0 else 3}))')
                if hi:
                    parts.append(f'(enemy: "{e}", count: {hi}, lane: Fixed({3 if j % 2 == 0 else 1}))')
            else:
                lane = "Spread" if c >= 2 else "Random"
                parts.append(f'(enemy: "{e}", count: {c}, lane: {lane})')
        flag = "true, " if k - 1 in fl else "false,"
        lines.append(f"        Wave(delay_ticks: {d[k - 1]}, flag: {flag} spawns: [{', '.join(parts)}]),")
    lines += ["    ],", ")"]
    return "\n".join(lines) + "\n", b


def main():
    dry = "--dry-run" in sys.argv
    for n in LEVELS:
        text, b = level_ron(n)
        print(f"1-{n}: {b}")
        if not dry:
            with open(os.path.join(ROOT, "assets", "data", "levels", f"1-{n}.level.ron"), "w", encoding="utf-8") as f:
                f.write(text)


if __name__ == "__main__":
    main()
