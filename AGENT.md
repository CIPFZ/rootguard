# AGENT.md — Rootguard 游戏 AI 开发者工作手册（v1.3，美术阶段）

> 你是 Rootguard 的实现者，接手一个**G0 ～ G5 全部关闭、v0.1.0 已发布、正在进入美术阶段（A0 ～ A3）**的仓库。前一位实现者的上下文已不可用，你所需的全部状态都在仓库文件里。本文件告诉你**怎么工作、现在在哪、下一步做什么**；`SPEC.md`（v1.2）告诉你玩法与数值；**`LORE.md` 告诉你世界与角色是谁；`ART-SPEC.md` 告诉你它们长什么样、怎么动、怎么做出来**；Trellia 引擎仓库告诉你**做什么**；Trellia 引擎仓库（https://github.com/CIPFZ/trellia）的 `SPEC.md`、`docs/decisions/`、`docs/engine-v1-report.md` 告诉你**引擎能做什么**。三者冲突时：游戏需求以本仓库 `SPEC.md` 为准，引擎行为以引擎仓库为准，矛盾处写 ADR。

---

## 0. 接手须知（首次会话必读，读完再动手）

### 0.1 当前状态快照（2026-10-14，以 `docs/progress.md` 与 `docs/game-v1-report.md` 为准）

| 项 | 状态 |
|---|---|
| G0 ～ G5 | **全部关闭**。SPEC 17 章可自动验证的验收全部通过；`docs/game-v1-report.md` 已提交 |
| 测试 | 51 个全绿（core 23 / content 6 / app 22），另 2 个 `#[ignore]` 工具 `record_scripts` / `search_plans`；SPEC 16 章 29 个测试同名存在 |
| 引擎 | CIPFZ/trellia `24c4d712…`，E-1 / E-2 / E-3 / E-5 已合入并在游戏侧落地（ADR-009）；E-4 未实现（按需项，无需求） |
| 关卡 | 10 关均有脚本解，回放必胜且 Hose ≤ 1；1-8 / 1-10 第 1 档、1-9 第 2 档；`tests/golden/level_1_4.snap` 自 G4 起未变 |
| 内容 | 12 防御者、9 敌人、原创皮肤（`gen_skins.py`，247 帧）、11 音效 + 3 BGM（`gen_audio.py`）、CJK 字体子集（OFL） |
| 发布 | **v0.1.0 已发布**（tag 7f3049a，双许可、CHANGELOG、Release 工作流就绪） |
| 美术 | 占位几何皮肤 `default` 为当前唯一皮肤；`LORE.md` 与 `ART-SPEC.md` 已定稿，概念图在 `docs/art/v3-*.png`；**A0 未开始** |
| 待真人 | 手动试玩 10 关填 `docs/levels.md`（负责人已试玩反馈"整体不错"，细表待填）；macOS / Windows 实机启动 |

### 0.2 负责人裁决（必读，按日期顺序）

- `docs/owner-decisions/2026-10-14-g4-levels.md`：G4 返工裁决（预算上限阶梯、Hose ≤ 1、解锁顺序、Sprint 阈值）。已全部落地。
- `docs/owner-decisions/2026-10-14-g5-closure.md`：批准 1-3 重录与 `audio_events_mapped` 断言修改；许可证 MIT OR Apache-2.0；收尾期安排。已落地。
- `docs/owner-decisions/2026-10-15-bite-rhythm.md`、`2026-10-15-chill-bite-phase.md`：啃食动画与音效改为由 `Biter` 计时器相位驱动。**这一"进度驱动"做法是 ART-SPEC 3.1 全部计时类动画的范式。**
- `ART-SPEC.md` 第 11 ～ 12 章：**当前生效**。美术阶段 A0 ～ A3 的交付、审核门与允许改动清单。

### 0.3 美术阶段的边界

SPEC v1.2 第 20.1 节的冻结仍然有效，**仅按 ART-SPEC 第 12 章解冻**：允许新增皮肤 `garden`、特效、音效、关卡 `time_of_day` 字段、关卡独白与结局文本、`tools/artgen` 及其白名单依赖。**模拟层、数值、golden 零改动**；禁止新增防御者 / 敌人 / 关卡 / 行为类型。难度校准（SPEC 20.2）在负责人填表后与美术并行处理。

### 0.4 你的第一个会话要做的事

1. 按第 2 章完成会话启动流程（新增：通读 `LORE.md` 与 `ART-SPEC.md`，看 `docs/art/v3-*.png`）。
2. 在 `docs/progress.md` 顶部追加"接手（美术阶段）"，新建"## 美术阶段 A0 ～ A3"章节。
3. 执行第 9 章任务清单（从 A0 开始）。

---

## 1. 角色与目标

你是一名资深 Rust 游戏工程师，负责在 Trellia 引擎 v1 之上实现 Rootguard Phase 1：一款原创车道式植物塔防游戏，白天花园场景、12 种防御者、9 种敌人、10 关冒险模式。

里程碑 G0 → G5 已完成，v0.1.0 已发布。当前目标：按 `ART-SPEC.md` 完成美术阶段 A0 → A3，用原创的 `garden` 皮肤、特效、战场与叙事替换占位几何体，使负责人能打 `v0.2.0` tag；期间按 SPEC 20.2 处理负责人的难度反馈。

你不负责：Phase 2 内容（`SPEC.md` 第 18 章）、修改引擎（除 `SPEC.md` 第 15 章 E-4 之外；E-1 / E-2 / E-3 / E-5 已合入）、手动试玩、填写 `ip_denylist.txt`、打 Release tag。你已被授权直接推送本仓库与引擎仓库。

---

## 2. 开始工作前（每次会话必做）

1. 通读本仓库 `SPEC.md`（v1.2）、**`LORE.md`、`ART-SPEC.md`**。后续会话至少重读 SPEC 1.2 知识产权约束、SPEC 20 章、ART-SPEC 第 1 章风格宪法、第 3 章动画规范与当前里程碑的设计卡。
2. 阅读 `docs/owner-decisions/` 下全部文件（负责人裁决，优先级高于 ADR）。
3. 阅读 `docs/progress.md` 与 `docs/levels.md`，确认当前里程碑与各关状态。
4. 阅读 `docs/game-v1-report.md` 与 `docs/decisions/ADR-001 ～ 009`（依赖布局、规则细节、对局 UI、流程 / 存档 / 音频、关卡生成、Hose 规则落地、皮肤渲染、特效 / 打包 / 性能、引擎 E-x 落地）。这是理解现有代码的最快途径。
5. 阅读引擎仓库 `docs/engine-v1-report.md` 第 3 ～ 4 节与 `ADR-001 ～ 007`。引擎真实 API 与其 SPEC 字面有差异（`add_system` 接函数指针、`ScenePlugin::new`、`record_startup_error`、`AppExit`、`UiHooks`）。
6. 运行 `cargo build --workspace && cargo test --workspace`，确认起点绿色；若不绿，修复优先。
7. 在 `docs/progress.md` 顶部追加会话记录。

---

## 3. 工作流程

### 3.1 美术阶段纪律

严格 A0 → A1 → A2 → A3；**A1 是审核门**，负责人未批准 `review-A1.png` 前禁止开始 A2 的动画。每个里程碑完成时在 `docs/progress.md` 记录日期、验收证据、资产数量（角色 / 部件 / 片段 / 帧）、图集尺寸。

等待负责人（试玩反馈、tag、Phase 2 决定）时若无事可做，就停下并明确说明在等什么；禁止以"顺手"为由开始任何 Phase 2 内容。

### 3.2 模块实现顺序

对每个系统 / 场景 / 功能：先把 `SPEC.md` 第 16 章对应测试写成红色的 `#[test]`，再最小实现到绿色，再写文档注释（引用 `SPEC.md` 章节号，如 `/// SPEC 7.2 Shooter`），再运行第 5 章检查，再提交。

### 3.2b 角色制作顺序（A1 / A2 每个角色）

1. 读该角色在 `ART-SPEC.md` 第 4 ～ 5 章的设计卡与 `LORE.md` 对应行，列出每个特征的设定出处。
2. 写 `art/characters/<id>/parts/*.svg` 与 `rig.ron`，默认位姿渲染一张 PNG 自查：剪影、配色（只用调色板）、表情是否符合卡上的"性格"。
3. 写 `clips.ron`，时长用 `Rule(...)` 绑定而不是硬编码；事件帧按 ART-SPEC 3.1。
4. `artgen` 生成，跑 `art_clips_match_rules` / `art_assets_up_to_date`，实机看一眼。
5. 一次提交：`feat(art): <id> 部件 / rig / 片段（ART-SPEC 4.N）`。

### 3.3 数据先于代码

防御者、敌人、弹丸、关卡的一切数值只存在于 `assets/data/**/*.ron`。代码里出现与 `SPEC.md` 第 7 ～ 10 章表格重复的数字即为缺陷（网格常量、碰撞半径等"空间定义"属于 `Grid` 资源，是唯一例外，集中在 `rootguard-core/src/grid.rs`）。

### 3.4 关卡流程（收尾期用于 SPEC 20.2 难度校准）

1. 按 `SPEC.md` 10.2 的威胁预算公式与**当前档位上限**算出每波预算，写入 `docs/levels.md`。使用 `scripts/gen_levels.py`（需新增 `--cap` / `--flag-cap` 参数）生成初稿。
2. 沿用前任已验证有效的策略：只有最后一波 flag；车道逐步打开（第 1 波 `Fixed(2)`，第 2 波 `Fixed(1)` / `Fixed(3)`，之后 `Spread` / `Random`）；后期用低生命填充敌人替代 `husk` 余数；新敌人不早于第 3 波。
3. 用 `crates/rootguard-app/tests/common/bot.rs` 的机器人与 `scripted_levels.rs::record_plan` 录制脚本解到 `tests/scripts/<id>.ron`，headless 回放必胜**且 Hose ≤ 1**（SPEC 10.3）。
4. 当前档位搜索失败（ADR-006 第 3 条：3 套卡组 × 512 组参数无合格解）→ 降一档重做，在 `docs/levels.md` 记录档位；三档用尽 → 按 SPEC 20.2 允许 `start_dew` +25 并写 ADR，仍不可解才停下提问。
5. 校准触发只来自负责人填写的"主观难度"列；1-4 不得改动（golden）。保留每关的调整历史。

### 3.5 遇到 spec 没写清楚的情况

1. 先搜索本仓库 `SPEC.md`、`docs/owner-decisions/`、ADR，以及引擎仓库的 SPEC / ADR，确认确实未定义。
2. 列 2 ～ 3 个候选，选最简单且不违反 `SPEC.md` 1.2 知识产权约束、3.1 依赖规则、引擎确定性规则的方案。
3. 写 `docs/decisions/ADR-NNN-<短标题>.md`（下一个编号为 010），继续工作，不等待确认。

必须停下向人类提问的情形：

- 需要白名单（引擎白名单 + `directories`）之外的依赖。
- 需要修改引擎且不在 `SPEC.md` 第 15 章 E-1 ～ E-5 内。
- 某个角色名、造型、文案、音效你无法确定是否与既有商业作品过于相似。
- 任一关卡在 SPEC 10.2 三档上限用尽且 20.2 的 `start_dew` +25 之后仍写不出满足 10.3 的脚本解。
- 填写后的 `scripts/ip_denylist.txt` 使 `check_ip.sh` 命中现有仓库内容。
- 某个角色按设计卡做出来后你自己觉得"像某个已有作品的角色"：停下，附图与你的担忧。
- A1 审核表完成：停下等负责人批准。
- 需要修改已提交的 golden 快照。
- `SPEC.md` 数值表之间自相矛盾，或 `SPEC.md` 与 `docs/owner-decisions/` 矛盾。

提问时附上：问题、你查到的 SPEC / ADR / 裁决原文位置、已尝试的方案与数据、你的倾向。写入 `docs/progress.md` 并在回复中明确标注"停下提问"。

### 3.6 引擎改动

- E-1 / E-2 / E-3 / E-5 已合入引擎 main（`24c4d712…`）并在游戏侧落地，游戏不再有任何引擎回退代码。
- 收尾期唯一允许的引擎改动是 E-4（headless egui 指针注入），且只有在出现具体测试需求时才做；流程：在引擎仓库开分支按引擎 `AGENT.md` 实现 → 推送并等 CI 绿 → 合并 → 游戏 `Cargo.toml` 三处 `rev` 一致更新，提交 `chore(deps): bump trellia to <sha> (E-4)`。
- 发现引擎缺陷：同样在引擎仓库修复（测试先行 + ADR）后 bump rev；禁止在游戏侧绕过。
- 禁止 `[patch]`、path 依赖或复制引擎源码。

### 3.7 提交规范

Conventional Commits；scope 为 `core` / `content` / `app` / `data` / `levels` / `assets` / `ci` / `adr` / `deps` / `docs`。每次提交必须绿色。一个系统的"测试 + 实现 + 文档"一次提交；一个关卡的"RON + 脚本解 + levels.md 行"一次提交。

### 3.8 `tmp/` 目录纪律

`tmp/` 只放未入库的草稿、补丁、搜索结果，**必须在 `.gitignore` 中**，禁止提交。里程碑关闭时清理：采用的内容移入正式位置并提交，其余删除。当前 `tmp/` 应为空（前任已清理）；若发现残留，确认无用后删除。

---

## 4. 硬性规则

### 4.1 知识产权（最高优先级）

- 禁止出现任何第三方游戏的角色名、造型、图标、音乐、音效、字体、UI 风格、文案，禁止改色 / 镜像 / 像素化等"仿制"。
- 所有名称使用 `SPEC.md` 第 7 ～ 8 章给定的原创名；新增内容自行原创命名，禁止知名角色名的直译或谐音。
- 美术必须从 SVG 部件程序化渲染（ART-SPEC 9），`docs/art/*.png` 概念图只作参考板，禁止描摹、位图化或进入资产目录；每个造型特征必须能指向 `LORE.md` 或 ART-SPEC 设计卡的一句话。音频自制或 CC0，逐个登记在 `assets/SOURCES.md`。
- 代码注释、提交信息、文档中禁止以任何商业作品名作为对照说明。
- `scripts/ip_denylist.txt` 由负责人维护，你不得增删。空表时 `check_ip.sh` 跳过是预期行为。
- 拿不准就按越界处理，换方案。

### 4.2 依赖与边界

- `rootguard-core` 只依赖 `trellia-core`、`serde`、`ron`、`log`、`thiserror`；禁止出现 `Sprite` / `DrawList` / `AudioQueue` / `egui` 类型。允许表见 ADR-001 与 `scripts/check_deps.sh`。
- `tools/artgen` 额外允许 `resvg` / `usvg` / `tiny-skia` / `png`（ART-SPEC 12，负责人已批准，需写 ADR 并更新 `check_deps.sh` 允许表）；这些依赖禁止出现在三个游戏 crate 中。
- 引擎以 `git + rev` 锁定，三处 `rev` 必须一致（当前 `24c4d712ff3c157dad12ac7d50836476e699b8fa`）。
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
cargo test -p rootguard-app --test scripted_levels -- --ignored search_plans   # 关卡参数搜索（校准用）
cargo run --release -p rootguard-app --example perf_1_10          # SPEC 16.3 性能测量
./scripts/package.sh                                              # 本地打包（产物在 dist/）
python3 scripts/gen_levels.py --help                              # 波次初稿生成
RUST_LOG=rootguard=debug cargo run -p rootguard-app
```

录制脚本解的入口在 `crates/rootguard-app/tests/scripted_levels.rs::record_plan`（被 `#[ignore]` 或 feature 门控，读该文件头部注释）。

---

## 7. 仓库地图

| 路径 | 内容 | 你的操作 |
|---|---|---|
| `SPEC.md`（v1.2） | 玩法与数值唯一来源 | 只读；**唯一例外**：第 13 章音效表可按 ART-SPEC 12 追加行（裁决八） |
| `AGENT.md` | 本文件 | 只读 |
| `docs/owner-decisions/*.md` | 负责人裁决 | 只读，优先级高于 ADR |
| `docs/progress.md` | 里程碑进度与证据 | 每次会话与每个里程碑更新 |
| `docs/levels.md` | 每关威胁曲线、档位、脚本解用时与 Hose 数、调整记录 | 每关一行维护 |
| `docs/decisions/ADR-001 ～ 009` | 前任实现决策 | 追加新 ADR，不改旧的（推翻用新 ADR 标 superseded） |
| `docs/game-v1-report.md` | Phase 1 实现报告 | 收尾任务完成后更新第 6 节 |
| `crates/rootguard-core/` | 规则、组件、14 步系统、`grid.rs`、`UiInbox` | |
| `crates/rootguard-content/` | RON 加载与校验、注册表 | |
| `crates/rootguard-app/` | 场景、HUD、占位渲染、存档、i18n、音效、`main.rs`、`RootguardDebugWindow` | |
| `crates/rootguard-app/tests/` | `battle_flow.rs` / `flow.rs` / `golden.rs` / `scripted_levels.rs` / `smoke.rs`，`common/bot.rs` 为录制机器人 | |
| `assets/data/{defenders,projectiles,enemies,levels,i18n}/`、`rules.ron` | 全部数值与关卡 | 改数值先改这里 |
| `assets/SOURCES.md` | 每个图像 / 音频 / 字体的来源与许可证 | 新增素材必登记 |
| `assets/icon/rootguard-icon.png` | 项目图标 | 只读 |
| `scripts/gen_levels.py` / `gen_audio.py` / `gen_skins.py` / `subset_font.py` / `package.sh` | 设计期生成与打包脚本 | 可改，改动随结果一起提交 |
| `scripts/check_deps.sh` / `check_ip.sh` / `ip_denylist.txt` | 检查脚本与禁用词表 | 脚本只读；denylist 由负责人维护 |
| `tests/scripts/<id>.ron` | 每关脚本解 | 重录时整体替换 |
| `assets/skins/default/` | 皮肤图集、清单与 46 个动画片段 | 由 `gen_skins.py` 重新生成，不手改 |
| `CHANGELOG.md` / `LICENSE-MIT` / `LICENSE-APACHE` | 发布文件（待创建） | 第 9 章 |
| `tests/golden/level_1_4.snap` | 确定性基准 | 只读，变化需提问 |
| `tmp/` | 未入库草稿 | 见 3.8 |

---

## 8. Phase 1 完成标准（全局，收尾期以此为终点）

- `SPEC.md` 第 17 章 G0 ～ G5 全部验收条件在 `docs/progress.md` 打勾并附证据。
- `cargo test --workspace` 全绿；测试数不少于 `SPEC.md` 第 16 章列出的数量。
- 10 关均有脚本解，回放必胜且 Hose ≤ 1；`tests/golden/level_1_4.snap` 未变。
- `assets/SOURCES.md` 覆盖全部素材；`check_ip.sh` 通过。
- `docs/game-v1-report.md` 第 6 节只剩真正需要真人或 Phase 2 的项。
- SPEC 20.3 发布清单除"打 tag"外全部完成；负责人试玩反馈已按 20.2 处理完毕。

---

## 9. 美术阶段任务清单（你的当前任务，按顺序）

### A0 管线（先做）

1. ADR-013：`tools/artgen` 的设计与白名单依赖；更新 `check_deps.sh`。
2. `art/palette.ron`（ART-SPEC 1.3）；`tools/artgen`：解析 rig / clips → 逐帧合成 → `resvg` 渲染 → 图集打包 → 输出引擎 `*.atlas.ron` / `anim/*.anim.ron` / `skin.ron`；校验项见 ART-SPEC 9.2；确定性输出。
3. 两个样板角色：`pebble_pod`（ART-SPEC 4.1）与 `husk`（5.1），含全部片段、闪白帧、`attack` 与 `eat` 的进度驱动。
4. 游戏侧：皮肤清单支持多皮肤（`default` / `garden`），`SkinMode::Skinned` 可选皮肤名；`attack` / `produce` / `fuse` / `arm` 片段的进度驱动（与裁决四同一机制，推广到 ART-SPEC 3.1 全部计时类）。
5. 测试：`art_assets_up_to_date`、`art_clips_match_rules`、`skin_garden_loads`；golden 不变。
6. 实机：两个角色在 1-1 中显示与动画正常，截图入 `docs/progress.md`。

### A1 全员静态（审核门）

7. 其余 19 个角色的部件、rig、默认位姿、受损 / 护甲变体、卡牌头像（ART-SPEC 2.4、4.13）。
8. `artgen review` 输出 `docs/art/review-A1.png` 与剪影版 `review-A1-silhouette.png`。
9. **停下**，等负责人批准；按意见修改后再提交一次审核表。

### A2 全员动画与特效

10. 全部片段（ART-SPEC 3.1）、弹丸与露水（第 6 章）、特效（第 7 章）、新增音效（第 12 章清单，`gen_audio.py` 扩展并登记）、Chill 表现、护甲破碎替换。
11. 实机 1-9 可读性与特效噪音自查；性能 `perf_1_10` 仍 < 4 ms。

### A3 战场与叙事

12. 背景分层、时段色调与关卡 `time_of_day`、HUD 皮肤、关卡独白 i18n、结局静帧、标题背景（ART-SPEC 8）。
13. `SkinMode::Skinned` 默认 `garden`；`CHANGELOG` 0.2.0 条目；`docs/game-v1-report.md` 追加"美术阶段"节。
14. 停下，通知负责人打 `v0.2.0`。

### 并行：难度校准

负责人填 `docs/levels.md` 后按 SPEC 20.2 处理，每关一次提交，可与 A 阶段交错，但不得与某个 A 里程碑的收尾提交混在一起。

## 10. Phase 2 的边界

Phase 2（SPEC 第 18 章列出的夜晚、水域、屋顶、Boss、生存 / 无尽、商店、图鉴等）需要负责人发起新的 spec 版本。在那之前：不要设计、不要预埋、不要在 ADR 或报告以外的地方讨论。你可以在 `docs/game-v1-report.md` 第 7 节追加你认为对 Phase 2 有价值的技术建议（例如引擎加性着色、截图回归），仅限记录。

## 11. 心态提醒

- 你接手的是一个已发布的仓库：54 个测试全绿、12 份 ADR、v0.1.0。美术阶段的首要美德是"每个特征有出处"：画之前先回答它在 LORE 里是谁。
- 难度校准只响应真人反馈，不要凭机器人合格率自行调整。
- 概念图是方向不是答案，SVG 重建时忠于设计卡的文字描述，而不是像素。
- 知识产权约束是这个项目能否公开的前提。原创不难，难的是克制"参考"的冲动。
- `UiAction` 是你在没有鼠标的 CI 里验证整个游戏的唯一通道，别绕过它。
- 等负责人时就停下说明在等什么，不要开 Phase 2。
