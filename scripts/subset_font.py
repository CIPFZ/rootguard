#!/usr/bin/env python3
"""从 Noto Sans CJK SC（SIL OFL 1.1）生成游戏所需的 CJK 字体子集（SPEC 15 E-1，SPEC 11.6）。

字符集 = assets/data/i18n/*.ron 中出现的全部字符 + 可打印 ASCII + 常用全角标点。
子集更名为 “Rootguard CJK Subset”（OFL 对修改版本的命名要求），输出：

    assets/fonts/RootguardCJK-Regular.otf
    assets/fonts/LICENSE-OFL.txt   （原字体许可证全文）

需要 fonttools（仅开发期工具，不是游戏依赖）。`tmp/venv/` 不入库（`.gitignore`），不存在时先重建：

    python3 -m venv tmp/venv && tmp/venv/bin/pip install fonttools
    tmp/venv/bin/python scripts/subset_font.py [NotoSansCJK-Regular.ttc] [OFL LICENSE]

文本表增加新汉字后需重新运行，并在 assets/SOURCES.md 更新记录。
"""

import glob
import os
import shutil
import sys

from fontTools import subset
from fontTools.ttLib import TTCollection

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
DEFAULT_TTC = "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc"
DEFAULT_LICENSE = "/usr/share/licenses/google-noto-cjk-fonts-common/LICENSE"
SC_INDEX_NAME = "Noto Sans CJK SC"
FAMILY = "Rootguard CJK Subset"
EXTRA = "，。！？：；、（）《》“”‘’…—·×～"


def charset():
    chars = set(chr(c) for c in range(0x20, 0x7F)) | set(EXTRA)
    for path in glob.glob(os.path.join(ROOT, "assets", "data", "i18n", "*.ron")):
        with open(path, encoding="utf-8") as f:
            chars |= set(f.read())
    return "".join(sorted(c for c in chars if c.isprintable()))


def main():
    ttc = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_TTC
    license_path = sys.argv[2] if len(sys.argv) > 2 else DEFAULT_LICENSE
    collection = TTCollection(ttc)
    font = next(f for f in collection.fonts if f["name"].getDebugName(4) == SC_INDEX_NAME)
    text = charset()
    options = subset.Options()
    options.layout_features = ["*"]
    options.name_IDs = ["*"]
    options.notdef_outline = True
    subsetter = subset.Subsetter(options)
    subsetter.populate(text=text)
    subsetter.subset(font)
    names = font["name"]
    for rec in list(names.names):
        if rec.nameID in (1, 16):
            rec.string = FAMILY
        elif rec.nameID in (3, 4):
            rec.string = f"{FAMILY} Regular"
        elif rec.nameID == 6:
            rec.string = "RootguardCJKSubset-Regular"
    out_dir = os.path.join(ROOT, "assets", "fonts")
    os.makedirs(out_dir, exist_ok=True)
    out = os.path.join(out_dir, "RootguardCJK-Regular.otf")
    font.save(out)
    shutil.copyfile(license_path, os.path.join(out_dir, "LICENSE-OFL.txt"))
    print(f"{out}: {len(text)} 字符, {os.path.getsize(out)} 字节")


if __name__ == "__main__":
    main()
