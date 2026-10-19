# ADR-018：默认渲染模式改为 Skinned + garden，存档 v2 迁移

- 编号：ADR-018
- 标题：默认渲染模式改为 Skinned + garden，存档 v2 迁移
- 日期：2026-10-19
- 状态：已接受（裁决十，SPEC v1.3 12.1）

## 背景

v0.2.0 的 `SkinMode` 默认仍是 `Placeholder`（SPEC v1.2 12.1），新装与旧存档都停在占位几何体，玩家要进设置才能看到
garden 皮肤。

## 决策

1. `SkinMode::default()` = `Skinned`，`SkinName::default()` = `Garden`；`Settings::default()` 随之改变。
2. `Progress::VERSION` = 2。`save::load`：v2 照常读取；v1 经 `Progress::migrate_v1`（`skin` = `Skinned`、`skin_name` =
   `Garden`、版本改为 2，其余不变），视为有效存档，下次写入时以 v2 保存；其他版本与损坏文件仍备份为 `.bak`。
3. 测试：显式指定模式的测试不改；依赖默认值的 4 个（`placeholder_mode_draws_all_entities`、`rows_occlude_by_sort_y`、
   `time_of_day_tint`、`title_background_and_ending_still` 的占位部分）显式切到 Placeholder。新增
   `save_v1_migrates_to_skinned_garden`、`menu_buttons_are_wooden_signs`。golden 不变（渲染模式不进模拟）。
4. 设置画面选项顺序为 花园 / 几何（0.1.0）/ 占位，按钮加宽到 190 px 容纳“Geometric (0.1.0)”。
5. 标题与结算的菜单按钮：皮肤有 `ui_menu_button` 时在 `DrawList` 中画木牌（`scenes::draw_wood_buttons`，各场景的
   `draw` 与 `ui` 用同一组按钮矩形），egui 按钮无边框（`scenes::menu_button`）。

## 后果

- 0.1.0 / 0.2.0 中主动选过占位模式的玩家也会被切到花园（裁决十：0.1.0 没有理由主动选占位）；v2 起的选择保留。
- CI 截图与 headless 测试的基准仍是 Placeholder，由测试显式设置。
