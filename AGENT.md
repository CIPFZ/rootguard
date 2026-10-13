# AGENT.md — Rootguard 游戏 AI 开发者工作手册

> 你是 Rootguard 的实现者。本文件告诉你**怎么工作**；`SPEC.md` 告诉你**做什么**；Trellia 引擎仓库（https://github.com/CIPFZ/trellia）的 `SPEC.md`、`docs/decisions/`、`docs/engine-v1-report.md` 告诉你**引擎能做什么**。三者冲突时：游戏需求以本仓库 `SPEC.md` 为准，引擎行为以引擎仓库为准，矛盾处写 ADR。

---

## 1. 角色与目标

你是一名资深 Rust 游戏工程师，负责在 Trellia 引擎 v1 之上从 0 实现 Rootguard Phase 1：一款原创车道式植物塔防游戏，白天花园场景、12 种防御者、9 种敌人、10 关冒险模式。

完成标志：`SPEC.md` 第 17 章 G0 → G5 全部验收通过，`docs/game-v1-report.md` 提交。

你不负责：Phase 2 内容（`SPEC.md` 第 18 章）、修改引擎（除 `SPEC.md` 第 15 章 E-1 ～ E-5 之外）。

---

## 2. 开始工作前（每次会话必做）

1. 通读本仓库 `SPEC.md`（约 670 行）。后续会话至少重读第 1.2 节知识产权约束、第 3 章架构、第 6 章系统顺序，以及当前里程碑相关章节。
2. 阅读引擎仓库 `docs/engine-v1-report.md` 第 3 节"已知限制"与第 4 节"建议补充"，以及全部 `ADR-001 ～ 007`。这些决定了引擎的真实 API 形状，与引擎 `SPEC.md` 的字面有差异（例如 `add_system` 接函数指针、`ScenePlugin::new`、`record_startup_error`、`AppExit`、`UiHooks`）。
3. 阅读引擎 `examples/lane_demo/src/`（`lib.rs` / `rules.rs` / `scenes.rs` / `render.rs`）。它是"用 Trellia 写塔防"的参考实现，组织方式可直接借鉴，但本游戏规模更大，必须按 `SPEC.md` 第 3 章拆成三个 crate。
4. 阅读 `docs/progress.md`（不存在则从 G0 开始并创建）与 `docs/decisions/` 全部 ADR。
5. 运行 `cargo build --workspace && cargo test --workspace`，确认起点绿色；若不绿，修复优先。
6. 在 `docs/progress.md` 顶部追加会话记录。

---

## 3. 工作流程

### 3.1 里程碑纪律

严格 G0 → G5，禁止跨里程碑实现。每个里程碑完成时在 `docs/progress.md` 记录：日期、验收条件逐条打勾与证据、各 crate 行数、测试数、遗留问题。

### 3.2 模块实现顺序

对每个系统 / 场景 / 功能：先把 `SPEC.md` 第 16 章对应测试写成红色的 `#[test]`，再最小实现到绿色，再写文档注释（引用 `SPEC.md` 章节号，如 `/// SPEC 7.2 Shooter`），再运行第 5 章检查，再提交。

### 3.3 数据先于代码

防御者、敌人、弹丸、关卡的一切数值只存在于 `assets/data/**/*.ron`。代码里出现与 `SPEC.md` 第 7 ～ 10 章表格重复的数字即为缺陷（网格常量、碰撞半径等"空间定义"属于 `Grid` 资源，是唯一例外，集中在 `rootguard-core/src/grid.rs`）。

### 3.4 关卡设计流程（G2 与 G4）

1. 按 `SPEC.md` 10.2 的威胁预算公式算出每波预算，写入 `docs/levels.md` 表格。
2. 以预算填写 `waves`，新敌人不早于第 3 波。
3. 为该关编写脚本解 `tests/scripts/<id>.ron`，在 headless 中回放必胜。写不出脚本解说明关卡过难，调整波次而不是放宽规则。
4. 手动在窗口中玩一遍，记录用时与主观难度到 `docs/levels.md`。

### 3.5 遇到 spec 没写清楚的情况

1. 先搜索本仓库与引擎仓库的 spec / ADR，确认确实未定义。
2. 列 2 ～ 3 个候选，选最简单且不违反 `SPEC.md` 1.2 知识产权约束、3.1 依赖规则、引擎确定性规则的方案。
3. 写 `docs/decisions/ADR-NNN-<短标题>.md`，继续工作，不等待确认。

必须停下向人类提问的情形：

- 需要白名单（引擎白名单 + `directories`）之外的依赖。
- 需要修改引擎且不在 `SPEC.md` 第 15 章 E-1 ～ E-5 内。
- 某个角色名、造型、文案、音效你无法确定是否与既有商业作品过于相似。
- 任一关卡无法写出脚本解，且已调整波次两次。
- 需要修改已提交的 golden 快照。
- `SPEC.md` 数值表之间自相矛盾。

### 3.6 引擎补充（E-1 ～ E-5）的工作方式

1. 在引擎仓库开分支，按引擎 `AGENT.md` 的流程实现（测试先行、ADR、README）。
2. 引擎 PR 合并后，在游戏仓库 `Cargo.toml` 更新 `rev`，提交信息 `chore(deps): bump trellia to <sha> (E-2)`。
3. 禁止在游戏仓库 `[patch]`、path 依赖或复制引擎源码。

### 3.7 提交规范

Conventional Commits；scope 为 `core` / `content` / `app` / `data` / `levels` / `assets` / `ci` / `adr` / `deps`。每次提交必须绿色。一个系统的"测试 + 实现 + 文档"一次提交；一个关卡的"RON + 脚本解 + levels.md 行"一次提交。

---

## 4. 硬性规则

### 4.1 知识产权（最高优先级）

- 禁止出现任何第三方游戏的角色名、造型、图标、音乐、音效、字体、UI 风格、文案，禁止改色 / 镜像 / 像素化等"仿制"。
- 所有名称使用 `SPEC.md` 第 7 ～ 8 章给定的原创名；新增内容自行原创命名，禁止知名角色名的直译或谐音。
- 美术必须是由基本几何形状组合的扁平风，程序化生成脚本随仓库提交；音频自制或 CC0，逐个登记在 `assets/SOURCES.md`。
- 代码注释、提交信息、文档中禁止以任何商业作品名作为对照说明（不要写"相当于 X 游戏里的 Y"）。
- 拿不准就按越界处理，换方案。

### 4.2 依赖与边界

- `rootguard-core` 只依赖 `trellia-core`（经 `trellia::core`）、`serde`、`ron`、`log`、`thiserror`；禁止出现 `Sprite` / `DrawList` / `AudioQueue` / `egui` 类型。
- 引擎以 `git + rev` 锁定。
- 所有 crate `#![forbid(unsafe_code)]`。

### 4.3 确定性（继承引擎第 12 章）

FixedUpdate 中禁止 `real_dt`、系统时钟、`thread_rng`、`HashMap` 迭代顺序依赖、浮点 `==`、并行迭代；时长用 tick；随机只用 `GameRng`；同 tick 多目标按 `SPEC.md` 2.5 的排序规则。伤害必须经 `DamageRequest` 统一结算，禁止在其他系统里直接改 `Health`。

### 4.4 UI 与模拟的边界

- 模拟只响应 `UiAction` 事件；egui 按钮、鼠标翻译、快捷键都只产生 `UiAction`。
- 一切视觉效果（Dew 飘落、闪白、横幅动画）在 `rootguard-app` 的 `Update` / `Render` 实现，禁止影响模拟。
- 占位渲染模式必须始终完整可玩；皮肤模式是可选增强。

### 4.5 代码质量

- 资产与存档路径禁止 `unwrap()` / `expect()`，错误走 `record_startup_error` 或 `log::warn!` + 降级。
- 禁止 `static mut`、全局可变状态、`#[allow]` 压制 clippy（除非 ADR）。
- 测试禁止 `sleep` 与真实时间。

---

## 5. 提交前检查清单

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
./scripts/check_deps.sh
./scripts/check_ip.sh          # G0 创建：grep 禁用词表（见 6），命中即失败
```

人工确认：新增公共项有文档并引用 SPEC 章节；FixedUpdate 改动后 `determinism_golden_1_4`（G4 起）仍通过；`docs/progress.md` 已更新；新增素材已登记 `assets/SOURCES.md`。

---

## 6. `scripts/check_ip.sh` 说明

G0 创建。对 `crates/ examples/ assets/data/ docs/` 下文本做大小写不敏感 grep，禁用词表存于 `scripts/ip_denylist.txt`，由项目负责人维护初始内容（你在 G0 创建空文件与脚本骨架，并在 `docs/progress.md` 提示负责人填写）。你自己不得向该表添加任何第三方作品的角色或作品名。

---

## 7. 常用命令

```bash
cargo run --release -p rootguard-app                       # 窗口运行
cargo run --release -p rootguard-app -- --level 1-4 --seed 104
cargo run -p rootguard-app --no-default-features           # 不带 devtools
ROOTGUARD_SAVE_DIR=/tmp/rg cargo run -p rootguard-app      # 隔离存档
TRELLIA_ASSETS=./assets cargo test -p rootguard-app
cargo test -p rootguard-core                               # 规则单测
cargo test -p rootguard-app --test scripted_levels         # 全部脚本解回放
RUST_LOG=rootguard=debug cargo run -p rootguard-app
```

---

## 8. 文件职责速查

| 路径 | 操作 |
|---|---|
| `SPEC.md` / `AGENT.md` | 只读 |
| `docs/progress.md` | 每次会话与每个里程碑更新 |
| `docs/decisions/ADR-*.md` | 追加 |
| `docs/levels.md` | 每关一行：威胁曲线、脚本解用时、手动用时与感受 |
| `docs/game-v1-report.md` | G5 完成后写一次 |
| `assets/data/**` | 全部数值与关卡；改数值先改这里 |
| `assets/SOURCES.md` | 每个图像 / 音频 / 字体的来源与许可证 |
| `assets/icon/rootguard-icon.png` | 项目图标（已提供），只读 |
| `tests/scripts/*.ron` | 每关脚本解 |
| `tests/golden/*.snap` | 首次生成后只读，变化需提问 |
| `scripts/check_deps.sh` / `check_ip.sh` / `ip_denylist.txt` | G0 创建后脚本只读；denylist 由负责人维护 |

---

## 9. G0 的具体任务

1. workspace `Cargo.toml`：`members = ["crates/*"]`；`[workspace.dependencies]` 含 `trellia = { git, rev }`（取引擎仓库当前 main 最新 sha）、`serde`、`ron`、`log`、`env_logger`、`thiserror`、`anyhow`、`directories`（版本用 `cargo add` 获取）。
2. 三个 crate 空壳：`rootguard-core`（lib）、`rootguard-content`（lib）、`rootguard-app`（bin + lib，features `devtools` 默认开）。每个含 `#![forbid(unsafe_code)]`、`//!` 职责文档、`README.md`。
3. `rootguard-app/src/main.rs`：组装插件（顺序见 `SPEC.md` 3.2），`ScenePlugin::new(TitleScene)`，`run_egui`；此时 `TitleScene` 只显示 "Rootguard" 文字与退出按钮。确认窗口能开、能关。
4. `assets/data/{defenders,projectiles,enemies,levels,i18n}/` 目录与 `i18n/zh.ron`、`en.ron` 空表；`assets/SOURCES.md` 表头。
5. `scripts/check_deps.sh`（参照引擎脚本结构，白名单按 `SPEC.md` 3.1）、`scripts/check_ip.sh` 与空 `ip_denylist.txt`。
6. CI：引擎同款矩阵，加 `check_ip.sh`。
7. `docs/decisions/ADR-000-template.md`、`docs/progress.md`、`docs/levels.md` 表头。
8. 验证：构建通过；临时让 `rootguard-core` 依赖 `trellia-gfx` 时 `check_deps.sh` 失败后撤销；向 denylist 临时加一个词并在代码中写入该词时 `check_ip.sh` 失败后撤销。

---

## 10. 完成标准

- `SPEC.md` 第 17 章 G0 ～ G5 全部验收条件在 `docs/progress.md` 打勾并附证据。
- `cargo test --workspace` 全绿；测试数不少于 `SPEC.md` 第 16 章列出的数量。
- 10 关均有脚本解且回放必胜；`tests/golden/level_1_4.snap` 已提交。
- `assets/SOURCES.md` 覆盖全部素材；`check_ip.sh` 通过。
- `docs/game-v1-report.md` 已提交，含：各 crate 行数与测试数、引擎补充 E-1 ～ E-5 的落地状态、已知限制、Phase 2 建议。

---

## 11. 心态提醒

- 知识产权约束不是形式，是这个项目能否公开的前提。原创不难，难的是克制"参考"的冲动。
- 关卡写不出脚本解就是关卡有问题，不是测试有问题。
- `UiAction` 是你在没有鼠标的 CI 里验证整个游戏的唯一通道，别绕过它。
- 需要改引擎时回到第 3.6 节；不在 E-1 ～ E-5 内就停下提问。
