#!/usr/bin/env bash
# 提交前检查清单（AGENT.md 第 5 章；裁决八 2.3：任何一步失败即不提交）。
#
#   ./scripts/precommit.sh            # 全部检查，成功时最后一行输出 "precommit: OK"
#   ./scripts/precommit.sh && git commit ...
#
# 依次：fmt、clippy（-D warnings）、doc（-D warnings）、依赖规则、禁用词、美术资产一致性、全部测试。
set -euo pipefail

cd "$(dirname "$0")/.."

step() { printf '\nprecommit: %s\n' "$*"; }

step "cargo fmt --check"
cargo fmt --all -- --check
step "cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings
step "cargo doc"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
step "check_deps"
./scripts/check_deps.sh
step "check_ip"
./scripts/check_ip.sh
step "artgen check"
cargo run -q --release -p artgen -- check
step "cargo test"
cargo test --workspace
echo
echo "precommit: OK"
