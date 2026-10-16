#!/usr/bin/env bash
# 美术阶段实机截图（ART-SPEC 11 A2 / A3）：Xvfb 中以花园皮肤 --autoplay 回放脚本解，按秒截若干张。
#
#   ./scripts/shot_garden.sh <脚本> <输出前缀> <秒>...
#   ./scripts/shot_garden.sh tests/scripts/1-9.ron /tmp/shots/n19 90 200 300
#
# 依赖同 scripts/screenshot.sh（Xvfb、ImageMagick import、Mesa）。存档目录隔离（ROOTGUARD_SAVE_DIR）。
# 环境变量 SHOT_LANG（Zh / En，默认 Zh）。截图时刻按真实时间，画面不逐帧固定。
set -euo pipefail

cd "$(dirname "$0")/.."
script="$1"; prefix="$2"; shift 2
lang="${SHOT_LANG:-Zh}"
cargo build -q --release --locked -p rootguard-app --no-default-features
mkdir -p "$(dirname "$prefix")"
work=$(mktemp -d)
n=90
while [ -e "/tmp/.X11-unix/X$n" ] || [ -e "/tmp/.X$n-lock" ]; do n=$((n + 1)); done
display=":$n"
Xvfb "$display" -screen 0 1280x720x24 >/dev/null 2>&1 &
xvfb=$!
trap 'kill "$xvfb" "${game:-}" 2>/dev/null || true; rm -rf "$work"' EXIT
sleep 1
cat > "$work/progress.ron" <<RON
(version: 1, cleared: [], unlocked: [], settings: (sfx: 0.0, music: 0.0, lang: $lang, skin: Skinned, skin_name: Garden))
RON
DISPLAY="$display" ROOTGUARD_SAVE_DIR="$work" target/release/rootguard --autoplay "$script" >"$work/log" 2>&1 &
game=$!
start=$(date +%s.%N)
for t in "$@"; do
    now=$(date +%s.%N)
    wait_s=$(echo "$start + $t - $now" | bc)
    if [ "$(echo "$wait_s > 0" | bc)" = 1 ]; then sleep "$wait_s"; fi
    if ! kill -0 "$game" 2>/dev/null; then echo "游戏提前退出：" >&2; cat "$work/log" >&2; exit 1; fi
    DISPLAY="$display" import -window root "${prefix}-${t}.png"
    echo "${prefix}-${t}.png"
done
