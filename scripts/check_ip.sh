#!/usr/bin/env bash
# 知识产权禁用词检查（SPEC.md 1.2，AGENT.md 第 6 章）。
#
# 对 crates/ examples/ assets/data/ docs/ 下的文本文件做大小写不敏感的匹配，禁用词表为
# scripts/ip_denylist.txt：每行一个词或短语，空行与 `#` 开头的行忽略。命中任一词即失败。
#
# 禁用词表由项目负责人维护；实现者不得向其中添加任何第三方作品的角色或作品名。
#
# 用法：./scripts/check_ip.sh   （任意目录执行均可）
set -euo pipefail

cd "$(dirname "$0")/.."

denylist="scripts/ip_denylist.txt"
[ -f "$denylist" ] || { echo "check_ip: 缺少 $denylist" >&2; exit 2; }

command -v python3 >/dev/null || { echo "check_ip: 需要 python3" >&2; exit 2; }

python3 - "$denylist" <<'PY'
import pathlib
import sys

ROOTS = ["crates", "examples", "assets/data", "docs"]
# 二进制或生成物不检查
SKIP_SUFFIXES = {".png", ".ogg", ".wav", ".ttf", ".otf", ".snap"}

words = []
for raw in pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines():
    w = raw.strip()
    if w and not w.startswith("#"):
        words.append(w.casefold())

if not words:
    print("check_ip: 禁用词表为空（请项目负责人填写 scripts/ip_denylist.txt），跳过匹配")
    sys.exit(0)

hits = []
scanned = 0
for root in ROOTS:
    base = pathlib.Path(root)
    if not base.is_dir():
        continue
    for path in sorted(base.rglob("*")):
        if not path.is_file() or path.suffix.lower() in SKIP_SUFFIXES:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        scanned += 1
        for lineno, line in enumerate(text.splitlines(), 1):
            folded = line.casefold()
            for w in words:
                if w in folded:
                    hits.append(f"{path}:{lineno}: 命中禁用词")

if hits:
    print("check_ip: 发现禁用词（词表内容不回显）：", file=sys.stderr)
    for h in hits:
        print(f"  {h}", file=sys.stderr)
    sys.exit(1)
print(f"check_ip: {scanned} 个文件未命中 {len(words)} 个禁用词")
PY
