#!/usr/bin/env bash
# README 截图（SPEC 20.3）：在 Xvfb 虚拟显示中以 --autoplay 回放脚本解，对局进行到一定时间后截图。
#
#   ./scripts/screenshot.sh                       # 占位 / 皮肤各一张，输出到 docs/screenshots/
#   ./scripts/screenshot.sh --level 1-5 --secs 150 --out /tmp/shots
#
# 依赖：Xvfb、ImageMagick（import）、OpenGL 软件渲染（Mesa）。渲染模式经隔离存档目录中的
# progress.ron 设置（ROOTGUARD_SAVE_DIR，SPEC 14），不读写用户存档。截图时刻按真实时间，画面不逐帧固定。
set -euo pipefail

cd "$(dirname "$0")/.."

level="1-5"
secs=150
out="docs/screenshots"
display=""
while [ $# -gt 0 ]; do
    case "$1" in
        --level) level="$2"; shift 2 ;;
        --secs) secs="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        --display) display="$2"; shift 2 ;;
        -h|--help) sed -n '2,8p' "$0"; exit 0 ;;
        *) echo "未知参数: $1" >&2; exit 2 ;;
    esac
done

cargo build --release --locked -p rootguard-app --no-default-features
mkdir -p "$out"
work=$(mktemp -d)
if [ -z "$display" ]; then
    # 取第一个空闲的显示号（已有的 X 服务器分辨率未必是 1280 × 720）
    n=90
    while [ -e "/tmp/.X11-unix/X$n" ] || [ -e "/tmp/.X$n-lock" ]; do n=$((n + 1)); done
    display=":$n"
fi
Xvfb "$display" -screen 0 1280x720x24 >/dev/null 2>&1 &
xvfb=$!
trap 'kill "$xvfb" 2>/dev/null || true; rm -rf "$work"' EXIT
sleep 1
kill -0 "$xvfb" 2>/dev/null || { echo "Xvfb 无法在 $display 启动" >&2; exit 1; }

for mode in Placeholder Skinned; do
    save="$work/$mode"
    mkdir -p "$save"
    cat > "$save/progress.ron" <<EOF
(version: 1, cleared: [], unlocked: [], settings: (sfx: 0.0, music: 0.0, lang: Zh, skin: $mode))
EOF
    DISPLAY="$display" ROOTGUARD_SAVE_DIR="$save" \
        target/release/rootguard --autoplay "tests/scripts/${level}.ron" >"$work/$mode.log" 2>&1 &
    game=$!
    sleep "$secs"
    if ! kill -0 "$game" 2>/dev/null; then
        echo "游戏提前退出（$mode），日志：" >&2
        cat "$work/$mode.log" >&2
        exit 1
    fi
    file="$out/battle-$(echo "$mode" | tr '[:upper:]' '[:lower:]').png"
    DISPLAY="$display" import -window root "$file"
    kill "$game" 2>/dev/null || true
    wait "$game" 2>/dev/null || true
    echo "截图：$file"
done
