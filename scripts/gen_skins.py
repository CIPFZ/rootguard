#!/usr/bin/env python3
"""Rootguard 原创皮肤程序化生成（SPEC 12.3）。

扁平几何风：每个角色 ≤ 6 个基本形状（椭圆 / 矩形 / 多边形）+ 粗轮廓，低饱和暖色调；
眼睛只用两个小圆点。全部造型由本脚本从零绘制，不参照任何既有作品。
为与同类型游戏的常见造型保持距离（SPEC 1.2 / 12.3，ADR-007）：射手是托着石子的“投石杯”，
不画侧面开口；墙不画眼睛；块茎雷武装后以裂纹发光表示；跳栏枯壳用弹簧腿而不是撑杆。

输出 assets/skins/default/：
    skin.png              图集（96 × 96 一格，4 倍超采样后缩小抗锯齿）
    skin.atlas.ron        引擎图集 RON（帧名 <id>_<动作>_<序号>；敌人每帧另有 <帧名>_flash 受伤闪白版）
    skin.ron              皮肤清单：图集及其全部动画片段名（rootguard-app 据此加载，格式同 tools/artgen）
    anim/<id>_<动作>.anim.ron  引擎动画片段 RON

动作：防御者 idle 4 帧（射手 / 咬夹草另有 attack 3 帧，块茎雷另有 armed 2 帧）；敌人 walk 4 帧、
eat 3 帧（末帧带 `eat_hit` 事件），有护甲的敌人另有 bare_walk / bare_eat（护甲破损后）。
eat 片段总长必须等于该敌人的 `bite_interval_ticks`（读取 assets/data/enemies/*.enemy.ron，不符即报错），
使每循环一次的 `eat_hit` 啃食音效与模拟中的伤害同频（负责人裁决 2026-10-15（三），ADR-012）；
呈现层按 `Biter` 计时器进度取帧，`eat_hit` 在末帧即与计时器到期同相（裁决（四））。
弹丸与 Dew 各 1 帧，锚点在中心；其余锚点在底部中心。

需要 Pillow（开发期工具）：python3 scripts/gen_skins.py
"""

import glob
import math
import os
import re

from PIL import Image, ImageDraw

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
OUT = os.path.join(ROOT, "assets", "skins", "default")
CELL = 96
SS = 4  # 超采样
# eat 片段：抬头 → 俯身 → 咬合（eat_hit，末帧）；总和须等于 bite_interval_ticks（裁决（四））。
# EAT_POSES 为每个片段帧使用的姿态图序号（姿态 1 前倾最深，即咬合）。
EAT_POSES = (0, 2, 1)
EAT_TICKS = (30, 20, 10)
EAT_HIT_FRAME = 2
OUTLINE = (52, 40, 32, 255)
LINE_W = 3


def rgb(hexv, a=255):
    return ((hexv >> 16) & 255, (hexv >> 8) & 255, hexv & 255, a)


class Pen:
    """在一格画布上以 96 × 96 坐标作图（内部 4 倍超采样）。"""

    def __init__(self):
        self.img = Image.new("RGBA", (CELL * SS, CELL * SS), (0, 0, 0, 0))
        self.d = ImageDraw.Draw(self.img)

    def _s(self, pts):
        return [(x * SS, y * SS) for x, y in pts]

    def ellipse(self, cx, cy, rx, ry, fill, outline=True):
        box = [(cx - rx) * SS, (cy - ry) * SS, (cx + rx) * SS, (cy + ry) * SS]
        self.d.ellipse(box, fill=rgb(fill), outline=OUTLINE if outline else None, width=LINE_W * SS if outline else 0)

    def rect(self, x0, y0, x1, y1, fill, radius=0, outline=True):
        box = [x0 * SS, y0 * SS, x1 * SS, y1 * SS]
        self.d.rounded_rectangle(box, radius=radius * SS, fill=rgb(fill),
                                 outline=OUTLINE if outline else None, width=LINE_W * SS if outline else 0)

    def poly(self, pts, fill, outline=True):
        self.d.polygon(self._s(pts), fill=rgb(fill), outline=OUTLINE if outline else None,
                       width=LINE_W * SS if outline else 0)

    def line(self, pts, width=LINE_W, color=None):
        self.d.line(self._s(pts), fill=color or OUTLINE, width=width * SS, joint="curve")

    def eyes(self, cx, cy, gap=7, r=2.6):
        for dx in (-gap / 2, gap / 2):
            self.ellipse(cx + dx, cy, r, r, 0x2A1F18, outline=False)

    def done(self):
        return self.img.resize((CELL, CELL), Image.LANCZOS)


# ------------------------------------------------------------------ 防御者
# 约定：底部 y = 92 为地面；角色水平居中 x = 48，面向右侧。

def sling_cup(p, x, cy, cup, pebble, fired, scale=1.0):
    """投石杯：斜向右上的杯形荚托着一颗石子；fired 时石子已射出。"""
    w, h = 20 * scale, 14 * scale
    p.poly([(x - w, cy - h * 0.4), (x + w, cy - h), (x + w * 0.7, cy + h * 0.6), (x - w * 0.6, cy + h)], cup)
    if not fired:
        p.ellipse(x + w * 0.35, cy - h * 0.9, 8 * scale, 8 * scale, pebble)


def shooter(f, attack, cup, pebble, stem=0x6E8B4E, leaf=0x7FA35A, pebble_poly=False):
    """单杯射手：茎、叶、投石杯、石子、眼。attack 0 / 1 / 2 = 后仰蓄力 / 弹出（石子已射出）/ 回位。"""
    p = Pen()
    sway = math.sin(f * math.pi / 2) * 1.5
    tilt = [-3, 4, 1][attack] if attack is not None else 0
    x = 46 + sway + tilt
    p.rect(44, 60, 52, 92, stem)                                         # 茎
    p.ellipse(32, 82, 12, 5, leaf)                                       # 叶
    sling_cup(p, x, 46, cup, pebble, fired=attack == 1)
    if pebble_poly and attack != 1:                                      # 冰晶石子
        px, py = x + 7, 34
        p.poly([(px, py - 9), (px + 7, py), (px, py + 9), (px - 7, py)], pebble)
    p.eyes(x - 6, 52, gap=7, r=2.4)
    return p.done()


def pebble_pod(f, attack=None):
    """石籽荚：棕色投石杯托着一颗青灰石子。"""
    return shooter(f, attack, cup=0x9C7A4E, pebble=0x9CA88C)


def frostpod(f, attack=None):
    """霜籽荚：蓝灰投石杯托着一枚菱形冰晶。"""
    return shooter(f, attack, cup=0x7FA3B3, pebble=0xD4ECF2, stem=0x6E8B7E, leaf=0x86A99A, pebble_poly=True)


def twinpod(f, attack=None):
    """双籽荚：分叉的茎上一高一低两个小投石杯。"""
    p = Pen()
    sway = math.sin(f * math.pi / 2) * 1.5
    tilt = [-2, 3, 1][attack] if attack is not None else 0
    x = 48 + sway + tilt
    p.poly([(44, 92), (52, 92), (52, 60), (60, 40), (54, 38), (46, 56)], 0x5F7F44)   # 分叉茎
    sling_cup(p, x + 8, 30, 0x8E7448, 0x9CA88C, fired=attack == 1, scale=0.8)
    sling_cup(p, x - 6, 58, 0x8E7448, 0x9CA88C, fired=attack == 2, scale=0.8)
    p.eyes(x + 2, 34, gap=6, r=2.2)
    return p.done()


def dewcup(f):
    """露杯：杯形花冠托着一滴露水，两片叶。"""
    p = Pen()
    bob = math.sin(f * math.pi / 2) * 2
    p.rect(45, 60, 51, 92, 0x6E8B4E)
    p.ellipse(34, 78, 12, 5, 0x7FA35A)
    p.ellipse(62, 72, 12, 5, 0x7FA35A)
    p.poly([(26, 34), (70, 34), (62, 62), (34, 62)], 0xD9A441)            # 杯
    p.ellipse(48, 34, 22, 6, 0xE8BE62)                                  # 杯口
    p.poly([(48, 10 + bob), (56, 26 + bob), (48, 32 + bob), (40, 26 + bob)], 0xA9D6E5)  # 露滴
    return p.done()


def barkwall(f):
    """树皮墙：直立的圆木段，顶部年轮，两道树皮纹。"""
    p = Pen()
    lean = math.sin(f * math.pi / 2) * 0.8
    p.rect(22 + lean, 18, 74 + lean, 92, 0x8D6E4F, radius=10)
    p.ellipse(48 + lean, 20, 26, 7, 0xC9A57A)                           # 年轮面
    p.ellipse(48 + lean, 20, 12, 3, 0xA9845B, outline=False)
    p.line([(30 + lean, 50), (40 + lean, 58)], width=2)
    p.line([(60 + lean, 70), (68 + lean, 62)], width=2)
    p.line([(36 + lean, 76), (44 + lean, 84)], width=2)
    return p.done()


def thornmat(f):
    """荆棘垫：贴地的扁平藤垫，上面一排三角棘刺。"""
    p = Pen()
    rise = (f % 2) * 1.5
    p.ellipse(48, 84, 40, 9, 0x6B5B3E)
    pts = []
    for i in range(6):
        x = 16 + i * 13
        pts.append([(x - 5, 82), (x, 66 - rise), (x + 5, 82)])
    for tri in pts:
        p.poly(tri, 0xB59A6A)
    return p.done()


def tuberblast(f, armed=False):
    """块茎雷：半埋的芜菁形块根、顶部叶簇；武装后出现发光环。"""
    p = Pen()
    p.ellipse(48, 90, 34, 6, 0x5A4632)                                  # 土堆
    p.ellipse(48, 76, 20, 16, 0xA0522D)                                 # 块根
    p.poly([(44, 62), (36, 44), (48, 56), (60, 44), (52, 62)], 0x6E8B4E)  # 叶簇
    if armed:
        crack = [(34, 72), (42, 80), (48, 70), (55, 82), (62, 74)]
        p.line(crack, width=4 + (f % 2), color=rgb(0xFFB347))
        p.line(crack, width=2, color=rgb(0xFFF1C2))
    p.eyes(48, 72 if armed else 76, gap=9, r=2.4)
    return p.done()


def emberbulb(f):
    """炽球：洋葱形鳞茎，顶端一簇火苗。"""
    p = Pen()
    flick = (f % 2) * 3
    p.ellipse(48, 66, 24, 24, 0xE4572E)
    p.poly([(40, 46), (48, 22 - flick), (56, 46)], 0xFFB347)            # 火苗
    p.poly([(44, 46), (48, 34 - flick), (52, 46)], 0xFFE08A, outline=False)
    p.ellipse(48, 90, 20, 4, 0x5A4632)
    p.eyes(48, 68, gap=10, r=3)
    return p.done()


def gourdshell(f):
    """葫芦壳：覆盖层，围在宿主外的编织葫芦环（中空）。"""
    p = Pen()
    w = 1 + (f % 2)
    p.d.rounded_rectangle([6 * SS, 10 * SS, 90 * SS, 94 * SS], radius=22 * SS, outline=rgb(0xB7A96A), width=(8 + w) * SS)
    p.d.rounded_rectangle([6 * SS, 10 * SS, 90 * SS, 94 * SS], radius=22 * SS, outline=OUTLINE, width=LINE_W * SS)
    for y in (30, 52, 74):
        p.line([(4, y), (14, y + 6)], width=2)
        p.line([(92, y), (82, y + 6)], width=2)
    return p.done()


def snapmaw(f, attack=None):
    """咬夹草：两片橄榄色夹叶（内侧锈红），短茎。"""
    p = Pen()
    open_ = [10, 2, 6][attack] if attack is not None else 8 + math.sin(f * math.pi / 2) * 2
    p.rect(44, 62, 52, 92, 0x5F7F44)
    p.ellipse(34, 84, 12, 5, 0x6F9A4A)
    p.poly([(30, 44 - open_), (78, 40 - open_ * 1.6), (70, 50), (34, 52)], 0x8A9A4B)   # 上颚
    p.poly([(30, 52), (70, 52), (78, 62 + open_ * 0.6), (34, 62)], 0x8A9A4B)            # 下颚
    p.poly([(38, 50), (66, 50), (66, 54), (38, 54)], 0x9E4A3A, outline=False)
    p.eyes(42, 40 - open_ * 0.6, gap=7, r=2.2)
    return p.done()


def gustbloom(f):
    """风铃花：钟形花冠，旋涡纹。"""
    p = Pen()
    swing = math.sin(f * math.pi / 2) * 3
    p.rect(45, 56, 51, 92, 0x6E8B4E)
    p.poly([(30 + swing, 58), (66 + swing, 58), (58 + swing, 24), (38 + swing, 24)], 0x80CBC4)
    p.ellipse(48 + swing, 58, 18, 5, 0xA6DDD7)
    p.d.arc([(40 + swing) * SS, 32 * SS, (56 + swing) * SS, 48 * SS], 30, 300, fill=OUTLINE, width=2 * SS)
    return p.done()


def sunlantern(f):
    """日灯笼：灯笼形果实，顶盖与发光芯。"""
    p = Pen()
    glow = (f % 4) * 1.2
    p.rect(45, 70, 51, 92, 0x6E8B4E)
    p.rect(30, 30, 66, 72, 0xFF9F1C, radius=14)
    p.rect(36, 22, 60, 32, 0x8D6E4F, radius=3)
    p.ellipse(48, 51, 8 + glow, 8 + glow, 0xFFE08A, outline=False)
    p.eyes(48, 60, gap=10, r=2.2)
    return p.done()


# ------------------------------------------------------------------ 敌人
# 枯壳怪：麻绳捆扎的干枯苞叶身体，面向左。walk 帧摆腿，eat 帧前倾。

def husk_body(p, f, eat, body=0xC8B89A, lean=0.0, width=1.0, height=1.0, legs=True):
    step = [0, 3, 0, -3][f % 4] if eat is None else 0
    tilt = [0, -5, -2][eat] if eat is not None else 0
    cx = 50 + lean + tilt
    top = 92 - 72 * height
    if legs:
        p.line([(cx - 6, 74), (cx - 10 - step, 92)], width=4)          # 腿
        p.line([(cx + 6, 74), (cx + 8 + step, 92)], width=4)
    p.poly([(cx - 16 * width, 76), (cx - 12 * width, top + 14), (cx, top), (cx + 12 * width, top + 14), (cx + 16 * width, 76)], body)  # 苞叶身
    p.line([(cx - 14 * width, 56), (cx + 14 * width, 56)], width=2, color=rgb(0x7A5A3A))  # 麻绳
    arm_y = 50 if eat is None else 46 + (eat or 0) * 2
    p.line([(cx - 8, arm_y), (cx - 24, arm_y + 6 - (eat or 0) * 3)], width=4)  # 手臂
    p.eyes(cx - 6, top + 22, gap=7, r=2.4)
    return cx, top


def husk(f, eat=None, armored=True):
    p = Pen()
    husk_body(p, f, eat)
    return p.done()


def twig_husk(f, eat=None, armored=True):
    p = Pen()
    cx, top = husk_body(p, f, eat, body=0xBDAA88)
    if armored:
        for dx, h in ((-8, 14), (0, 20), (8, 12)):
            p.line([(cx + dx, top + 6), (cx + dx * 1.6, top - h)], width=3, color=rgb(0x6B4A2E))
    return p.done()


def clay_husk(f, eat=None, armored=True):
    p = Pen()
    cx, top = husk_body(p, f, eat, body=0xC2B08E)
    if armored:
        p.d.chord([(cx - 18) * SS, (top - 6) * SS, (cx + 18) * SS, (top + 26) * SS], 180, 360, fill=rgb(0xB0705A), outline=OUTLINE, width=LINE_W * SS)
    return p.done()


def sprinter_husk(f, eat=None, armored=True):
    p = Pen()
    husk_body(p, f * 2 if eat is None else f, eat, body=0xB89BB8, lean=-6, width=0.8)
    p.line([(30, 40), (22, 40)], width=2, color=rgb(0xE0D0E0))
    p.line([(32, 48), (20, 48)], width=2, color=rgb(0xE0D0E0))
    return p.done()


def vault_husk(f, eat=None, armored=True):
    """跳栏枯壳：苞叶身下是两条弹簧腿（Vault 用掉后仍保留，SPEC 8.1 的跳栏不是护甲）。"""
    p = Pen()
    cx, top = husk_body(p, f, eat, body=0xA9B0CC, legs=False)
    squash = [0, 2, 0, -2][f % 4] if eat is None else 0
    for dx in (-7, 7):
        zig = [(cx + dx, 74)]
        for i in range(1, 5):
            zig.append((cx + dx + (4 if i % 2 else -4), 74 + i * (4.5 + squash * 0.25)))
        zig.append((cx + dx, 92))
        p.line(zig, width=3, color=rgb(0x5E6680))
    return p.done()


def shield_husk(f, eat=None, armored=True):
    p = Pen()
    cx, top = husk_body(p, f, eat, body=0xB8B4A6)
    if armored:
        p.rect(cx - 30, top + 4, cx - 18, 88, 0x9AA5AC, radius=2)          # 门板
        p.line([(cx - 28, top + 24), (cx - 20, top + 24)], width=2)
        p.line([(cx - 28, top + 52), (cx - 20, top + 52)], width=2)
    return p.done()


def carrier_husk(f, eat=None, armored=True):
    p = Pen()
    cx, top = husk_body(p, f, eat, body=0xB5B894)
    p.rect(cx + 10, top + 12, cx + 30, top + 44, 0x8C9A62, radius=3)       # 背篓
    p.line([(cx + 12, top + 24), (cx + 28, top + 24)], width=2)
    p.ellipse(cx + 20, top + 10, 5, 5, 0xAED581)                           # 篓里的小芽
    return p.done()


def drum_husk(f, eat=None, armored=True):
    p = Pen()
    cx, top = husk_body(p, f, eat, body=0xA88E74, width=1.35, height=0.92)
    if armored:
        p.ellipse(cx, 62, 20, 16, 0x8D5A3A)                                  # 鼓腹
        p.ellipse(cx, 62, 12, 9, 0xD9B48A, outline=False)
    return p.done()


def sproutling(f, eat=None, armored=True):
    p = Pen()
    step = [0, 2, 0, -2][f % 4] if eat is None else 0
    tilt = [0, -4, -2][eat] if eat is not None else 0
    cx = 50 + tilt
    p.line([(cx - 4, 82), (cx - 6 - step, 92)], width=3)
    p.line([(cx + 4, 82), (cx + 6 + step, 92)], width=3)
    p.ellipse(cx, 72, 13, 13, 0x9CC26A)
    p.poly([(cx - 2, 60), (cx - 12, 46), (cx, 54), (cx + 10, 44), (cx + 4, 60)], 0x6F9A4A)
    p.eyes(cx - 4, 70, gap=6, r=2)
    return p.done()


# ------------------------------------------------------------------ 其他

def pebble_shot(f):
    p = Pen()
    p.ellipse(48, 48, 9, 8, 0x9CCC65)
    return p.done()


def frost_shot(f):
    p = Pen()
    p.poly([(48, 38), (58, 48), (48, 58), (38, 48)], 0x90CAF9)
    return p.done()


def dew(f):
    """Dew：金色水滴（圆 + 尖顶）与浅色微光。"""
    p = Pen()
    p.ellipse(48, 50, 22, 22, 0xFFF3B0, outline=False)
    p.poly([(48, 24), (60, 46), (36, 46)], 0xFFD84D, outline=False)
    p.ellipse(48, 54, 13, 13, 0xFFD84D, outline=False)
    p.ellipse(44, 52, 4, 5, 0xFFF8DC, outline=False)
    return p.done()


DEFENDERS = {
    "pebble_pod": (pebble_pod, True),
    "dewcup": (dewcup, False),
    "barkwall": (barkwall, False),
    "frostpod": (frostpod, True),
    "twinpod": (twinpod, True),
    "thornmat": (thornmat, False),
    "tuberblast": (tuberblast, False),
    "emberbulb": (emberbulb, False),
    "gourdshell": (gourdshell, False),
    "snapmaw": (snapmaw, True),
    "gustbloom": (gustbloom, False),
    "sunlantern": (sunlantern, False),
}

# id → (绘制函数, 是否有护甲；与敌人 RON 的 armor 一致，有护甲才生成 bare_ 破损版)
ENEMIES = {
    "husk": (husk, False),
    "twig_husk": (twig_husk, True),
    "clay_husk": (clay_husk, True),
    "sprinter_husk": (sprinter_husk, False),
    "vault_husk": (vault_husk, False),
    "shield_husk": (shield_husk, True),
    "carrier_husk": (carrier_husk, False),
    "drum_husk": (drum_husk, True),
    "sproutling": (sproutling, False),
}

OTHER = {"pebble": pebble_shot, "frost_pebble": frost_shot, "dew": dew}


# 循环播放的 attack 片段（咬夹草咀嚼期间持续播放；射手的 attack 只在发射后播放一次）
LOOPING_ATTACK = {"snapmaw"}
# 受伤闪白：向白色混合的比例（与占位渲染的 0.7 一致，SPEC 12.2）
FLASH_MIX = 0.7


def flash(img):
    """受伤闪白版：保留透明度，颜色向白色混合 FLASH_MIX。"""
    r, g, b, a = img.split()
    white = Image.new("L", img.size, 255)
    mix = [Image.blend(ch, white, FLASH_MIX) for ch in (r, g, b)]
    return Image.merge("RGBA", (*mix, a))


def bite_intervals():
    """敌人 id → bite_interval_ticks（来自 assets/data/enemies/*.enemy.ron）。"""
    out = {}
    for path in glob.glob(os.path.join(ROOT, "assets", "data", "enemies", "*.enemy.ron")):
        text = open(path, encoding="utf-8").read()
        eid = re.search(r'\bid:\s*"([a-z0-9_]+)"', text)
        interval = re.search(r"\bbite_interval_ticks:\s*(\d+)", text)
        if not (eid and interval):
            raise SystemExit(f"gen_skins: 无法从 {path} 读取 id / bite_interval_ticks")
        out[eid.group(1)] = int(interval.group(1))
    return out


def frames():
    """生成全部 (帧名, 图像, 锚点) 与动画片段定义 [(片段名, looping, [(帧名, ticks, 事件)])]。"""
    out = []
    clips = []

    def add(name, img, anchor=(0.5, 1.0)):
        out.append((name, img, anchor))

    for did, (fn, attacks) in DEFENDERS.items():
        idle = []
        for f in range(4):
            name = f"{did}_idle_{f}"
            add(name, fn(f))
            idle.append((name, 15, None))
        clips.append((f"{did}_idle", True, idle))
        if attacks:
            atk = []
            for a in range(3):
                name = f"{did}_attack_{a}"
                add(name, fn(0, attack=a))
                atk.append((name, 6, None))
            clips.append((f"{did}_attack", did in LOOPING_ATTACK, atk))
        if did == "tuberblast":
            armed = []
            for f in range(2):
                name = f"{did}_armed_{f}"
                add(name, tuberblast(f, armed=True))
                armed.append((name, 20, None))
            clips.append((f"{did}_armed", True, armed))
    intervals = bite_intervals()
    for eid, (fn, armored) in ENEMIES.items():
        if sum(EAT_TICKS) != intervals.get(eid):
            raise SystemExit(f"gen_skins: eat 片段总长 {sum(EAT_TICKS)} ≠ {eid} 的 bite_interval_ticks {intervals.get(eid)}")
        variants = [("", True)] + ([("bare_", False)] if armored else [])
        for prefix, arm in variants:
            walk = []
            for f in range(4):
                name = f"{eid}_{prefix}walk_{f}"
                img = fn(f, armored=arm)
                add(name, img)
                add(f"{name}_flash", flash(img))
                walk.append((name, 12, None))
            clips.append((f"{eid}_{prefix}walk", True, walk))
            for e in range(3):
                name = f"{eid}_{prefix}eat_{e}"
                img = fn(0, eat=e, armored=arm)
                add(name, img)
                add(f"{name}_flash", flash(img))
            eat = [(f"{eid}_{prefix}eat_{pose}", EAT_TICKS[i], "eat_hit" if i == EAT_HIT_FRAME else None)
                   for i, pose in enumerate(EAT_POSES)]
            clips.append((f"{eid}_{prefix}eat", True, eat))
    for oid, fn in OTHER.items():
        add(f"{oid}_0", fn(0), anchor=(0.5, 0.5))
        clips.append((oid, True, [(f"{oid}_0", 60, None)]))
    return out, clips


def main():
    imgs, clips = frames()
    cols = 16
    rows = math.ceil(len(imgs) / cols)
    sheet = Image.new("RGBA", (cols * CELL, rows * CELL), (0, 0, 0, 0))
    lines = ["// 由 scripts/gen_skins.py 生成，勿手改（SPEC 12.3）", "Atlas(", '    image: "skins/default/skin.png",', "    frames: ["]
    for i, (name, img, (ax, ay)) in enumerate(imgs):
        x, y = (i % cols) * CELL, (i // cols) * CELL
        sheet.paste(img, (x, y))
        lines.append(f'        (name: "{name}", x: {x}, y: {y}, w: {CELL}, h: {CELL}, anchor: ({ax}, {ay})),')
    lines += ["    ],", ")"]
    anim_dir = os.path.join(OUT, "anim")
    os.makedirs(anim_dir, exist_ok=True)
    for old in os.listdir(anim_dir):
        if old.endswith(".anim.ron"):
            os.remove(os.path.join(anim_dir, old))
    sheet.save(os.path.join(OUT, "skin.png"), optimize=True)
    with open(os.path.join(OUT, "skin.atlas.ron"), "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")
    for name, looping, fr in clips:
        body = [f'// 由 scripts/gen_skins.py 生成', "AnimationClip(", f'    name: "{name}",', f"    looping: {'true' if looping else 'false'},", "    frames: ["]
        for frame, ticks, event in fr:
            ev = f', event: "{event}"' if event else ""
            body.append(f'        (frame: "{frame}", ticks: {ticks}{ev}),')
        body += ["    ],", ")"]
        with open(os.path.join(OUT, "anim", f"{name}.anim.ron"), "w", encoding="utf-8") as f:
            f.write("\n".join(body) + "\n")
    # 清单格式与 tools/artgen 相同：图集及其片段的列表（ADR-014）
    manifest = ["// 由 scripts/gen_skins.py 生成，勿手改（SPEC 12.3）", "Skin(", "    atlases: [", "        (",
                '            atlas: "skins/default/skin.atlas.ron",', "            clips: ["]
    manifest += [f'                "{name}",' for name, _, _ in clips]
    manifest += ["            ],", "        ),", "    ],", ")"]
    with open(os.path.join(OUT, "skin.ron"), "w", encoding="utf-8") as f:
        f.write("\n".join(manifest) + "\n")
    print(f"{len(imgs)} 帧，{len(clips)} 个片段，图集 {sheet.size[0]}×{sheet.size[1]}")


if __name__ == "__main__":
    main()
