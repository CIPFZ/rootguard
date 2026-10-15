#!/usr/bin/env python3
"""Rootguard 音频程序化合成（SPEC 1.2.4 / 13）。

只用 Python 标准库生成 22050 Hz 单声道 16 位 WAV，再用 oggenc（vorbis-tools）编码为
assets/audio/*.ogg。全部波形由本脚本从零合成，旋律由固定种子在五声音阶上随机游走生成，
不采样、不引用任何外部音频。重新生成：

    python3 scripts/gen_audio.py              # 全部
    python3 scripts/gen_audio.py sfx_bite     # 只生成指定文件（不带扩展名），避免无关文件因编码器差异变动

输出逐字节可能随 oggenc 版本略有差异，但听感一致；生成后在 assets/SOURCES.md 登记。
"""

import math
import os
import random
import shutil
import struct
import subprocess
import sys
import tempfile
import wave

RATE = 22050
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
OUT = os.path.join(ROOT, "assets", "audio")


def silence(seconds):
    return [0.0] * int(RATE * seconds)


def env(i, n, attack=0.005, release=None):
    """线性起音 + 指数衰减包络。"""
    t = i / RATE
    a = min(1.0, t / attack) if attack > 0 else 1.0
    total = n / RATE
    rel = release if release is not None else total
    return a * math.exp(-4.0 * t / max(rel, 1e-6))


def tone(freq_from, freq_to, seconds, shape="sine", vol=0.6, decay=None, attack=0.004):
    n = int(RATE * seconds)
    out = []
    phase = 0.0
    for i in range(n):
        f = freq_from + (freq_to - freq_from) * i / max(n - 1, 1)
        phase += 2 * math.pi * f / RATE
        if shape == "sine":
            v = math.sin(phase)
        elif shape == "square":
            v = 0.6 if math.sin(phase) >= 0 else -0.6
        elif shape == "tri":
            v = 2 / math.pi * math.asin(math.sin(phase))
        else:
            raise ValueError(shape)
        out.append(v * vol * env(i, n, attack, decay))
    return out


def noise(seconds, vol=0.5, smooth=0.0, seed=1, decay=None, attack=0.003):
    rng = random.Random(seed)
    n = int(RATE * seconds)
    out = []
    last = 0.0
    for i in range(n):
        x = rng.uniform(-1, 1)
        last = last * smooth + x * (1 - smooth)
        out.append(last * vol * env(i, n, attack, decay))
    return out


def mix(*tracks):
    n = max(len(t) for t in tracks)
    return [sum(t[i] for t in tracks if i < len(t)) for i in range(n)]


def concat(*parts):
    out = []
    for p in parts:
        out.extend(p)
    return out


def write_wav(path, samples, peak=0.85):
    m = max(1e-9, max(abs(s) for s in samples))
    scale = peak / m if m > peak else 1.0
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(b"".join(struct.pack("<h", int(max(-1, min(1, s * scale)) * 32767)) for s in samples))


# ---------------------------------------------------------------- 音效

def sfx():
    return {
        "sfx_shoot": tone(620, 300, 0.07, "tri", 0.7, decay=0.05),
        "sfx_hit": mix(noise(0.06, 0.5, 0.6, seed=2, decay=0.03), tone(170, 110, 0.06, "sine", 0.6, decay=0.04)),
        "sfx_hit_armor": mix(tone(1180, 1150, 0.09, "square", 0.25, decay=0.05), tone(1660, 1630, 0.09, "square", 0.2, decay=0.04)),
        "sfx_plant": mix(tone(400, 520, 0.12, "tri", 0.6, decay=0.08), noise(0.05, 0.15, 0.8, seed=3, decay=0.03)),
        "sfx_dig": noise(0.22, 0.6, 0.92, seed=4, decay=0.12, attack=0.02),
        "sfx_dew": mix(tone(1320, 1320, 0.28, "sine", 0.45, decay=0.18), tone(1980, 1980, 0.2, "sine", 0.25, decay=0.1)),
        "sfx_husk_down": tone(380, 110, 0.32, "tri", 0.6, decay=0.25),
        "sfx_flag_wave": concat(tone(220, 222, 0.32, "square", 0.35, decay=0.6, attack=0.03),
                                tone(330, 332, 0.42, "square", 0.35, decay=0.6, attack=0.03)),
        "sfx_hose": [s * math.sin(math.pi * i / int(RATE * 0.6)) for i, s in enumerate(noise(0.6, 0.7, 0.85, seed=5, decay=10.0))],
        "sfx_win": concat(*[tone(f, f, 0.13, "tri", 0.55, decay=0.12) for f in (523, 659, 784, 1046)]),
        "sfx_lose": concat(*[tone(f, f * 0.98, 0.17, "tri", 0.55, decay=0.15) for f in (392, 330, 262, 196)]),
        # 啃食（皮肤 eat 片段的 eat_hit 帧，ADR-010）：两下短促的低沉碎响，音量偏低，因为会频繁触发
        "sfx_bite": concat(mix(noise(0.045, 0.32, 0.7, seed=6, decay=0.02), tone(150, 95, 0.045, "sine", 0.3, decay=0.025)),
                           silence(0.015),
                           mix(noise(0.035, 0.22, 0.75, seed=7, decay=0.015), tone(130, 90, 0.035, "sine", 0.2, decay=0.02))),
        # ---- 美术阶段 A2 新增（ART-SPEC 12 清单；情绪依据见各设计卡与 LORE §5）
        # 露杯 / 日灯笼产出：极轻的一滴（4.2）
        "sfx_drip": mix(tone(1500, 900, 0.09, "sine", 0.18, decay=0.05), tone(2400, 2200, 0.04, "sine", 0.06, decay=0.02)),
        # 树皮墙倒下：木头裂开（4.3）
        "sfx_wood_crack": concat(noise(0.05, 0.55, 0.3, seed=21, decay=0.02),
                                 mix(noise(0.2, 0.35, 0.85, seed=22, decay=0.1), tone(120, 70, 0.22, "tri", 0.45, decay=0.15))),
        # 霜籽荚发射：发射声 + 高频“叮”（4.4）
        "sfx_shoot_frost": mix(tone(620, 320, 0.07, "tri", 0.55, decay=0.05), tone(2640, 2600, 0.16, "sine", 0.22, decay=0.1)),
        # 冰石命中：冰晶碎响（6 章）
        "sfx_hit_frost": mix(noise(0.05, 0.25, 0.2, seed=23, decay=0.02), tone(1980, 1900, 0.1, "sine", 0.2, decay=0.06),
                             tone(2970, 2900, 0.07, "sine", 0.12, decay=0.04)),
        # 荆棘垫扎：轻（4.6）
        "sfx_prick": mix(noise(0.03, 0.2, 0.1, seed=24, decay=0.012), tone(1800, 1400, 0.03, "tri", 0.12, decay=0.015)),
        # 块茎雷：闷响 + 呼气（4.7“终于呼出来”）
        "sfx_tuber_burst": concat(mix(tone(90, 45, 0.18, "sine", 0.7, decay=0.12), noise(0.12, 0.45, 0.9, seed=25, decay=0.06)),
                                  noise(0.35, 0.25, 0.7, seed=26, decay=0.3, attack=0.08)),
        # 炽球：低频轰 + 嘶（4.8）
        "sfx_ember_burst": mix(tone(70, 35, 0.4, "sine", 0.75, decay=0.3), noise(0.45, 0.35, 0.95, seed=27, decay=0.25),
                               noise(0.5, 0.15, 0.2, seed=28, decay=0.4, attack=0.1)),
        # 葫芦壳破碎：空壳裂开（4.9）
        "sfx_shell_crack": concat(noise(0.04, 0.5, 0.3, seed=29, decay=0.015),
                                  mix(tone(260, 180, 0.2, "tri", 0.4, decay=0.12), noise(0.15, 0.2, 0.8, seed=30, decay=0.08))),
        # 咬夹草吞：一声合拢（4.10）
        "sfx_snap": mix(noise(0.04, 0.55, 0.5, seed=31, decay=0.012), tone(320, 160, 0.06, "square", 0.25, decay=0.03)),
        # 咬夹草咽下：低沉下滑（4.10）
        "sfx_gulp": concat(tone(240, 120, 0.12, "sine", 0.45, decay=0.1), tone(180, 90, 0.1, "sine", 0.3, decay=0.08)),
        # 风铃花：一口气吹出的呼啸（4.11）
        "sfx_gust": [v * math.sin(math.pi * i / int(RATE * 0.55)) for i, v in enumerate(noise(0.55, 0.55, 0.93, seed=32, decay=10.0))],
        # 日灯笼第 4 次：温暖的铃（4.12）
        "sfx_lantern": mix(tone(660, 660, 0.7, "sine", 0.3, decay=0.5), tone(990, 990, 0.6, "sine", 0.15, decay=0.4),
                           tone(1320, 1320, 0.45, "sine", 0.08, decay=0.3)),
        # 守卫被吃光：轻轻的叹息下滑（4.13）
        "sfx_wilt": tone(520, 260, 0.25, "tri", 0.22, decay=0.2, attack=0.02),
        # 枝冠折断：干脆的枝条断裂（5.2）
        "sfx_twig_snap": concat(noise(0.025, 0.6, 0.0, seed=33, decay=0.008), silence(0.02), noise(0.03, 0.4, 0.2, seed=34, decay=0.01)),
        # 陶盆碎：陶片碰撞（5.3）
        "sfx_pot_break": mix(noise(0.18, 0.4, 0.4, seed=35, decay=0.08), tone(1400, 1300, 0.12, "square", 0.12, decay=0.05),
                             tone(1900, 1850, 0.09, "square", 0.08, decay=0.04)),
        # 跳栏：芦苇弹响（5.5）
        "sfx_vault": tone(180, 520, 0.16, "tri", 0.4, decay=0.12),
        # 背篓倒下：种子撒一地（5.7）
        "sfx_seeds_spill": concat(*[mix(silence(0.012 + 0.006 * (k % 3)), noise(0.012, 0.25 - 0.015 * k, 0.1, seed=40 + k, decay=0.005))
                                    for k in range(12)]),
        # 鼓腹枯壳每步：沉闷的咚（5.8）
        "sfx_thud": mix(tone(80, 55, 0.14, "sine", 0.4, decay=0.09), noise(0.05, 0.12, 0.9, seed=36, decay=0.03)),
        # 葫芦身炸开：空心裂响 + 回音（5.8）
        "sfx_gourd_break": concat(noise(0.05, 0.6, 0.3, seed=37, decay=0.02),
                                  mix(tone(150, 90, 0.35, "tri", 0.5, decay=0.25), tone(300, 180, 0.3, "sine", 0.2, decay=0.2))),
        # 小芽怪合拢：比其他 Husk 轻（5.9）
        "sfx_sprout_down": tone(700, 420, 0.16, "sine", 0.25, decay=0.12),
    }


# ---------------------------------------------------------------- 音乐

PENTA = [0, 2, 4, 7, 9]


def note_freq(base, degree):
    octave, step = divmod(degree, len(PENTA))
    return base * 2 ** ((PENTA[step] + 12 * octave) / 12)


def compose(seed, bpm, bars, base, lead_shape, bass_shape, drums, density):
    """在五声音阶上随机游走的旋律 + 根音低音（+ 可选打击）；总长恰为整小节，可无缝循环。"""
    rng = random.Random(seed)
    beat = 60.0 / bpm
    step = beat / 2  # 八分音符
    steps = bars * 8
    total = int(RATE * step * steps)
    track = [0.0] * total
    degree = 5
    for k in range(steps):
        start = int(RATE * step * k)
        if rng.random() < density:
            degree = max(0, min(10, degree + rng.choice([-2, -1, -1, 1, 1, 2])))
            length = step * rng.choice([1, 1, 2])
            n = tone(note_freq(base, degree), note_freq(base, degree), length, lead_shape, 0.28, decay=length * 0.9, attack=0.01)
            for i, v in enumerate(n):
                if start + i < total:
                    track[start + i] += v
        if k % 4 == 0:
            root = [0, 0, 3, 4][(k // 8) % 4]
            b = tone(note_freq(base / 4, root), note_freq(base / 4, root), step * 3.5, bass_shape, 0.3, decay=step * 3, attack=0.01)
            for i, v in enumerate(b):
                if start + i < total:
                    track[start + i] += v
        if drums and k % 2 == 0:
            d = noise(0.05, 0.25 if k % 8 == 0 else 0.12, 0.3, seed=seed + k, decay=0.02)
            for i, v in enumerate(d):
                if start + i < total:
                    track[start + i] += v
    # 首尾 5 ms 淡入淡出，避免循环点爆音
    fade = int(RATE * 0.005)
    for i in range(fade):
        track[i] *= i / fade
        track[-1 - i] *= i / fade
    return track


def music():
    return {
        "bgm_title": compose(seed=11, bpm=88, bars=8, base=330.0, lead_shape="tri", bass_shape="sine", drums=False, density=0.55),
        "bgm_battle": compose(seed=23, bpm=124, bars=12, base=294.0, lead_shape="square", bass_shape="tri", drums=True, density=0.7),
        "bgm_outcome": compose(seed=37, bpm=96, bars=4, base=392.0, lead_shape="sine", bass_shape="sine", drums=False, density=0.5),
    }


def main():
    oggenc = shutil.which("oggenc")
    if not oggenc:
        sys.exit("gen_audio: 需要 oggenc（vorbis-tools）")
    os.makedirs(OUT, exist_ok=True)
    clips = {**sfx(), **music()}
    only = sys.argv[1:]
    unknown = [n for n in only if n not in clips]
    if unknown:
        sys.exit(f"gen_audio: 未知名称 {unknown}，可选 {sorted(clips)}")
    with tempfile.TemporaryDirectory() as tmp:
        for name, samples in clips.items():
            if only and name not in only:
                continue
            wav = os.path.join(tmp, name + ".wav")
            write_wav(wav, samples)
            ogg = os.path.join(OUT, name + ".ogg")
            subprocess.run([oggenc, "-Q", "-q", "3", "-o", ogg, wav], check=True)
            print(f"{ogg}  {len(samples) / RATE:.2f}s")


if __name__ == "__main__":
    main()
