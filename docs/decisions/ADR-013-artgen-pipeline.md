# ADR-013：美术管线 `tools/artgen` 的设计与白名单依赖

- 编号：ADR-013
- 标题：美术管线 `tools/artgen` 的设计与白名单依赖
- 日期：2026-10-15
- 状态：已接受

## 背景

ART-SPEC 9 要求一切 `garden` 皮肤资产由 `tools/artgen` 从 SVG 部件与动画脚本离线渲染：纯 Rust、
确定性输出、CI 中重新生成并比对（`art_assets_up_to_date`），并校验颜色、画布与锚点、时长绑定、事件帧
与图集尺寸（9.2）。负责人已批准 `resvg` / `usvg` / `tiny-skia` / `png` 仅供该工具使用（ART-SPEC 12，
AGENT 4.2）。ART-SPEC 未规定部件 / rig / 片段文件的具体格式、颜色如何“只引用调色板”、以及若干细节。

## 决策

1. **crate 位置与依赖**：`tools/artgen`（lib + bin），加入 workspace（`members = ["crates/*", "tools/*"]`），
   使 fmt / clippy / test / doc 与 CI 一并覆盖。直接依赖：`rootguard-core`、`rootguard-content`（读取
   规则数据，与游戏加载与校验完全相同）、`anyhow`、`serde`、`ron`、`resvg`（`default-features = false`，
   不含文字与位图；`usvg` / `tiny-skia` 经 `resvg` 再导出使用，保证版本一致）。`check_deps.sh` 登记 `artgen`
   的允许表，并新增检查：三个游戏 crate 的传递依赖中不得出现 `resvg` / `usvg` / `tiny-skia`。`png` 不在
   该检查内——引擎 `trellia-assets` 本就用它解码图像（引擎白名单）。workspace 为这些渲染 crate 设置
   `profile.dev.package.*.opt-level`，否则 debug 测试过慢。
2. **颜色只来自调色板**：部件中颜色只能写 `fill="@名"` / `stroke="@名"`（或 `none`），`@名.shade` 为
   1.2 的一阶阴影（守卫 / 道具按 HSV 明度 × (1 − 0.18)，Husk × (1 − 0.22)）。加载时拒绝十六进制、颜色名、
   `style`、渐变 / 滤镜 / 蒙版 / 位图 / 文字元素与 `id` / `href`（拼接部件时会冲突）；合成后再以 usvg 树
   检查每个实际颜色（可发现漏写 `fill` 时 SVG 默认的黑色）。允许 `opacity` / `fill-opacity`（ART-SPEC
   只禁止纹理位图、渐变网格与模糊滤镜）。
3. **调色板的两处补充**（`art/palette.ron`）：1.2 材质表规定的 Husk 描边 `#4A3A24` 未列入 1.3 的 14 色表，
   以全局名 `husk_outline` 加入（以 1.2 为准）；v1.2 起 1.3 表另含 `flush`（裁决七），共 16 个全局名；14 色中没有白色，眼白与露珠高光用
   `dew_glow`（1.3 用途“露水光晕”，与“露珠高光”一致），嘴内用 `bark_dark`。专属色逐条注明设计卡出处，
   每角色 ≤ 2 个由工具校验。
4. **文件格式**（`art/README.md` 有完整说明）：部件为完整 SVG，`viewBox` 必须等于 2.1 的画布；
   `rig.ron` 给出类别（决定画布、锚点、材质、所属图集与 1.4 尺寸上限）、部件层级 / 枢轴 / z 序 / 变体、
   受损状态（2.4，以替换部件实现，输出片段名前缀 `<id>_<状态>_<片段>`）与是否生成闪白帧；`clips.ron`
   给出时长（`Ticks(n)` 或 `Rule("…")`）、帧划分（`Every(n)` / `List([...])`）、关键帧（平移 / 旋转 /
   缩放，相对默认位姿，`Smooth` / `Linear` / `Step`）、变体阶跃切换与事件帧。每帧在其起始 tick 取样。
5. **时长绑定与必需绑定**：`Rule` 路径为 `defenders.<id>.<行为>.<字段>` 与 `enemies.<id>.bite_interval_ticks`。
   ART-SPEC 3.1 中由计时器驱动的片段必须绑定对应规则，写固定 tick 或绑定别的规则即拒绝生成：
   `attack` → `shooter.interval_ticks`、`produce` → `producer.interval_ticks`、`arm` → `mine.arm_ticks`、
   `fuse` → `blast` / `gust.fuse_ticks`、`eat` → `bite_interval_ticks`。事件帧必须落在帧起点；`eat` 恰有一个
   `eat_hit` 且在末帧；`attack` 的 `fire` 数等于连发数（9.2 的“恰有一个 fire”按 4.5 双籽荚连发 2 枚推广）。
6. **尺寸校验**：rig 的 `rest` 片段第 0 帧中 alpha ≥ 64 的像素外接框不得超过 1.4 的上限（守卫 92 × 104、
   Husk 128 × 130 等）；地面阴影不透明度 0.22 低于阈值，不计入。
7. **图集**：按类别输出 `garden_defenders` / `garden_husks` / `garden_fx`（2.1；`ui` / `bg` 在 A3 加入）。
   像素完全相同的帧只存一份（idle 各周期的重复帧），货架法确定性排布，帧间留 2 px 透明边，宽度取
   能容纳的最小 2 的幂，任一边超过 4096 即报错。闪白帧 `<帧名>_flash` 为保留 alpha 的纯白剪影（2.3）。
   皮肤清单 `skin.ron` 改为“图集 + 其片段”的列表（游戏侧多图集加载见 ADR-014）。
8. **确定性**：只用有序容器；PNG 由 tiny-skia 固定参数编码。`art_assets_up_to_date`（`tools/artgen/tests`）
   重新构建两次并与提交文件比较：RON 逐字节；PNG 在 x86_64 Linux（生成资产的平台）逐字节，其他平台按
   解码后每通道 ±2 比较——tiny-skia 的 SIMD 路径在不同架构上可能在抗锯齿边缘产生最低位差异。
   文本输出（RON）忽略换行风格比较：Windows 检出时 `core.autocrlf` 会把它们转成 CRLF（首次推送时 Windows CI
   因此失败，本地把副本转成 CRLF 复现后修正；SVG 源文件含 CRLF 不影响渲染结果）。
9. **命令**：`cargo run --release -p artgen -- build | check | preview <id>`；`preview` 把默认位姿与每个
   片段的帧条写到 `tmp/artgen/` 供自查（不入库）。
10. **A1 补充（2026-10-16）**：守卫类别额外输出卡牌头像 `<id>_portrait` 与去饱和 85% 的 `<id>_portrait_off`
    （80 × 100，图集 `garden_ui`，ART-SPEC 2.1 / 4.13）：默认位姿按 rig `portrait` 窗口（缺省按可见范围自动取景）
    直接以矢量缩放渲染，不是位图缩放；费用角标由 HUD 绘制（管线不渲染文字）。`artgen review [--partial]`
    输出 `docs/art/review-A1(-partial).png` 与剪影版：每格为各状态默认位姿 + 头像，序号按 SPEC 7.1 / 8.1 顺序，
    用内置点阵数字绘制，对照表 `docs/art/review-A1.md`。

## 后果

- 新角色只需新增 `art/characters/<id>/` 并运行 `build`；数据中的计时数值变化而片段未跟进时，构建与测试
  都会失败，迫使美术与规则同步。
- 部件不能用 `id` / `use` / `clipPath`，复用形状只能复制路径；换来拼接部件时零冲突与简单的校验。
- 非 x86_64 平台上 PNG 只做近似比较；若将来需要跨平台逐字节一致，可改为关闭 tiny-skia 的 SIMD（需
  在依赖层面替换默认特性）。

## 替代方案

- 扩展 `gen_skins.py`（Pillow）：ART-SPEC 9.2 要求纯 Rust 与 SVG 部件，否决。
- 颜色直接写十六进制、事后比对调色板：阴影色需手算，且写错时只报“颜色不对”而非名字，否决。
- 部件按 rig 嵌套 `<g>` 输出：绘制顺序会被层级绑死（如后臂在身体之后、却跟随身体），改为每个部件独立
  计算世界矩阵、按 z 平铺。
- 图集按角色各一张：张数多、加载与绘制切换图集更频繁，且不符合 2.1 的五张图集，否决。
