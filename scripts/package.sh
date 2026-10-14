#!/usr/bin/env bash
# 发布打包（SPEC 17 G5“三平台打包脚本”）：Linux / macOS / Windows（Git Bash）通用。
#
#   ./scripts/package.sh                  # 为本机目标打包
#   ./scripts/package.sh --target x86_64-pc-windows-msvc
#
# 产物：dist/rootguard-<版本>-<目标>/（可执行文件 + assets/ + 许可证 + 说明），以及同名 .tar.gz（Windows 为 .zip）。
# 发布构建不带 devtools（AGENT 4.4：调试窗口在发布构建中不存在）。游戏在可执行文件同级查找 assets/。
set -euo pipefail

cd "$(dirname "$0")/.."

target=""
while [ $# -gt 0 ]; do
    case "$1" in
        --target) target="$2"; shift 2 ;;
        -h|--help) sed -n '2,8p' "$0"; exit 0 ;;
        *) echo "未知参数: $1" >&2; exit 2 ;;
    esac
done

version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version *= *"\(.*\)"/\1/p' Cargo.toml)
host=$(rustc -vV | sed -n 's/^host: //p')
target=${target:-$host}
case "$target" in
    *windows*) exe="rootguard.exe" ;;
    *) exe="rootguard" ;;
esac

echo "打包 rootguard ${version}（${target}）"
cargo build --release --locked -p rootguard-app --no-default-features --target "$target"

name="rootguard-${version}-${target}"
out="dist/${name}"
rm -rf "$out"
mkdir -p "$out"
cp "target/${target}/release/${exe}" "$out/"
cp -R assets "$out/assets"
cp LICENSE-MIT LICENSE-APACHE "$out/"
cat > "$out/README.txt" <<EOF
Rootguard ${version} (${target})

运行 / Run: ./${exe}
assets/ 必须与可执行文件放在同一目录。 / Keep assets/ next to the executable.
存档目录可用环境变量 ROOTGUARD_SAVE_DIR 覆盖。 / Override the save directory with ROOTGUARD_SAVE_DIR.
代码与项目自有素材以 MIT OR Apache-2.0 双许可发布（LICENSE-MIT / LICENSE-APACHE）。
素材来源与许可证见 assets/SOURCES.md；字体许可证见 assets/fonts/LICENSE-OFL.txt。
Code and project-owned assets: MIT OR Apache-2.0 (LICENSE-MIT / LICENSE-APACHE).
Asset sources and licenses: assets/SOURCES.md; font license: assets/fonts/LICENSE-OFL.txt.
EOF

# 布局自检：可执行文件、数据、皮肤清单与各许可证都在
for f in "$exe" LICENSE-MIT LICENSE-APACHE assets/data/rules.ron assets/skins/default/skin.ron assets/fonts/LICENSE-OFL.txt assets/SOURCES.md; do
    [ -e "$out/$f" ] || { echo "打包缺少 $f" >&2; exit 1; }
done

case "$target" in
    *windows*)
        archive="dist/${name}.zip"
        rm -f "$archive"
        if command -v 7z >/dev/null 2>&1; then
            (cd dist && 7z a -tzip -bso0 "${name}.zip" "$name")
        else
            powershell -NoProfile -Command "Compress-Archive -Path 'dist/${name}' -DestinationPath '${archive}'"
        fi
        ;;
    *)
        archive="dist/${name}.tar.gz"
        tar -C dist -czf "$archive" "$name"
        ;;
esac
echo "完成：${archive}"
