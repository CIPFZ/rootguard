# ADR-001：引擎依赖的引入方式与 check_deps.sh 允许表

- 编号：ADR-001
- 标题：引擎依赖的引入方式与 check_deps.sh 允许表
- 日期：2026-10-13
- 状态：已接受

## 背景

SPEC.md 3.1 R3 写作 `[workspace.dependencies] trellia = { git, rev }`，同时规定
`rootguard-core` 只依赖 “`trellia::core`（即 `trellia-core`）”、`rootguard-content` 依赖 `trellia-assets`。
但引擎门面 crate `trellia` 的非可选依赖包含 `trellia-gfx` / `trellia-audio` / `trellia-scene` /
`trellia-headless`（后两者又依赖 `egui`）。若 `rootguard-core` 经门面依赖引擎，其传递依赖树必然包含
R1 禁止的 crate。

此外 SPEC.md 3.1 只列出各 crate 的“主要”依赖，没有说明 `rootguard-content` / `rootguard-app`
能否使用 `log`、`serde` 等通用库与 `anyhow` / `env_logger`。

## 决策

1. `[workspace.dependencies]` 中同时声明 `trellia`、`trellia-core`、`trellia-assets` 三项，
   全部指向 `https://github.com/CIPFZ/trellia` 的**同一 rev**（升级时三处同时修改）。
   `rootguard-core` 直接依赖 `trellia-core`，`rootguard-content` 直接依赖 `trellia-assets`，
   `rootguard-app` 依赖门面 `trellia`（`default-features = false`，按需开启 `egui` / `devtools`）。
   Cargo 对同一 git 源同一 rev 的包只解析一次，三者共享同一份引擎代码。
2. `scripts/check_deps.sh` 的直接依赖允许表（“通用” = `serde` / `ron` / `log` / `thiserror`）：

   | crate | 允许的直接依赖 |
   |---|---|
   | rootguard-core | `trellia-core` + 通用（R1 原文） |
   | rootguard-content | `rootguard-core` + `trellia-assets` + 通用 |
   | rootguard-app | 全部 trellia crate + core + content + `directories` + `anyhow` + `env_logger` + 通用 |

3. 在引擎脚本结构之上增加三项检查：R1 对 `rootguard-core` 做**传递**依赖检查；R2 对
   `rootguard-core/src` 做呈现类型标识符匹配；R3 要求 trellia 依赖来源为引擎 git 仓库且带 `rev`，
   并禁止根 `Cargo.toml` 出现 `[patch]`。只检查 normal / build 依赖，不检查 dev 依赖。

## 后果

- R1 在依赖树层面成立，而不仅是“直接依赖表里没有”。
- 引擎升级需同时改三处 `rev`；`check_deps.sh` 的 R3 检查会拒绝 path 依赖。
- `check_deps.sh` 在 G0 之后只读；新增依赖边需新 ADR 并向人类说明（AGENT.md 3.5）。

## 替代方案

1. `rootguard-core` 依赖门面 `trellia` 并关闭默认 feature：门面的 gfx / audio / scene / headless
   为非可选依赖，仍违反 R1，否决。
2. 只声明 `trellia` 一项，子 crate 通过 `trellia = { package = ... }` 重命名引入：写法晦涩，
   且同样需要多处 `rev`，否决。
