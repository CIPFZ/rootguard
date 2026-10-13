#!/usr/bin/env bash
# 依赖规则检查（SPEC.md 3.1 R1 ～ R3 + 0.3 白名单；细化见 docs/decisions/ADR-001）。
#
# 结构沿用 Trellia 引擎的 scripts/check_deps.sh：用 `cargo metadata` 解析每个 workspace crate 的
# 直接依赖（normal + build，不含 dev），与下方硬编码的允许表比对。另外：
#
#   - R1：rootguard-core 的完整（传递）依赖树中不得出现 trellia-gfx / trellia-audio /
#         trellia-scene / egui / eframe；
#   - R2：rootguard-core 源码中不得出现 Sprite / DrawList / AudioQueue / egui 等呈现类型；
#   - R3：trellia 系列依赖必须来自 git 仓库且以 rev 锁定，禁止 path 依赖与 [patch]。
#
# 任一违规即打印 crate 名与违规项并以非零状态退出。
#
# 用法：./scripts/check_deps.sh   （任意目录执行均可）
set -euo pipefail

cd "$(dirname "$0")/.."

command -v python3 >/dev/null || { echo "check_deps: 需要 python3" >&2; exit 2; }

metadata="$(mktemp)"
trap 'rm -f "$metadata"' EXIT
cargo metadata --format-version 1 >"$metadata"

python3 - "$metadata" <<'PY'
import json
import pathlib
import re
import sys

ENGINE_GIT = "https://github.com/CIPFZ/trellia"
# 通用第三方依赖（引擎白名单中与平台无关的部分）
COMMON = {"serde", "ron", "log", "thiserror"}
TRELLIA = {"trellia", "trellia-core", "trellia-assets", "trellia-gfx", "trellia-audio",
           "trellia-scene", "trellia-devtools", "trellia-egui", "trellia-headless"}

# crate -> (允许的直接依赖, 规则说明)
ALLOWED = {
    # R1：core 只允许 trellia-core + serde / ron / log / thiserror
    "rootguard-core": ({"trellia-core"} | COMMON, "R1"),
    # 3.1：content 只允许 core + trellia-assets（+ 通用库，ADR-001）
    "rootguard-content": ({"rootguard-core", "trellia-assets"} | COMMON, "3.1/ADR-001"),
    # 3.1：app 允许全部 trellia crate、core、content、directories 与后端常用库
    "rootguard-app": ({"rootguard-core", "rootguard-content", "directories", "anyhow", "env_logger"}
                      | TRELLIA | COMMON, "3.1"),
}
# R1：core 传递依赖中禁止出现
CORE_FORBIDDEN_TRANSITIVE = {"trellia-gfx", "trellia-audio", "trellia-scene", "egui", "eframe"}
# R2：core 源码中禁止出现的呈现类型标识符
CORE_FORBIDDEN_TOKENS = ["Sprite", "DrawList", "DrawCmd", "AudioQueue", "AudioRequest", "egui"]

with open(sys.argv[1], encoding="utf-8") as f:
    meta = json.load(f)

members = set(meta["workspace_members"])
packages = {p["id"]: p for p in meta["packages"]}
violations = []
checked = 0

for pid in sorted(members):
    pkg = packages[pid]
    name = pkg["name"]
    if name not in ALLOWED:
        violations.append(f"{name}: 未登记的 workspace crate（请在 check_deps.sh 中登记其规则）")
        continue
    allowed, rule = ALLOWED[name]
    checked += 1
    for dep in pkg["dependencies"]:
        if dep["kind"] == "dev":
            continue
        if dep["name"] not in allowed:
            violations.append(f"{name} -> {dep['name']}  违反 {rule}")
        # R3：trellia 依赖必须来自引擎 git 仓库并以 rev 锁定
        if dep["name"] in TRELLIA:
            source = dep.get("source") or ""
            if dep.get("path") or not source.startswith(f"git+{ENGINE_GIT}") or "rev=" not in source:
                violations.append(f"{name} -> {dep['name']}  违反 R3（必须 git + rev，实际 {source or 'path'}）")

# R1：rootguard-core 的传递依赖
resolve = meta.get("resolve") or {}
nodes = {n["id"]: n for n in resolve.get("nodes", [])}
core_ids = [pid for pid in members if packages[pid]["name"] == "rootguard-core"]
for core_id in core_ids:
    seen, stack = set(), [core_id]
    while stack:
        cur = stack.pop()
        if cur in seen:
            continue
        seen.add(cur)
        for dep in nodes.get(cur, {}).get("deps", []):
            kinds = {k.get("kind") for k in dep.get("dep_kinds", [])}
            if kinds and kinds <= {"dev"}:
                continue
            stack.append(dep["pkg"])
    for pid in sorted(seen):
        dep_name = packages[pid]["name"] if pid in packages else pid
        if dep_name in CORE_FORBIDDEN_TRANSITIVE:
            violations.append(f"rootguard-core =>(传递) {dep_name}  违反 R1")

# R2：rootguard-core 源码中的呈现类型
core_src = pathlib.Path("crates/rootguard-core/src")
pattern = re.compile(r"\b(" + "|".join(CORE_FORBIDDEN_TOKENS) + r")\b")
for path in sorted(core_src.rglob("*.rs")):
    for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        m = pattern.search(line)
        if m:
            violations.append(f"{path}:{lineno}: 出现呈现类型 `{m.group(1)}`  违反 R2")

# R3：禁止 [patch]
root_manifest = pathlib.Path("Cargo.toml").read_text(encoding="utf-8")
if re.search(r"^\s*\[patch", root_manifest, re.MULTILINE):
    violations.append("Cargo.toml: 出现 [patch] 段  违反 R3")

if violations:
    print("check_deps: 发现依赖规则违规：", file=sys.stderr)
    for v in violations:
        print(f"  {v}", file=sys.stderr)
    sys.exit(1)
print(f"check_deps: {checked} 个 crate 全部符合依赖规则 R1 ～ R3")
PY
