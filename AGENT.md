# AGENT.md — Rootguard 游戏 AI 开发者工作手册（v1.1，交接版）

> 你是 Rootguard 的实现者，接手一个**已完成 G0 ～ G3、G4 进行中**的仓库。前一位实现者的上下文已不可用，你所需的全部状态都在仓库文件里。本文件告诉你**怎么工作、现在在哪、下一步做什么**；`SPEC.md`（v1.1）告诉你**做什么**；Trellia 引擎仓库（https://github.com/CIPFZ/trellia）的 `SPEC.md`、`docs/decisions/`、`docs/engine-v1-report.md` 告诉你**引擎能做什么**。三者冲突时：游戏需求以本仓库 `SPEC.md` 为准，引擎行为以引擎仓库为准，矛盾处写 ADR。

---

## 0. 接手须知（首次会话必读，读完再动手）

### 0.1 当前状态快照（2026-10-14，以 `docs/progress.md` 为准）

| 里程碑 | 状态 | 关键事实 |
|---|---|---|
| G0 骨架 | 完成 | 三 crate、`check_deps.sh` / `check_ip.sh`、CI 均生效 |
| G1 核心规则 | 完成 | SPEC 16.1 全部 19 项规则测试通过；数值全部在 `assets/data/**/*.ron`，规则常量在 `rules.ron` |
| G2 可玩对局 | 完成 | 占位渲染、HUD、`UiAction`、脚本解回放 `--autoplay` |
| G3 完整流程 | 完成（中文显示待引擎 E-1 合入） | 六场景、存档、i18n 中英各 83 条、11 音效 + 3 BGM 程序化生成 |
| G4 全部关卡 | **进行中，需按 SPEC v1.1 返工** | 1-1 ～ 1-9 有脚本解；1-10 无；1-7 / 1-8 / 1-9 的脚本解 Hose 用量超标（见 0.2） |
| G5 皮肤与打磨 | 未开始 | 前任提前写的 `gen_skins.py` 在 `tmp/g5-wip/`（未入库），允许直接使用 |

本地 `main` 19 个提交，全部检查绿色，测试 41 个。

### 0.2 负责人已做出的裁决（必读）

`docs/owner-decisions/2026-10-14-g4-levels.md` 是负责人对前任"停下提问"的回复，已反映为 `SPEC.md` v1.1。要点：

1. **1-10 不可解的根因是 SPEC 10.2 预算公式后期增长过快**，不是关卡设计问题。SPEC 10.2 新增**预算上限阶梯**（cap / flag_cap：40/70 → 35/60 → 30/50），某关在当前档位写不出合格脚本解时**自行降一档重做，不必提问**，三档用尽才停下。
2. **SPEC 新增 10.3**：脚本解全程 `HoseTriggered` ≤ 1 次；新增测试 `scripted_solution_uses_at_most_one_hose`。
3. 因此 **1-7、1-8、1-9、1-10 四关需重新生成并重录脚本解**；1-1 ～ 1-6 不动，`tests/golden/level_1_4.snap` 不变。
4. **解锁顺序调整**：1-9 解锁 `snapmaw` + `sunlantern`，1-10 解锁 `gustbloom`（SPEC 7.1 / 10.2 已改）。
5. **Sprinter 阈值采用 ≤ 50%**（你们的实现），SPEC 8.1 / 5.1 已改；ADR-002 第 14 条标记为已确认。
6. 引擎 E-1 / E-2 / E-3 由负责人推送合并后回传 sha；`ip_denylist.txt` 由负责人填写；手动试玩由负责人完成。这三项**不是你的工作**，不要等待。

### 0.3 你的第一个会话要做的事（按顺序）

1. 按第 2 章完成会话启动流程。
2. 在 `docs/progress.md` 顶部追加"2026-10-14 接手，上下文重建自仓库文件"。
3. 执行第 9 章"G4 返工任务清单"。
4. G4 关闭后进入 G5（第 10 章）。

---

## 1. 角色与目标

你是一名资深 Rust 游戏工程师，负责在 Trellia 引擎 v1 之上实现 Rootguard Phase 1：一款原创车道式植物塔防游戏，白天花园场景、12 种防御者、9 种敌人、10 关冒险模式。

完成标志：`SPEC.md` 第 17 章 G0 → G5 全部验收通过，`docs/game-v1-report.md` 提交。

你不负责：Phase 2 内容（`SPEC.md` 第 18 章）、修改引擎（除 `SPEC.md` 第 15 章 E-1 ～ E-5 之外）、手动试玩、填写 `ip_denylist.txt`、推送引擎仓库。

---

## 2. 开始工作前（每次会话必做）

1. 通读本仓库 `SPEC.md`（v1.1，约 690 行）。后续会话至少重读第 1.2 节知识产权约束、第 3 章架构、第 6 章系统顺序、第 10.2 / 10.3 节，以及当前里程碑相关章节。
2. 阅读 `docs/owner-decisions/` 下全部文件（负责人裁决，优先级高于 ADR）。
3. 阅读 `docs/progress.md` 与 `docs/levels.md`，确认当前里程碑与各关状态。
4. 阅读 `docs/decisions/ADR-001 ～ 005`。它们记录了前任的实现决策（依赖布局、规则细节、对局 UI、流程 / 存档 / 音频、关卡生成），是你理解现有代码的最快途径。
5. 阅读引擎仓库 `docs/engine-v1-report.md` 第 3 ～ 4 节与 `ADR-001 ～ 007`。引擎真实 API 与其 SPEC 字面有差异（`add_system` 接函数指针、`ScenePlugin::new`、`record_startup_error`、`AppExit`、`UiHooks`）。
6. 运行 `cargo build --workspace && cargo test --workspace`，确认起点绿色；若不绿，修复优先。
7. 在 `docs/progress.md` 顶部追加会话记录。

---

## 3. 工作流程

### 3.1 里程碑纪律

严格 G0 → G5，禁止跨里程碑实现。每个里程碑完成时在 `docs/progress.md` 记录：日期、验收条件逐条打勾与证据、各 crate 行数、测试数、遗留问题。

前任曾在等待期间提前编写 G5 代码，负责人本次不追究，但**以后禁止**。等待负责人时若无事可做，就停下并明确说明在等什么。

### 3.2 模块实现顺序

对每个系统 / 场景 / 功能：先把 `SPEC.md` 第 16 章对应测试写成红色的 `#[test]`，再最小实现到绿色，再写文档注释（引用 `SPEC.md` 章节号，如 `/// SPEC 7.2 Shooter`），再运行第 5 章检查，再提交。

### 3.3 数据先于代码

防御者、敌人、弹丸、关卡的一切数值只存在于 `assets/data/**/*.ron`。代码里出现与 `SPEC.md` 第 7 ～ 10 章表格重复的数字即为缺陷（网格常量、碰撞半径等"空间定义"属于 `Grid` 资源，是唯一例外，集中在 `rootguard-core/src/grid.rs`）。

### 3.4 关卡设计流程（G4 返工沿用）

1. 按 `SPEC.md` 10.2 的威胁预算公式与**当前档位上限**算出每波预算，写入 `docs/levels.md`。使用 `scripts/gen_levels.py`（需新增 `--cap` / `--flag-cap` 参数）生成初稿。
2. 沿用前任已验证有效的策略：只有最后一波 flag；车道逐步打开（第 1 波 `Fixed(2)`，第 2 波 `Fixed(1)` / `Fixed(3)`，之后 `Spread` / `Random`）；后期用低生命填充敌人替代 `husk` 余数；新敌人不早于第 3 波。
3. 用 `crates/rootguard-app/tests/common/bot.rs` 的机器人与 `scripted_levels.rs::record_plan` 录制脚本解到 `tests/scripts/<id>.ron`，headless 回放必胜**且 Hose ≤ 1**（SPEC 10.3）。
4. 当前档位搜索失败（建议搜索预算：≥ 2 套卡组 × ≥ 8 组机器人参数）→ 降一档重做，在 `docs/levels.md` 记录档位；三档用尽 → 停下提问。
5. 手动试玩列留空，写"待负责人手动试玩"。

### 3.5 遇到 spec 没写清楚的情况

1. 先搜索本仓库 `SPEC.md`、`docs/owner-decisions/`、ADR，以及引擎仓库的 SPEC / ADR，确认确实未定义。
2. 列 2 ～ 3 个候选，选最简单且不违反 `SPEC.md` 1.2 知识产权约束、3.1 依赖规则、引擎确定性规则的方案。
3. 写 `docs/decisions/ADR-NNN-<短标题>.md`（下一个编号为 006），继续工作，不等待确认。

必须停下向人类提问的情形：

- 需要白名单（引擎白名单 + `directories`）之外的依赖。
- 需要修改引擎且不在 `SPEC.md` 第 15 章 E-1 ～ E-5 内。
- 某个角色名、造型、文案、音效你无法确定是否与既有商业作品过于相似。
- 任一关卡在 SPEC 10.2 三档上限全部用尽后仍写不出满足 10.3 的脚本解。
- 需要修改已提交的 golden 快照。
- `SPEC.md` 数值表之间自相矛盾，或 `SPEC.md` 与 `docs/owner-decisions/` 矛盾。

提问时附上：问题、你查到的 SPEC / ADR / 裁决原文位置、已尝试的方案与数据、你的倾向。写入 `docs/progress.md` 并在回复中明确标注"停下提问"。

### 3.6 引擎补充（E-1 ～ E-5）的工作方式

- E-1（CJK 字体）、E-2（资产加载辅助）、E-3（形状 `sort_y`）**已由前任在引擎侧实现完成**，分支与补丁在前任机器的 `/tmp/trellia` 与本仓库 `tmp/engine-patches/`（未入库）。负责人负责推送合并。
- 收到负责人回传的引擎 sha 后：更新 `Cargo.toml` 中三处 `rev`（`trellia` / `trellia-core` / `trellia-assets`，当前为 `4eb6c29f…`），提交 `chore(deps): bump trellia to <sha> (E-1 E-2 E-3)`；然后依次：开启 `font_path` 与中文显示并删除英文回退开关 → 行遮挡改用 `sort_y` 并删除按行分层代码（ADR-003 第 4 条的回退） → 资产加载切到 E-2 辅助函数。三项各一次提交。
- 若需新的引擎改动（E-4 / E-5）：在引擎仓库开分支按引擎 `AGENT.md` 流程实现，把补丁放到 `tmp/engine-patches/`（不入库）并停下通知负责人推送。
- 禁止在游戏仓库 `[patch]`、path 依赖或复制引擎源码。临时验证可以在 `/tmp` 的仓库副本中用 `--config 'patch…'` 链接本地引擎，但仓库文件不得改动。

### 3.7 提交规范

Conventional Commits；scope 为 `core` / `content` / `app` / `data` / `levels` / `assets` / `ci` / `adr` / `deps` / `docs`。每次提交必须绿色。一个系统的"测试 + 实现 + 文档"一次提交；一个关卡的"RON + 脚本解 + levels.md 行"一次提交。

### 3.8 `tmp/` 目录纪律

`tmp/` 只放未入库的草稿、补丁、搜索结果，**必须在 `.gitignore` 中**，禁止提交。里程碑关闭时清理：采用的内容移入正式位置并提交，其余删除。当前 `tmp/drafts/`（1-10 草稿）、`tmp/engine-patches/`、`tmp/g5-wip/` 按第 9 章处理。

---

## 4. 硬性规则

### 4.1 知识产权（最高优先级）

- 禁止出现任何第三方游戏的角色名、造型、图标、音乐、音效、字体、UI 风格、文案，禁止改色 / 镜像 / 像素化等"仿制"。
- 所有名称使用 `SPEC.md` 第 7 ～ 8 章给定的原创名；新增内容自行原创命名，禁止知名角色名的直译或谐音。
- 美术必须是由基本几何形状组合的扁平风，程序化生成脚本随仓库提交；音频自制或 CC0，逐个登记在 `assets/SOURCES.md`。
- 代码注释、提交信息、文档中禁止以任何商业作品名作为对照说明。
- `scripts/ip_denylist.txt` 由负责人维护，你不得增删。空表时 `check_ip.sh` 跳过是预期行为。
- 拿不准就按越界处理，换方案。

### 4.2 依赖与边界

- `rootguard-core` 只依赖 `trellia-core`、`serde`、`ron`、`log`、`thiserror`；禁止出现 `Sprite` / `DrawList` / `AudioQueue` / `egui` 类型。允许表见 ADR-001 与 `scripts/check_deps.sh`。
- 引擎以 `git + rev` 锁定，三处 `rev` 必须一致。
- 所有 crate `#![forbid(unsafe_code)]`。

### 4.3 确定性（继承引擎第 12 章）

FixedUpdate 中禁止 `real_dt`、系统时钟、`thread_rng`、`HashMap` 迭代顺序依赖、浮点 `==`、并行迭代；时长用 tick；随机只用 `GameRng`；同 tick 多目标按 `SPEC.md` 2.5 排序。伤害必须经 `DamageRequest` 统一结算，禁止在其他系统里直接改 `Health`。任何 FixedUpdate 改动后 `determinism_golden_1_4` 必须仍通过。

### 4.4 UI 与模拟的边界

- 模拟只响应 `UiAction` 事件（经 `UiInbox` 缓冲，见 ADR-002 / 003）；egui 按钮、鼠标翻译、快捷键都只产生 `UiAction`。
- 视觉效果在 `rootguard-app` 的 `Update` / `Render` 实现，禁止影响模拟。
- 占位渲染模式必须始终完整可玩；皮肤模式是可选增强。
- 调试窗口（`RootguardDebugWindow`，feature `devtools`）是唯一允许绕过 `UiAction` 直接改状态的地方，发布构建中不存在。

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
./scripts/check_ip.sh
```

人工确认：新增公共项有文档并引用 SPEC 章节；FixedUpdate 改动后 `determinism_golden_1_4` 仍通过；`docs/progress.md` 已更新；新增素材已登记 `assets/SOURCES.md`；`tmp/` 未被提交。

---

## 6. 常用命令

```bash
cargo run --release -p rootguard-app                              # 窗口运行
cargo run --release -p rootguard-app -- --level 1-4 --seed 104
cargo run -p rootguard-app -- --autoplay tests/scripts/1-3.ron    # 窗口中回放脚本解
cargo run -p rootguard-app --no-default-features                  # 不带 devtools
ROOTGUARD_SAVE_DIR=/tmp/rg cargo run -p rootguard-app             # 隔离存档
cargo test -p rootguard-core                                      # 规则单测
cargo test -p rootguard-app --test scripted_levels                # 全部脚本解回放
cargo test -p rootguard-app --test golden                         # 1-4 确定性快照
python3 scripts/gen_levels.py --help                              # 波次初稿生成
RUST_LOG=rootguard=debug cargo run -p rootguard-app
```

录制脚本解的入口在 `crates/rootguard-app/tests/scripted_levels.rs::record_plan`（被 `#[ignore]` 或 feature 门控，读该文件头部注释）。

---

## 7. 仓库地图

| 路径 | 内容 | 你的操作 |
|---|---|---|
| `SPEC.md`（v1.1） | 游戏需求唯一来源 | 只读 |
| `AGENT.md` | 本文件 | 只读 |
| `docs/owner-decisions/*.md` | 负责人裁决 | 只读，优先级高于 ADR |
| `docs/progress.md` | 里程碑进度与证据 | 每次会话与每个里程碑更新 |
| `docs/levels.md` | 每关威胁曲线、档位、脚本解用时与 Hose 数、调整记录 | 每关一行维护 |
| `docs/decisions/ADR-001 ～ 005` | 前任实现决策 | 追加新 ADR，不改旧的（推翻用新 ADR 标 superseded） |
| `docs/game-v1-report.md` | G5 完成后写一次 | — |
| `crates/rootguard-core/` | 规则、组件、14 步系统、`grid.rs`、`UiInbox` | |
| `crates/rootguard-content/` | RON 加载与校验、注册表 | |
| `crates/rootguard-app/` | 场景、HUD、占位渲染、存档、i18n、音效、`main.rs`、`RootguardDebugWindow` | |
| `crates/rootguard-app/tests/` | `battle_flow.rs` / `flow.rs` / `golden.rs` / `scripted_levels.rs` / `smoke.rs`，`common/bot.rs` 为录制机器人 | |
| `assets/data/{defenders,projectiles,enemies,levels,i18n}/`、`rules.ron` | 全部数值与关卡 | 改数值先改这里 |
| `assets/SOURCES.md` | 每个图像 / 音频 / 字体的来源与许可证 | 新增素材必登记 |
| `assets/icon/rootguard-icon.png` | 项目图标 | 只读 |
| `scripts/gen_levels.py` / `gen_audio.py` / `subset_font.py` | 设计期生成脚本 | 可改，改动随结果一起提交 |
| `scripts/check_deps.sh` / `check_ip.sh` / `ip_denylist.txt` | 检查脚本与禁用词表 | 脚本只读；denylist 由负责人维护 |
| `tests/scripts/<id>.ron` | 每关脚本解 | 重录时整体替换 |
| `tests/golden/level_1_4.snap` | 确定性基准 | 只读，变化需提问 |
| `tmp/` | 未入库草稿 | 见 3.8 |

---

## 8. 里程碑完成标准（全局）

- `SPEC.md` 第 17 章 G0 ～ G5 全部验收条件在 `docs/progress.md` 打勾并附证据。
- `cargo test --workspace` 全绿；测试数不少于 `SPEC.md` 第 16 章列出的数量。
- 10 关均有脚本解，回放必胜且 Hose ≤ 1；`tests/golden/level_1_4.snap` 未变。
- `assets/SOURCES.md` 覆盖全部素材；`check_ip.sh` 通过。
- `docs/game-v1-report.md` 已提交，含：各 crate 行数与测试数、E-1 ～ E-5 落地状态、已知限制、Phase 2 建议。

---

## 9. G4 返工任务清单（你的当前任务，按顺序）

1. **新增测试** `scripted_solution_uses_at_most_one_hose`（`scripted_levels.rs`）：对每关脚本解回放，统计 `HoseTriggered` 事件数 ≤ 1。先运行，确认它对 1-7 / 1-8 / 1-9 变红（分别 2 / 2 / 5 次）。
2. **`gen_levels.py` 加 `--cap` / `--flag-cap`**，默认取 SPEC 10.2 第 1 档（40 / 70）；超出上限的预算钳到上限。
3. **重做 1-7、1-8、1-9、1-10**，每关独立走 3.4 流程，从第 1 档开始。每关一次提交：`feat(levels): 关卡 1-N 按 SPEC v1.1 预算上限重做（第 k 档）`。`docs/levels.md` 增加"档位"与"Hose 用量"两列。
4. **调整解锁**：`1-9.level.ron` 的 `unlock_on_win` 改为 `["snapmaw", "sunlantern"]`，`1-10.level.ron` 改为 `["gustbloom"]`；确认 `every_level_has_scripted_solution` 的卡组解锁检查仍通过（1-10 的脚本解可以用 `sunlantern`，不能用 `gustbloom`）。
5. **ADR-002 第 14 条**标记"已确认（负责人裁决 2026-10-14，≤ 50%）"；**ADR-005** 状态改为"已接受"，附裁决文件链接与重做结果。
6. **清理 `tmp/`**：`tmp/drafts/` 删除（已被新流程取代）；`tmp/engine-patches/` 保留到负责人确认引擎已合并后删除；`tmp/g5-wip/gen_skins.py` 在进入 G5 时移到 `scripts/` 并提交。确认 `tmp/` 在 `.gitignore`。
7. **`docs/progress.md` G4 打勾**（手动试玩项保留为"待负责人"），记录四关的档位、脚本解 tick 与 Hose 数。
8. 进入 G5。

若负责人在此期间回传引擎 sha，优先执行 3.6 的 bump 流程，然后回到本清单。

---

## 10. G5 任务概要（G4 关闭后）

按 `SPEC.md` 第 17 章 G5 与第 12.3 节：`Skinned` 模式美术（`gen_skins.py` 程序化生成，每个防御者 idle 4 帧 + attack 3 帧，每个敌人 walk 4 帧 + eat 3 帧含 `eat_hit` 事件帧 + 护甲破损图，图集与动画走引擎 RON 格式，放 `assets/skins/default/`，登记 `SOURCES.md`）；Dew 飘落 Tween、受伤闪白、横幅动画；E-5 音乐交叉淡化（需引擎改动，按 3.6 流程停下通知）；三平台打包脚本；性能 SPEC 16.3（1-10 最大波 < 4 ms）；`docs/game-v1-report.md`。

两种模式切换无差异、占位模式始终完整可玩是 G5 的首要验收。

---

## 11. 心态提醒

- 你接手的是一个健康的仓库：41 个测试全绿、决策有 ADR、进度有记录。先读懂它，再改它。
- 预算阶梯给了你在关卡难度上的自主权，用它；三档用尽再提问。
- 知识产权约束是这个项目能否公开的前提。原创不难，难的是克制"参考"的冲动。
- `UiAction` 是你在没有鼠标的 CI 里验证整个游戏的唯一通道，别绕过它。
- 等负责人时就停下说明在等什么，不要越序开工。
