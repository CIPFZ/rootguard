# Rootguard 开发进度

## 会话记录（新记录追加在最上方）

- 2026-10-14 会话 1（续）：负责人批准对 1-9 / 1-10 做第三次波次调整；1-9 可解并提交，1-10 仍不可解，再次停下提问。
- 2026-10-13 会话 1：通读 SPEC.md / AGENT.md、引擎 v1 报告、ADR-001 ～ 007 与 `examples/lane_demo`；
  计划完成 G0 骨架并开始 G1。

## 给项目负责人的待办

- [ ] **【阻塞 G4】1-10 在第三次调整后仍无法写出脚本解**（AGENT.md 3.5）：详见 `docs/levels.md`“1-4 ～ 1-10 的设计与调整记录”。
  可选方向：放缓后期预算曲线（偏离 SPEC 10.2 公式）、提高 1-10 起始 Dew（偏离 10.2 关卡表）、或缩减波数。

- [ ] 填写 `scripts/ip_denylist.txt`（AGENT.md 第 6 章；实现者不得自行添加第三方作品名）。
- [ ] **E-1 / E-2 / E-3 引擎补充需推送**：实现已在引擎本地分支链 `feat/e2-world-asset-helpers`（56082a4）→
  `feat/e3-sorted-shapes`（8484ce3）→ `feat/e1-cjk-font`（76be022）（补丁副本 `tmp/engine-patches/0001～0003`，未入库）完成并通过引擎全部检查；本环境无引擎仓库推送权限与 `gh`，
  需负责人推送 / 合并后，游戏仓库再 `chore(deps): bump trellia to <sha> (E-2)` / `(E-3)`。
  在此之前游戏按 SPEC 12.2 的回退方案按行分层（不依赖 E-3），界面按 SPEC 11.6 以英文显示（不依赖 E-1）。
  bump 后：`i18n::CJK_FONT_AVAILABLE = true`，`main.rs` 设置 `font_path: Some("assets/fonts/RootguardCJK-Regular.otf")`。
- [ ] **手动试玩 1-1 ～ 1-3**（G2 验收“手动完整玩通 1-3”）：实现者只能在 Xvfb 中用 xdotool 点击验证鼠标通路、
  用 `--autoplay` 在窗口中回放脚本解（1-3 胜利，截图证据），无法代替真人试玩与主观难度记录。
- [ ] **确认 Sprint 阈值**：SPEC 8.1“hp < 50%”与 16.1“hp ≤ 5 后速度 0.675”在 hp = 5 时矛盾，
  暂按验收测试取“≤ 50%”（ADR-002 第 14 条）。
- [ ] 仓库原有目录 `assests/icon/` 疑为笔误，已移动为 SPEC / AGENT / README 所述的 `assets/icon/`（文件未改动）。

## 里程碑

| 里程碑 | 状态 |
|---|---|
| G0 骨架 | ✅ 2026-10-13 |
| G1 核心规则 | ✅ 2026-10-13（E-2 待推送，见下） |
| G2 可玩对局 | ✅ 2026-10-13（手动试玩待负责人，E-3 待推送） |
| G3 完整流程 | ✅ 2026-10-13（中文显示待 E-1 合入并 bump rev） |
| G4 全部关卡 | ⏸ 1-1 ～ 1-9 完成；1-10 无脚本解，待负责人裁决 |
| G5 皮肤与打磨 | 未开始 |

## G0 骨架（2026-10-13 完成）

验收（SPEC.md 17）：

- [x] workspace `Cargo.toml`：`members = ["crates/*"]`，引擎锁定 `rev = 4eb6c29f2b23132ecea106264bde8fd4e151efe8`
  （引擎 main 最新提交，M5 验收后）；`trellia` / `trellia-core` / `trellia-assets` 三项同 rev（ADR-001）。
- [x] 三 crate 空壳：`rootguard-core`（lib）、`rootguard-content`（lib）、`rootguard-app`（bin `rootguard` + lib，
  feature `devtools` 默认开），均含 `#![forbid(unsafe_code)]`、`//!` 职责文档、`README.md`。
- [x] `main.rs` 按 SPEC.md 3.2 顺序组装插件，`ScenePlugin::new(TitleScene)`，`run_egui`。
  证据：Xvfb 下运行截图显示 "Rootguard" 标题与 Quit 按钮；xdotool 点击 Quit 后进程以 0 退出。
- [x] `assets/data/{defenders,projectiles,enemies,levels,i18n}/`，`i18n/zh.ron`、`en.ron` 空表；`assets/SOURCES.md` 表头。
- [x] `scripts/check_deps.sh`（引擎脚本结构 + R1 传递检查 + R2 源码检查 + R3 来源检查）、`scripts/check_ip.sh`、空 `ip_denylist.txt`。
- [x] CI：`.github/workflows/ci.yml`，引擎同款三平台矩阵 + `check_ip.sh`。
- [x] `docs/decisions/ADR-000-template.md`、`ADR-001`、`docs/progress.md`、`docs/levels.md` 表头。
- [x] 构建通过：`cargo build/clippy -D warnings/test/doc --workspace` 全绿。
- [x] 依赖规则生效：临时给 `rootguard-core` 加 `trellia-gfx` 依赖时 `check_deps.sh` 报
  `rootguard-core -> trellia-gfx 违反 R1` 与传递违规并退出 1；在 core 源码写入 `DrawList` 时报 R2 违规；均已撤销。
- [x] 禁用词检查生效：向 denylist 临时加入一个虚构词并写入 content 源码时 `check_ip.sh` 退出 1；已撤销。

规模：rootguard-core 11 行、rootguard-content 9 行、rootguard-app 约 100 行；测试 1（headless 冒烟）。

遗留：`ip_denylist.txt` 待负责人填写。

## G1 核心规则（2026-10-13 完成）

验收（SPEC.md 17）：

- [x] `rootguard-core` 全部组件 / 资源 / 事件 / 系统（SPEC 5 / 6），14 步 FixedUpdate 顺序由 `battle_tick` 固定。
- [x] `rootguard-content` 加载与 SPEC 4.5 校验（错误含文件路径与字段名，一次报告全部错误）。
- [x] 12 防御者 + 2 弹丸 + 9 敌人 RON、全局规则 `rules.ron`（ADR-002 A3）、临时关卡 1-1。
- [x] SPEC 16.1 全部 19 个测试通过（`cargo test -p rootguard-core --test rules`），测试名与 SPEC 表格一致。
- [x] `every_level_loads_and_validates` 通过（`rootguard-content/tests/load.rs`，另有 4 个校验失败用例）。
- [~] E-2：引擎实现完成（ADR-008、3 个测试、`lane_demo` 改用后 golden 不变），等待推送后 bump rev。

规模：rootguard-core 4597 行、rootguard-content 561 行、rootguard-app 134 行（含测试与文档注释）；
测试 31（core 19 + 4 单元，content 5 + 1 单元，app 1 冒烟 + 1）。

决策：ADR-002（24 条规则细节）。

## G2 可玩对局（2026-10-13 完成，手动试玩待负责人）

验收（SPEC.md 17）：

- [x] `BattleScene` + 占位渲染（SPEC 12.2）+ HUD（SPEC 11.2）+ `UiAction` + 鼠标 / 快捷键翻译（SPEC 11.1 / 11.4）。
- [x] 关卡 1-1 ～ 1-3（`docs/levels.md`，实际威胁与预算逐波一致），三关脚本解 `tests/scripts/*.ron` 回放必胜。
- [x] `level_1_1_scripted_win`（胜利于 level_tick 8061）、`level_1_1_idle_lose`、`placeholder_mode_draws_all_entities`。
- [~] 手动完整玩通 1-3：Xvfb + xdotool 实点验证了“点卡 → 点格放置 → 射击 / 敌人 / Dew 下落”；
  `cargo run -p rootguard-app -- --autoplay tests/scripts/1-3.ron` 在窗口中回放至 VICTORY（与 headless 同 tick）。
  真人试玩与主观记录待负责人。
- [~] E-3：引擎实现完成（ADR-009、1 个测试），等待推送；游戏用按行分层回退（ADR-003 第 4 条）。

测试：core 23、content 6、app 7（battle_flow 3、scripted_levels 1 + 1 录制工具、smoke 1、cli 1）。
决策：ADR-003。

## G3 完整流程（2026-10-13 完成，中文显示待 E-1 合入）

验收（SPEC.md 17）：

- [x] Title / LevelSelect / SeedSelect / Pause / Outcome / Settings 六个场景，导航全部经 `UiAction`（ADR-004）。
- [x] 存档：`progress.ron` 原子写、损坏与版本不符走 `.bak` 备份、`ROOTGUARD_SAVE_DIR` 覆盖。
- [x] i18n：`zh.ron` / `en.ron` 各 83 条、key 完全一致，全部防御者 / 敌人名称与描述齐备（`i18n_tables_complete`）。
- [x] 音效与 BGM：11 个音效 + 3 首循环 BGM，`scripts/gen_audio.py` 程序化合成，已登记 `assets/SOURCES.md`。
- [x] `ui_flow_title_to_battle`、`save_roundtrip`、`audio_events_mapped` 通过。
- [~] E-1 与中文 HUD：引擎实现完成（ADR-010，1 个测试）；字体子集（Noto Sans CJK SC，OFL，166 KB）已提交。
  证据：在 `/tmp` 的仓库副本中以 `cargo build --config 'patch...path=/tmp/trellia/...'` 链接本地引擎分支、
  打开 `font_path` 与 `CJK_FONT_AVAILABLE` 后，Xvfb 截图中标题、按钮、HUD（关卡 / 铲子 / 暂停）、暂停菜单
  中文显示正确（仓库文件未改动）。正式启用需推送引擎分支并 bump rev。
- 窗口实点验证：标题 → 选关 → 1-1 → 暂停按钮 → 暂停菜单 → 设置均正常（Xvfb + xdotool 截图）。
  Xvfb 下无窗口管理器，键盘焦点不可用，Esc 等快捷键只经 headless 测试覆盖。

测试：core 23、content 6、app 13。决策：ADR-004。

## G4 全部关卡（进行中，2026-10-13）

- [x] 1-4 ～ 1-8 的 RON 与脚本解（`every_level_has_scripted_solution` 8 关回放全部胜利，并检查卡组均已解锁）。
- [x] `docs/levels.md` 威胁曲线：每波实际威胁与预算逐一相等（脚本核对）。
- [x] `determinism_golden_1_4`：`tests/golden/level_1_4.snap`（1588 字节，sha256 e3c7a4b1…），debug 与 release 一致。
- [x] devtools 扩展：`RootguardDebugWindow`（F1），Dew +100、下一波、立即胜负、敌人列表；DebugDraw 格子与碰撞区间。
- [x] 1-9：第三次调整（负责人批准）后得到脚本解（33097 tick，用掉 5 个 Hose）。
- [ ] 1-10：第三次调整后仍无脚本解，已再次停下提问（草稿在 `tmp/drafts/`，未入库）。
- [ ] 手动通关全部关卡并记录用时与感受：需真人。

决策：ADR-005。
