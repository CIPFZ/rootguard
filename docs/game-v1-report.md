# Rootguard Phase 1 实现报告（game v1）

> 日期：2026-10-14 ｜ 规格：`SPEC.md` v1.1 ｜ 引擎：CIPFZ/trellia `24c4d712ff3c157dad12ac7d50836476e699b8fa`
> 进度与逐条验收证据见 `docs/progress.md`，关卡数据见 `docs/levels.md`，实现决策见 `docs/decisions/ADR-001 ～ 009`。

## 1. 结论

SPEC 第 17 章 G0 ～ G5 的全部自动化验收已通过；需要真人完成的项目（手动试玩 10 关、主观难度、
实机性能读数与音频听感）以及 `scripts/ip_denylist.txt` 的填写由负责人负责，见第 6 节。

| 里程碑 | 状态 | 关键证据 |
|---|---|---|
| G0 骨架 | ✅ | `check_deps.sh` / `check_ip.sh`、三平台 CI |
| G1 核心规则 | ✅ | SPEC 16.1 全部 19 项规则测试 |
| G2 可玩对局 | ✅（手动试玩待负责人） | `level_1_1_scripted_win / idle_lose`、`placeholder_mode_draws_all_entities` |
| G3 完整流程 | ✅ | `ui_flow_title_to_battle`、`save_roundtrip`、`audio_events_mapped`、`chinese_hud_when_lang_zh` |
| G4 全部关卡 | ✅（手动试玩待负责人） | `every_level_has_scripted_solution`、`scripted_solution_uses_at_most_one_hose`、`determinism_golden_1_4` |
| G5 皮肤与打磨 | ✅ | `skin_switching_keeps_golden_1_4`、`tests/visual_fx.rs`、`examples/perf_1_10`、`scripts/package.sh` |

## 2. 规模与测试

| crate | 行数（含测试与文档注释） | 测试 |
|---|---|---|
| `rootguard-core` | 4652 | 23（规则 19 + 单元 4） |
| `rootguard-content` | 577 | 6 |
| `rootguard-app` | 5718 | 22 |
| 合计 | 10947 | **51**（另有 2 个 `#[ignore]` 工具：`record_scripts`、`search_plans`） |

SPEC 第 16 章列出的 29 个测试全部存在且同名；`tests/golden/level_1_4.snap` 自 G4 生成后未变
（sha256 e3c7a4b1…），且在对局中反复切换渲染模式后仍逐字节一致。

## 3. 引擎补充 E-1 ～ E-5

| 编号 | 状态 | 引擎 | 游戏侧 |
|---|---|---|---|
| E-1 CJK 字体 | ✅ 已合入 | `RunConfig.font_path`（引擎 ADR-010） | 中文 HUD；字体为 Noto Sans CJK SC 子集（OFL） |
| E-2 资产加载辅助 | ✅ 已合入 | `load_atlas_into` / `load_clip_into` / `load_audio_into`（ADR-008） | 皮肤与音频加载 |
| E-3 形状 `sort_y` | ✅ 已合入 | `*_sorted` 绘制命令（ADR-009） | 行遮挡，删除按行分层 |
| E-4 headless egui 指针注入 | 未实现（按需项） | — | 全部 UI 流程以 `UiAction` 覆盖，无需求 |
| E-5 音乐交叉淡化 | ✅ 已合入 | `AudioRequest::CrossfadeMusic`（ADR-011） | 场景切换 BGM 淡化 60 tick |

四项补充在引擎 main 4eb6c29 之上依次提交（aff9a7b → df4e78a → 6ba59c7 → 24c4d71），引擎 CI 三平台通过
（run 38521747226）；游戏 CI 三平台检查与打包通过（run 38523777097）；
游戏落地记录见 ADR-009。

## 4. 内容

- 12 防御者、8 敌人 + 1 衍生、10 关，数值全部在 `assets/data/**/*.ron`。
- 关卡按 SPEC 10.2 威胁预算生成（`scripts/gen_levels.py`），v1.1 预算上限：1-8 / 1-10 第 1 档、1-9 第 2 档；
  每关脚本解回放必胜且 Hose ≤ 1（10.3）。
- 原创美术（`scripts/gen_skins.py`，247 帧 / 46 片段）与音频（`scripts/gen_audio.py`，11 音效 + 3 BGM），
  全部程序化生成并登记 `assets/SOURCES.md`；造型约定见 ADR-007。

## 5. 性能（SPEC 16.3）

release headless CPU 帧耗时（模拟 + DrawList 生成），AMD EPYC 9K65：SPEC 假设负载（43 敌人 / 45 防御者 / 94 弹丸）
下 p99 约 100 µs、最大 1.4 ms，两种渲染模式均远低于 4 ms。测量方法与原始数字见 ADR-008 与 `docs/progress.md`。

## 6. 已知限制与待负责人事项

1. **手动试玩未做**：10 关的手动用时与主观难度为空。1-9 / 1-10 的机器人合格率很低（4 / 1536、16 / 1536），
   真人难度可能偏高；1-3 的脚本解按 10.3 重录（ADR-006），关卡数据未改。
2. **禁用词表为空**：`check_ip.sh` 目前跳过匹配，需负责人填写 `scripts/ip_denylist.txt`。
3. **实机验证有限**：Linux 只在 Xvfb 中运行；macOS / Windows 的安装包只由 CI 构建，未在实机启动；
   音频（含交叉淡化）的听感未经人耳确认（本环境无音频设备）。
4. **性能读数口径**：devtools 面板的 “frame ms” 是帧间隔（垂直同步下约 16.7 ms），不反映负载；GPU / egui
   绘制耗时未测。
5. **皮肤为简单几何造型**：每角色 ≤ 6 个形状、96 px 帧；`eat_hit` 事件帧目前只作为数据存在，无消费者。
6. **呈现层启发式**：Dew 收集飞行动画以“实体消失且未到期”判定（ADR-008）。
7. **项目许可证未定**：仓库与安装包不含项目许可证文件（素材许可证随 `assets/` 分发）。
8. **机器人战术**：录制机器人对咬夹草等战术单位的运用较弱，只影响脚本解搜索，不影响游戏。

## 7. Phase 2 建议

- 内容按 SPEC 第 18 章（夜晚、水域、屋顶、Boss、生存 / 无尽等），需新的游戏 spec 版本。
- 关卡工具：把 `search_plans` 的合格率作为难度指标写入 `docs/levels.md`，并在真人试玩后校准预算公式与档位。
- 引擎：E-4（headless 注入指针事件）以便测试 egui 布局；`AudioBackend` 若需要更多时间性效果可考虑逐帧回调；
  `Sprite` 着色目前只能变暗，可考虑加性着色以支持闪白等效果而无需额外帧。
- 呈现：消费 `eat_hit` 事件（啃食音效与动画同步）、更丰富的皮肤（更大分辨率、更多帧）、CI 中的截图回归。
- 发布：确定项目许可证；macOS `.app` 包与图标、Windows 图标资源。
