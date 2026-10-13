# ADR-004：G3 场景流程、存档、音频与文本的实现细节

- 编号：ADR-004
- 标题：G3 场景流程、存档、音频与文本的实现细节
- 日期：2026-10-13
- 状态：已接受

## 背景

实现 SPEC.md 3.4 / 11.1 / 11.3 / 11.6 / 13 / 14 时遇到以下未写清之处。

## 决策

1. **导航集中在 `flow::ui_router_system`**（`UiPlugin`，`Update`，先于场景 `update`）：唯一消费场景类
   `UiAction`（StartLevel / ConfirmSeeds / Restart / NextLevel / BackTo* / PauseToggle / Set*）。
   主场景之间切换用 `SceneCommands::clear + push`（Title、LevelSelect、SeedSelect、Battle 互相 replace），
   Pause / Outcome / Settings 为 push。会话资源 `Session { level, cards }` 支撑“重试 / 下一关”。
   设置场景的“返回”与选卡场景内的增删卡不在 SPEC 11.1 的 `UiAction` 列表中，作为场景内部状态 / 命令处理。
2. **关卡解锁**：冒险顺序第一关始终可进；其余关在上一关通关后解锁（SPEC 1.3 线性解锁）。
   `StartLevel` 对未解锁关卡忽略并警告。
3. **初始卡牌**写在 `rules.ron` 的 `starting_cards`（SPEC 7.1“初始”），新存档据此解锁。
4. **音效事件的消费**：引擎事件在两帧内可见，`iter()` 会重复处理。`SfxPlugin` 在 `Update` 中
   **取走（drain）** SPEC 13 表中的全部事件，是它们唯一的消费者；“同一 tick 同一音效最多 2 次”
   以“每帧上限 = 2 × 本帧 tick 数”实现。`BattleEnded` 只用于音效，存档与结算由 `BattleScene` 检测
   `phase` 完成（语义同 SPEC 14“写入时机 BattleEnded(Won)”）。
5. **BGM**：Title / LevelSelect / SeedSelect 用标题曲，Battle 用对局曲，Outcome 用结算曲；
   Pause / Settings 不换曲；与当前曲目相同时不重新开始。E-5 交叉淡化留待 G5。
6. **音频加载**未使用 E-2（尚未进入游戏所用 rev）：音频只需一个 `Assets<AudioClip>`，直接
   `server.load_audio` + `add_with_path`，不涉及多资源同时借用。
7. **存档位置可注入**：`SaveLocation::{Default, Dir}`，`build_app_full` 供测试使用临时目录，
   避免测试读写用户 data 目录；`ROOTGUARD_SAVE_DIR` 仍按 SPEC 14 覆盖默认目录。
   “新游戏”重置进度但保留设置。
8. **文本**：`I18n` 资源持有两张表，`t(world, key)` 按设置语言取值，缺失 key 返回 key 并警告一次
   （内部 `Mutex<BTreeSet>`，非全局状态）。中文显示依赖 E-1；在游戏所用的引擎 rev 支持
   `RunConfig.font_path` 前，`I18n::display_lang` 把中文回退为英文（SPEC 11.6），常量
   `CJK_FONT_AVAILABLE` 在 bump rev 时改为 true，并在 `main.rs` 设置 `font_path`。
   英文表中的语言名使用 “Chinese”（避免英文模式下出现缺字方框）。
9. **CJK 字体**为 Noto Sans CJK SC（OFL 1.1）子集，由 `scripts/subset_font.py` 按文本表字符集生成并更名，
   许可证全文随字体提交（`assets/fonts/LICENSE-OFL.txt`）。
10. **对局被覆盖时**（暂停 / 设置）`BattleScene` 通过 `on_pause / on_resume` 停用 HUD 按钮与鼠标翻译。
11. **设置场景遮住下方场景**（`blocks_draw_below = true`），避免下方 egui 按钮可点。

## 后果

- 所有流程可由 `UiAction` 在 headless 中驱动（`ui_flow_title_to_battle`、`save_roundtrip`、`audio_events_mapped`）。
- E-1 合入后的切换只涉及 `CJK_FONT_AVAILABLE` 与 `main.rs` 一行。

## 替代方案

- （1）各场景各自处理导航动作：重复代码且覆盖层下的场景收不到动作，否决。
- （4）给事件加 tick 戳去重：需要改 core 事件结构，收益小，否决。
- （8）游戏层直接调用 `egui::Context::set_fonts`：绕开 SPEC 11.6“E-1 落地前以英文运行”的约定，否决。
