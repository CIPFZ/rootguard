# ADR-009：引擎补充 E-1 / E-2 / E-3 / E-5 的落地

- 编号：ADR-009
- 标题：引擎补充 E-1 / E-2 / E-3 / E-5 的落地
- 日期：2026-10-14
- 状态：已接受；取代 ADR-003 第 4 条、ADR-004 第 5 条中“E-5 留待 G5”、第 6 条与第 8 条中的回退部分

## 背景

引擎 E-1 / E-2 / E-3 / E-5 已合入 CIPFZ/trellia main（24c4d71，引擎 ADR-008 ～ 011），游戏 `rev` 已同步。
此前的回退方案（英文显示、按行分层、手写资产加载样板、BGM 硬切）需要按 AGENT 3.6 移除。

## 决策

1. **E-1**：`main.rs` 以 `RunConfig.font_path = cjk_font_path()` 加载 `assets/fonts/RootguardCJK-Regular.otf`
   （资产根目录与游戏资产相同；文件缺失时警告并不加载）。删除 `CJK_FONT_AVAILABLE` 与 `I18n::display_lang`，
   设置为中文时直接显示中文。英文表中的语言名仍为 “Chinese”（无需改动）。
2. **E-3**：对局角色全部画在 `Layer::WORLD`，同一行的形状、文字与精灵共用行底 y（`Grid::sort_y(row)`）作为
   `sort_y`，行内按提交顺序——与原“每行一个图层”的绘制顺序等价。HUD / Dew / 覆盖层图层不变。
3. **E-2**：皮肤与音频加载改用 `load_atlas_into` / `load_clip_into` / `load_audio_into`。
4. **E-5**：`sfx::play_music` 首次播放（此前无 BGM）用 `PlayMusic`，之后的场景切换用
   `CrossfadeMusic { ticks: 60 }`。保留首次 `PlayMusic` 是因为 SPEC 16.2 `audio_events_mapped` 字面要求日志含
   `PlayMusic`；该测试相应断言“标题 1 次 PlayMusic + 对局 / 结算交叉淡化”。

## 后果

- 新增测试 `chinese_hud_when_lang_zh`、`rows_occlude_by_sort_y`；golden 不变。
- 中文显示的实机效果已在 Xvfb 截图中核对；交叉淡化的听感未经人耳确认（本环境无音频设备）。

## 替代方案

- （4）全部切换都用 `CrossfadeMusic`（首次即淡入）：与 SPEC 16.2 的字面断言冲突，否决。
- （2）用实体 y 而不是行底 y 作为 `sort_y`：同行内的前后关系会依赖角色的 y 偏移，与占位渲染的约定不同，否决。
