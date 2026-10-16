# Rootguard 开发进度

## 会话记录（新记录追加在最上方）

- 2026-10-16 会话 8：执行裁决（七）`2026-10-16-a1-review.md`（守卫通过；Husk 去鸟化后进 A2）与 ART-SPEC v1.2。
  先做 Husk 去鸟化并重出审核表（不停下），然后完成 A2 全部七类（每类一张帧条），实机 1-9 自查与性能测量。
  **停下**，等负责人在底图上看一局 1-9。记录见“美术阶段 → A1 / A2”。

- 2026-10-16 会话 7：执行裁决（六）`2026-10-16-a1-go.md`：枯壳怪通过并定为 8 个 Husk 的模板，A1 放行。先查“错位
  白色剪影”：不是闪白帧问题，是重叠敌人的绘制顺序（ADR-014 后续条目），已修。随后按“11 守卫 → 8 Husk”完成 A1 的
  19 个角色，每 4 个更新一次中间审核表，最后输出 `review-A1.png` 与剪影版。**停下等 A1 审核**。

- 2026-10-16 会话 6：执行负责人裁决 `2026-10-16-a0-review.md`（五）与 ART-SPEC v1.1（负责人已入库 f07056e）：
  石籽荚通过并作为守卫模板；枯壳怪按 v1.1 1.2 / 1.5 / 5.1 重做头部与姿势，重拍实机截图。**停下**，等负责人看过
  重做后的枯壳怪再开始 A1。记录见“美术阶段 → A0 审核（裁决五）”。

- 2026-10-15 会话 5：**接手（美术阶段）**，上下文重建自仓库文件（AGENT v1.3、SPEC v1.2、LORE、ART-SPEC v1、
  负责人裁决四份、ADR-001 ～ 012、`docs/art/v3-*.png`）。起点：`cargo build/test --workspace` 绿色，54 个测试。
  发现裁决（四）（Chill 下啃食相位）已入库但未落地，而 A0 第 4 项以它为范式，先落地（ADR-012 后续条目）：
  测试 55 个全绿，golden 未变。随后开始 A0，记录见下方“美术阶段 A0 ～ A3”。

- 2026-10-15 会话 4：执行负责人裁决 `2026-10-15-bite-rhythm.md`（负责人已入库）。eat 片段对齐啃食间隔（ADR-012），
  `subset_font.py` 注释补充 venv 重建说明；CHANGELOG 日期负责人已在 7f3049a 填写（2026-10-14），未再改动；
  禁用词表负责人已填（58 词），`check_ip.sh` 零命中。测试 54 个全绿。回到等待试玩反馈。
- 2026-10-14 会话 3：接手（收尾期），上下文重建自仓库文件（SPEC v1.2、AGENT v1.2、负责人裁决两份、
  `docs/game-v1-report.md`、ADR-001 ～ 009）。起点：`cargo build/test --workspace` 绿色，51 个测试。
  负责人裁决 `2026-10-14-g5-closure.md` 原在仓库根目录（文件名带前导空格）且未入库，移至 `docs/owner-decisions/`
  并与 SPEC / AGENT v1.2 一并提交。执行 AGENT.md 第 9 章收尾清单，记录见下方“收尾期”章节。
- 2026-10-14 会话 2（续）：负责人授权实现者直接推送 GitHub。引擎补丁 0001 ～ 0004 应用到 `/data/workspace/trellia`
  main 并推送（4eb6c29 → 24c4d71，引擎 CI 三平台通过）；游戏 bump rev 后依次落地 E-1（中文显示）、E-3（`sort_y`）、
  E-2（加载辅助）、E-5（BGM 交叉淡化）（ADR-009），撰写 `docs/game-v1-report.md`，**G5 关闭**。测试 51 个全绿。
- 2026-10-14 会话 2：接手，上下文重建自仓库文件（SPEC v1.1、AGENT v1.1、负责人裁决、ADR-001 ～ 005、
  引擎报告第 3 ～ 4 节）。起点：`cargo build/test --workspace` 绿色，41 个测试。负责人裁决文件原在仓库根目录
  且未入库，按 AGENT.md 移至 `docs/owner-decisions/2026-10-14-g4-levels.md` 并与 SPEC / AGENT v1.1 一并提交。
  执行 AGENT.md 第 9 章 G4 返工清单并关闭 G4（手动试玩待负责人）：新增 `scripted_solution_uses_at_most_one_hose`，
  `gen_levels.py --tier/--cap/--flag-cap`，参数搜索工具 `search_plans`；1-7 / 1-8 / 1-10 第 1 档、1-9 第 2 档重做，
  另发现 1-3 原脚本解 Hose 2 并重录（ADR-006）。测试 42 个全绿，golden 未变。
  随后进入 G5：皮肤模式与原创图集（ADR-007）、Dew 飞行 / 横幅动画、打包脚本、性能测量（ADR-008）；
  E-5 在引擎分支实现完成（补丁 0004），**按 AGENT 3.6 停下，等待负责人推送合并引擎 E-1 ～ E-5 并回传 sha**。
  测试 49 个全绿。
- 2026-10-14 会话 1（续）：负责人批准对 1-9 / 1-10 做第三次波次调整；1-9 可解并提交，1-10 仍不可解，再次停下提问。
- 2026-10-13 会话 1：通读 SPEC.md / AGENT.md、引擎 v1 报告、ADR-001 ～ 007 与 `examples/lane_demo`；
  计划完成 G0 骨架并开始 G1。

## 给项目负责人的待办

- [x] ~~**打 `v0.1.0` tag 前**：把 `CHANGELOG.md` 中 `## [0.1.0] - Unreleased` 改为实际日期~~（负责人 7f3049a 已填 2026-10-14），并按需在“已知限制”中
  更新实机验证结果（Release 说明原样取自该条目，ADR-011）；tag 必须与 `Cargo.toml` 的 `0.1.0` 一致，否则 Release 作业失败。
- [ ] 实机验证（macOS / Windows 启动、音频与交叉淡化听感、新增啃食音效 `sfx.bite` 的密度）结果写入本文件。
- [x] ~~**决定**：Chill 状态下啃食音效是伤害频率的 2 倍~~：负责人裁决（四）采用 `Biter` 相位驱动，2026-10-15 落地（ADR-012 后续条目）。
- [x] ~~【阻塞 G4】1-10 无法写出脚本解~~：负责人裁决 2026-10-14（SPEC v1.1 预算上限阶梯），已按第 1 档解决。
- [ ] **手动试玩 10 关**并填 `docs/levels.md` 的“手动用时 / 主观难度”列。建议重点关注 1-9（第 2 档，搜索合格率
  4 / 1536）与 1-10（第 1 档，合格率 16 / 1536）：机器人的合格解很少，真人难度可能偏高。
- [ ] **知悉**：1-3 不在裁决列出的返工关卡中，但其原脚本解用掉 2 个 Hose，违反 SPEC 10.3；已只重录脚本解、
  关卡数据未改（ADR-006）。如负责人认为 1-3 应完全不动，请告知。
- [x] ~~填写 `scripts/ip_denylist.txt`~~：负责人 ff2dfa7 已填（58 词），`check_ip.sh` 对 110 个文件零命中。
- [x] ~~E-1 / E-2 / E-3 / E-5 引擎补充需推送~~：2026-10-14 已由实现者推送至引擎 main 24c4d71 并在游戏中落地（ADR-009）。原记录：实现已在引擎本地分支链 `feat/e2-world-asset-helpers`（56082a4）→
  `feat/e3-sorted-shapes`（8484ce3）→ `feat/e1-cjk-font`（76be022）→ `feat/e5-music-crossfade`（0069b02，2026-10-14 新增，ADR-011）
  （补丁副本 `tmp/engine-patches/0001～0004`，未入库，已验证可按序 `git am` 到引擎 main 4eb6c29）完成并通过引擎全部检查（108 个测试）；
  本环境无引擎仓库推送权限与 `gh`，需负责人推送 / 合并后回传 sha，游戏仓库再 `chore(deps): bump trellia to <sha> (E-1 E-2 E-3 E-5)`。
  已在 `/tmp` 的仓库副本中以 `--config patch` 链接该分支链验证：游戏 49 个测试（含 golden）全部通过。
  在此之前游戏按 SPEC 12.2 的回退方案按行分层（不依赖 E-3），界面按 SPEC 11.6 以英文显示（不依赖 E-1）。
  bump 后：`i18n::CJK_FONT_AVAILABLE = true`，`main.rs` 设置 `font_path: Some("assets/fonts/RootguardCJK-Regular.otf")`。
- [ ] **手动试玩 1-1 ～ 1-3**（G2 验收“手动完整玩通 1-3”）：实现者只能在 Xvfb 中用 xdotool 点击验证鼠标通路、
  用 `--autoplay` 在窗口中回放脚本解（1-3 胜利，截图证据），无法代替真人试玩与主观难度记录。
- [x] 确认 Sprint 阈值：负责人裁决 2026-10-14 采用“≤ 50%”，ADR-002 第 14 条已标记确认。
- [ ] 仓库原有目录 `assests/icon/` 疑为笔误，已移动为 SPEC / AGENT / README 所述的 `assets/icon/`（文件未改动）。

## 美术阶段 A0 ～ A3（ART-SPEC v1，2026-10-15 起）

任务顺序见 AGENT.md 第 9 章；规格见 `ART-SPEC.md` 第 11 ～ 12 章。A1 是审核门（负责人批准 `review-A1.png` 前不开始 A2）。
难度校准（SPEC 20.2）仍等待 `docs/levels.md` 主观难度列，与 A 阶段并行、分开提交。

| 里程碑 | 状态 |
|---|---|
| A0 管线 | ✅ 2026-10-15 |
| A1 全员静态（审核门） | ✅ 2026-10-16（裁决七：守卫通过、Husk 去鸟化后放行） |
| A2 全员动画与特效 | ✅ 2026-10-16（裁决八通过） |
| A3 战场与叙事 | 进行中 |

### A2 全员动画与特效（裁决七放行，2026-10-16）

每完成一类（守卫 attack / produce / fuse、Husk walk + eat、护甲破碎、特效、音效）输出一张实机帧条到
`docs/screenshots/`（战场底图上），提交后继续；A2 全部完成后停下，负责人在底图上看一局 1-9。

- [x] 前置 1：战场底图 `bg_base.png`（ART-SPEC v1.2 8.1 L0 ～ L2，ADR-015）：天空渐变与草地格由 `artgen` 按设计卡
  公式从调色板生成（几何取自游戏 `Grid`），远景 `art/bg/far.svg`（沉睡的根母、枯玉米地、云）；调色板新增 `background`
  段（`sky_top` / `sky_bottom` / `grass_dry` / `brass`）。游戏侧花园皮肤画底图 + 非激活车道 40% 遮罩
  （`garden_background_drawn_with_lane_mask`）；温室 / 入口 / Hose 标记仍是占位（A3）。
  **待负责人确认**：8.1 的“格底色与角色主色明度差 ≥ 25”按设计卡配色不可满足（格 L* 62 ～ 77，21 个主色中 20 个
  不达标，见生成的 `docs/art/bg-readability.md`）；现以“描边色与格底色 ≥ 25”为强制项（39.2 / 36.6，满足），
  主色只报告。详见 ADR-015 第 3 条。
- [x] 前置 2：弹丸 `pebble`（灰色小石子、微旋转）、`frost_pebble`（冰蓝冰石 + 3 帧白雾尾）与 Dew（金色水滴 + 呼吸
  光晕；下落时左右摆、过期前 180 tick 闪烁，ART-SPEC 6），图集 `garden_fx`。
- [x] 第 1 类 守卫 attack / 战斗类（2026-10-16，ADR-016 第 1 节）：霜籽荚 idle（发抖）/ attack（O 嘴）；双籽荚 idle
  （两荚反相呼吸、互瞪）/ attack（前荚 0、后荚 6 = `volley_gap_ticks` 发射，`artgen` 校验）；咬夹草拆成上下荚瓣，
  idle / snap（张嘴 44° ≤ 45°，gulp 在 11）/ chew；荆棘垫 idle（打盹）/ trigger（绑定 `spikes.interval_ticks`，prick 在 0）；
  树皮墙 idle（偶尔瞟眼）。游戏侧：snap / chew / trigger 的状态选择、被啃 2 px 抖动。帧条（逐帧合成在战场底图上，
  `artgen strip`）：`docs/screenshots/a2-attack.png`。测试 `chomper_and_spikes_clips_follow_state`。
- [x] 第 2 类 produce（2026-10-16，ADR-016 第 2 节）：生产者改为两层绘制（身体 idle 按时钟 + produce 叠层按生产
  计时器）。露杯：露珠 12 → 28 px 并越来越亮、末 60 tick 光晕脉冲、pop 后打哈欠；日灯笼：三盏随进度依次点亮、末 60
  tick 同时爆亮、各自轻摆，第 4 次生产（`bonus_every`）换金色 `produce_gold`。帧条 `docs/screenshots/a2-produce.png`
  （`底层+叠层` 合成）。测试 `producer_overlay_follows_timer`。
- [x] 第 3 类 fuse（2026-10-16，ADR-016 第 3 节）：炽球 fuse（越来越亮 / 鼓到 1.15 / 抖动递增 / 怒 → 闭眼憋气）、
  风铃花 fuse（花瓣逐帧加速旋转、腮越鼓越大，钟口单独不转）、块茎雷 arm（随进度从土里露出、芽长出）/ armed（芽尖
  每 30 tick 一闪）；引信芽加高 30% 多卷半圈（裁决七）。帧条 `docs/screenshots/a2-fuse.png`。测试
  `fuse_and_arm_clips_follow_timers`。
  提交纪律记录：本类提交 a1f73c3 时我没看测试结果就提交推送（`skin_garden_loads` 的回退示例因 garden 新增
  `tuberblast_armed` 失效），随后 919e243 修复；之后每次提交命令都在测试失败时中止。
- [x] 第 4 类 Husk walk + eat（2026-10-16，ADR-016 第 4 节）：9 个 Husk 的 walk / eat（全部状态各一套）、疾行枯壳
  sprint、跳栏枯壳 vault（绑定数据中的 40 tick；ART-SPEC 写 30）、小芽怪 spawn；陶盆走路晃、门板随步上下且啃食时侧开、
  背篓一颠、鼓腹每步一沉（thud 事件）、小芽怪跳起来咬。游戏侧：walk / sprint 由走过的距离驱动（脚不打滑、Chill 自然
  减半），sprint / vault / spawn 的状态选择，`vaulted_` / `cracked_` / `bare_` 前缀。帧条
  `docs/screenshots/a2-husk-walk-eat.png`。测试 `husk_clips_follow_movement_and_state`。
- [x] 第 5 类 护甲破碎与受损阶段（2026-10-16，ADR-016 第 5 节）：特效实体管线（`fx.rs`，先于 sfx 捕获事件、双缓冲去重、
  按 tick 播放）；`armor_break_twig / pot / plank / gourd`（`art/fx/`，`artgen` 新增 Fx 类别与不透明度关键帧）；树皮墙 /
  葫芦壳三级受损接入游戏、葫芦壳闪白抖动。帧条 `docs/screenshots/a2-armor.png`。测试 `armor_break_spawns_effects`、
  `defender_damage_states`。**发现**：10 关脚本解中门板（Shield）从未被打碎，门板碎片特效在实际对局里少见。
- [x] 第 6 类 特效（2026-10-16，ADR-016 第 6 节）：ART-SPEC 7 全表（hit_puff / hit_frost / crumble / sprout_fold / wilt /
  plant_dust / tuber_burst / ember_wave / gust_ring / dew_pop / hose_burst / frost_crown）+ 口部小尘 / 白气，按事件生成；
  Chill 霜晶；放置弹性缩放与铲除残影（4.13）。帧条 `docs/screenshots/a2-fx.png`。测试 `battle_effects_follow_events`、
  `constructed_effects`。未做：咬夹草叶齿残影与咀嚼结束气泡（无对应事件）。
- [x] 第 7 类 音效（2026-10-16，ADR-016 第 7 节）：ART-SPEC 12 的 20 个新音效（`gen_audio.py`，登记 `SOURCES.md`，SPEC 13
  补行）；模拟事件 + 片段事件帧（gulp / thud / prick / vault，固定取 garden 片段、不随设置变化）。波形图
  `docs/screenshots/a2-sfx.png`。测试 `art_phase_sfx_mapped`、`constructed_sfx_mapped`。听感待负责人实机确认。
- [x] 第 11 步 自查（2026-10-16）：Xvfb 实机 1-9（花园皮肤、战场底图）`docs/screenshots/a2-1-9.png`（第 380 s）与
  `a2-1-9-strip.png`（第 90 / 200 / 300 / 440 s）：同屏约 20 个单位时角色、护甲状态、霜晶与特效可分辨，特效噪音适中；
  受伤闪白（纯白剪影 6 tick）在大块守卫上较显眼，属 ART-SPEC 2.3 规定。性能（`perf_1_10`，新增 `--garden`）：

  | 场景 | 模式 | p50 | p99 | 最大 |
  |---|---|---|---|---|
  | 1-10 脚本解（全程） | Placeholder | 84 µs | 148 µs | 929 µs |
  | 1-10 脚本解（全程） | Skinned（garden） | 118 µs | 220 µs | 1.03 ms |
  | 压力场景 | Placeholder | 160 µs | 196 µs | 277 µs |
  | 压力场景 | Skinned（garden） | 227 µs | 284 µs | 353 µs |

  均远低于 4 ms（SPEC 16.3）。占位模式比 G5 慢约一倍：片段事件帧（音效）对每个角色每帧计算一次姿态，与渲染模式无关。
- **A2 完成**。资产：artgen: 42 个角色，316 个部件文件，119 个片段，1538 帧（去重后 987 张图）；garden_defenders.png 4096 × 1560；garden_fx.png 2048 × 1156；garden_husks.png 4096 × 2592；garden_ui.png 512 × 408；garden_bg.png 1280 × 720。测试 89 个全绿，golden 未变。
- CI：bd9990f ～ 0c8432d 的 macOS / Windows 运行失败（`art_assets_up_to_date`：`garden_husks.png`、`garden_fx.png` 像素差超过
  ±2；Linux 通过）。CI 增加失败测试的错误注解后定位，改为非参考平台允许 0.1% 离群通道（ADR-013 第 8 条修订）；
  另有 0c8432d 一次 fmt 漏检（命令链用 `;` 断了门控），已在 28f40cc 修正。
  CI run 38774747294（9039660）三平台全部通过。
- **停下**（裁决七第 4 节）：等负责人在底图上看一局 1-9。待确认事项：ADR-015 第 3 条（8.1 主色明度约束按设计卡配色不可满足，
  现以描边为强制项）；SPEC 13 按 ART-SPEC 12 补了 20 行音效（SPEC 在 AGENT 中标为只读，按 sfx.bite 的先例处理）；
  门板碎片特效在 10 关脚本解中从未出现；咬夹草叶齿残影 / 咀嚼气泡未做；音效听感未经人耳确认。

### A3（裁决八 `docs/owner-decisions/2026-10-17-a2-review.md`，2026-10-16 起）

- [x] A2 审核通过；ADR-015 第 3 条标记已确认；ADR-013 补“容差只用于渲染比对，golden 三平台逐字节”；提交前检查改为
  `scripts/precommit.sh`（任一步失败即退出，提交命令接在其后用 `&&`）。
- [x] 第 1 项 闪白 70% 混白（ART-SPEC v1.3 2.3）：`artgen` 对同一位姿重新合成，描边以外的颜色向白混 70%；全部角色重新生成。

### A1 全员静态（裁决六放行，2026-10-16）

顺序：11 个守卫（模板石籽荚）→ 8 个 Husk（模板枯壳怪）；每 4 个输出 `docs/art/review-A1-partial.png` 并提交，
不停下；19 个完成后输出 `review-A1.png` 与 `review-A1-silhouette.png`，停下。

- [x] 裁决（六）第 2 节“错位白色剪影”：`artgen` 的 176 个帧中，闪白帧与本体帧尺寸、锚点相同，alpha 逐像素相同；
  headless 中每个敌人只提交一个精灵、落点一致。原因是同一行重叠的枯壳怪中被命中的最左一只整只变白、却被
  按实体 id 后画的同伴盖住只露白边。改为同一行从右到左提交（`overlapping_enemies_draw_leftmost_on_top`，先红）。
  `artgen` 无需改动，资产不变。测试 77 个全绿。

- [x] 工具：`artgen` 卡牌头像（80 × 100 + 去饱和“不可用”版，`garden_ui` 图集）与 `artgen review [--partial]`
  审核表命令（ADR-013 第 10 条）；对照表与特征出处 `docs/art/review-A1.md`。A1 每个角色只有静态 `idle` / `walk`
  默认位姿，动画在 A2；花园皮肤中这些角色在实机里静止显示。
- [x] 第 1 批（2026-10-16）：露杯、树皮墙（3 级受损）、霜籽荚、双籽荚 → `review-A1-partial.png`。
  过程中尺寸校验拦下双籽荚宽 95 > 92，已收窄；双籽荚初稿后荚被前荚挡住、悲观表情看不出，已分开。
- [x] 第 2 批（2026-10-16）：荆棘垫、块茎雷、炽球、葫芦壳（3 级受损）→ `review-A1-partial.png` 更新。
  块茎雷新增专属色 `flush`（4.7“腮帮憋到发紫”，调色板无紫色）。颜色校验拦下炽球齿线漏写 `fill="none"`（默认黑）；
  尺寸校验拦下荆棘垫宽 96 > 92；葫芦壳初稿壳沿压住宿主眼睛（4.9 要求露出），整体上移 10 px。审核表为葫芦壳
  附罩在石籽荚上的示意。
- [x] 第 3 批（2026-10-16）：咬夹草、风铃花、日灯笼、枝冠枯壳（Helm 2 级 + 破碎）→ 12 个守卫全部完成。
  咬夹草是知识产权风险最高的一张（茎上大嘴植物的常见母题），做成横躺豆荚形，初稿读作“青蛙”已改，已在对照表中
  请负责人重点看；风铃花初稿腮读作眼镜，改为喇叭钟；日灯笼 97 × 110 超限，整体缩至 90%；枝冠初稿有鹿角感，
  改为枝窝。颜色校验又拦下一次漏写 `fill="none"`（日灯笼流苏线）。
- [x] 第 4 批（2026-10-16）：陶盔枯壳、疾行枯壳、跳栏枯壳（vaulted 状态）、门板枯壳 → `review-A1-partial.png` 更新。
  陶盆初稿盖住眼睛、门板初稿挡住眼睛，均调整到“只露眼睛”；跳栏枯壳的芦花初稿读作矛头（1.5 禁止持握武器），改为
  下垂绒穗，杆身 140 px 超限已下移。
- [x] 第 5 批（2026-10-16）：背篓枯壳、鼓腹枯壳（Drum 2 级 + 破碎露出真身）、小芽怪 → **19 个全部完成**。
  背篓枯壳的嘴初稿读作“缝起来的嘴”，改为张开的小口；鼓腹枯壳 159 > 150 px，整体 93%。
- [x] `docs/art/review-A1.png`（21 个角色 · 各受损 / 护甲 / 跨越后状态 · 12 个守卫的头像与“不可用”版 · 葫芦壳罩在
  宿主上的示意）与 `docs/art/review-A1-silhouette.png`；对照表与每个特征的出处 `docs/art/review-A1.md`，其末尾是
  实现者的剪影自查（两对较弱、“小鸟感”仍在且背篓枯壳最明显）。
- 资产：21 角色 / 173 个部件文件 / 62 个片段 / 263 帧（去重后 168 张图）；`garden_defenders.png` 2048 × 780、
  `garden_husks.png` 2048 × 664、`garden_ui.png` 512 × 408。调色板：新增 1 个专属色（块茎雷 `flush`）。
- ~~停下：A1 审核门~~ → 裁决（七）：守卫 1 ～ 12 通过；Husk 设计通过，须统一去鸟化。
- [x] Husk 去鸟化（2026-10-16，v1.2 1.2 / 1.5，只做减法）：脚改为不分趾的根须球、头顶鞘尖向后倒伏、头缩小 15%
  坐在前肩上无颈（背仍是最高点）。枯壳怪剪影自查 `docs/screenshots/a1-husk-debird.png`；跳栏枯壳芦苇杆 8 px、芦花
  加倍下垂、杆顶高出头顶约 30 px；鼓腹枯壳 bare 去掉碗状根座，改为细腿 + 两侧碎壳；`flush` 改为全局色 `#B57BA6`。
  `review-A1.png` / 剪影版重出，说明见 `docs/art/review-A1.md` 末节。

### A0 审核（裁决五，2026-10-16）

- 石籽荚：通过，作为 12 个守卫的模板（画布、描边、阴影、眼与嘴画法、部件拆分）。
- 枯壳怪：不通过（读作“骷髅僵尸”），按 ART-SPEC v1.1 重做，身体（层叠干鞘、配色、根须腿、掉屑）保留：

  | 部位 | 重做后 | 出处 |
  |---|---|---|
  | 眼 | 米色眼球被下垂的椭圆眼皮盖住上半，小瞳孔贴下缘，无高光；无黑眼窝 | v1.1 1.2 |
  | 嘴 | 歪斜小口，2 粒零散干种子（`straw` 色，不用米白以免读成牙），垂一片干叶舌头；无下颌线 | v1.1 1.2 / 5.1 |
  | 头形 | 尖顶干种荚，顶端 4 片干鞘尖向后乱翘、聚在荚尖（不排成一圈，避免读成鸡冠） | v1.1 1.2 / 5.1 |
  | 姿势 | 驼背：头前倾压低到肩下、背部成最高点；双臂下垂、与同侧腿反相摆动；前臂画在头后 | v1.1 1.2 / 5.1 |
  | eat | 前臂向前下方扶在守卫侧面（40°，低于嘴，避免回到“平举前伸”），俯身猛点头咬合 | v1.1 5.1 |
  | 手 | 钝圆的叶鞘末端垂一缕纤维（原先两缕读作钳子） | 1.5 |

- 自查：纯黑剪影已看过，不再能读出“僵尸”，是弯腰的植物。**仍有的顾虑**：尖顶 + 驼背 + 根须脚的剪影略带
  “小鸟”感。我试过在背上加一片下垂长干叶来强化“枯玉米”，结果读作尾羽（更像鸡），已撤回；请负责人判断
  是否需要处理。
- 实机证据（Xvfb，release 无 devtools，花园皮肤）：`docs/screenshots/a0-garden-1-1.png`（第 60 s，一只正在受击
  闪白）、`a0-garden-1-1-strip.png`（第 30 / 45 / 75 / 90 / 105 s 行走）、新增 `a0-garden-1-1-eat-strip.png`
  （临时脚本把石籽荚放在最右格，第 32 ～ 40 s：走近、扶上、俯身咬合，石籽荚受损后叶片下垂）。
- 测试：artgen 15、`tests/skin.rs` 8、`tests/flow.rs` 7 通过；尺寸校验通过；只改了 `art/characters/husk/` 与生成资产。

### A0 管线

- [x] ADR-013：`tools/artgen` 设计与白名单依赖；`check_deps.sh` 登记 `artgen` 允许表，并新增“游戏 crate 传递依赖中
  不得出现 resvg / usvg / tiny-skia”检查（临时给 app 加 resvg 时报 4 条违规，已撤销）
- [x] `art/palette.ron`（14 色 + 1.2 的 `husk_outline`，13 个角色专属色）；`tools/artgen` 解析 rig / clips → 合成 →
  resvg 渲染 → 去重打包 → 引擎 RON；9.2 校验（调色板、画布、1.4 尺寸、时长必需绑定、事件帧、图集 ≤ 4096）；
  14 个单元测试；格式说明 `art/README.md`
- [x] 样板角色 `pebble_pod`（4.1）：idle 270（3 个呼吸周期，张望、眨眼各一次）、attack 85（绑定
  `shooter.interval_ticks`，`fire` 在 0，ADR-014 第 5 条）、受损状态 hurt（叶片下垂）、闪白；`husk`（5.1）：
  walk 40、eat 60（绑定 `bite_interval_ticks`，`eat_hit` 在末帧 50）、闪白。尺寸校验抓到两次超限（husk 133 > 130、
  pebble_pod 107 > 104），已调整造型。资产：2 角色 / 21 个部件文件 / 6 个片段 / 176 帧（去重后 84 张图）；
  `garden_defenders.png` 1024 × 1168、`garden_husks.png` 1024 × 648
- [x] 游戏侧多皮肤（ADR-014）：`SkinName` + `Settings.skin_name`（旧存档兼容）+ `UiAction::SetSkinName`，设置界面
  “占位 / 几何 / 花园”；`Skins` 全部加载，查找顺序“所选 → default → 占位”；清单统一为多图集格式（`gen_skins.py`
  同步，default 图集不变）；`attack` / `produce` / `arm` / `fuse` 由计时器进度驱动（`timed_elapsed`）；
  防御者受损片段与闪白帧。CJK 字体子集随新增文字（几、何、花、园）重新生成
- [x] 测试 `art_assets_up_to_date`（tools/artgen，重建两次并比对）、`art_clips_match_rules`、`skin_garden_loads`
  （先红：无 `Skins` 资源）、`attack_anim_phase_follows_shooter_timer`、`save_without_skin_name_loads`、
  `render` 单元测试 2 个；golden 与 `skin_switching_keeps_golden_1_4` 通过。测试 76 个全绿（artgen 15、core 23、
  content 6、app 32）。说明：`attack_anim_phase_follows_shooter_timer` 在旧映射下也会通过（garden 的 attack 长度
  恰等于周期），新映射的差异部分由 `timed_elapsed` 单元测试覆盖
- [x] 实机截图（Xvfb，release 不带 devtools，隔离存档设为“皮肤 + 花园”，`--autoplay tests/scripts/1-1.ron`）：
  `docs/screenshots/a0-garden-1-1.png`（第 60 s 全屏：石籽荚、两只枯壳怪、卡槽图标均为 garden；弹丸与 Dew 回退
  default）与 `a0-garden-1-1-strip.png`（第 30 / 45 / 75 / 90 / 105 s 的激活车道：行走相位各异、石籽荚射击）。
  1-1 脚本解中枯壳怪走不到石籽荚面前，eat 片段未入镜，由 `eat_anim_phase_follows_biter_timer` 覆盖。
  未经人眼观看连续动画（Xvfb 只能截静帧）

**A0 验收**（ART-SPEC 11）：`art_assets_up_to_date`、`art_clips_match_rules` 通过；golden 未变；实机截图两角色正常。
提交 da89f41（artgen）、0791a00（pebble_pod）、ee6e6e3（husk）、5dcc6db（游戏侧）。
CI run 38699227013（049b3c4）：Ubuntu / macOS 通过（macOS arm64 上 PNG 近似比较生效）；Windows `cargo test` 失败——
日志页无法加载，推断并在本地复现为 CRLF 检出导致 RON 逐字节比较失败，修正见下一提交（ADR-013 第 8 条）。
CI run 38700772389（f4aa88c）三平台全部通过。

## 收尾期（SPEC v1.2 第 20 章，2026-10-14 起）

负责人裁决 `docs/owner-decisions/2026-10-14-g5-closure.md`；任务顺序见 AGENT.md 第 9 章。

**当前状态：** 等待 `docs/levels.md` 主观难度列（手动试玩 10 关，重点 1-9 / 1-10），收到后按 SPEC 20.2 与
AGENT 3.4 逐关处理；等待期间进行美术阶段（AGENT v1.3 第 9 章）。

### 1. 发布清单 SPEC 20.3（2026-10-14，除打 tag 外完成）

- [x] 许可证：`LICENSE-MIT`（版权行 Rootguard contributors）、`LICENSE-APACHE`（apache.org 原文，sha256 cfc7749b…）；
  `[workspace.package] license = "MIT OR Apache-2.0"`，三个 crate 继承（`cargo metadata` 确认）。提交 6370ed8。
- [x] `package.sh` 打包两份许可证并纳入布局自检；本机 Linux 包内容确认含 `LICENSE-MIT` / `LICENSE-APACHE`；
  `check_deps.sh` 通过。提交 1b3fb30。
- [x] `CHANGELOG.md`（Keep a Changelog），`0.1.0` 条目提炼自报告第 4 ～ 6 节，含“已知限制”；日期原为 `Unreleased`，
  负责人已于 7f3049a 改为 2026-10-14（发布说明原样取自该条目）。提交 1b94505。
- [x] `README.md`：简介、截图（`docs/screenshots/battle-placeholder.png` / `battle-skinned.png`，1-5 脚本解约 150 s，
  新增 `scripts/screenshot.sh` 在 Xvfb 中生成）、三平台安装、操作说明、许可证说明。提交 784b88d。
- [x] CI：`v*` tag 触发 `release` 作业（三平台检查与打包通过后，核对 tag 与版本、从 CHANGELOG 提取说明、`gh release`
  上传安装包），Linux push 运行另上传 `rootguard-screenshots` 产物（ADR-011）。提交 15b425d、e94923d。**未打 tag。**
  证据：CI run 38541417044（e94923d）三平台检查、打包与截图全部通过，产物 rootguard-ubuntu-latest 7.69 MB、
  rootguard-macos-latest 6.24 MB、rootguard-windows-latest 5.89 MB、rootguard-screenshots 103 KB。
  15b425d / 3696f05 两次运行的 Linux 截图步骤失败（日志需登录查看；推测缺 X11 / EGL 运行库），e94923d 补齐运行库后通过。
  `release` 作业的 tag 前置逻辑（版本核对、CHANGELOG 提取）已在本地用同一脚本验证；完整流程要到负责人打 tag 时才会首次运行。
- [ ] Git tag `v0.1.0`：负责人在实机验证后打。

### 2. `eat_hit` 事件消费（2026-10-14 完成）

- [x] 呈现层在敌人 eat 片段进入 `eat_hit` 帧时发送引擎 `AnimationEvent`，`SfxPlugin` 播放新音效 `sfx.bite`
  （`gen_audio.py` 生成，登记 `SOURCES.md`；SPEC 13 补一行；ADR-010）。模拟层未改，`determinism_golden_1_4`、
  `skin_switching_keeps_golden_1_4` 通过，`tests/golden/level_1_4.snap` 未变。
- [x] 测试先红后绿：`bite_sfx_on_eat_hit`（红：啃食期间 0 次），另加单元测试 `events_between_reports_entered_event_frames`。
  测试 53 个全绿（core 23、content 6、app 24）。提交 3696f05。
- ~~听感待负责人：每只啃食中的敌人约每秒 2 次~~ → 2026-10-15 按裁决（三）把 eat 片段改为 `[20, 10, 30]` = 60 tick，
  与 `bite_interval_ticks` 一致，`eat_hit` 每秒 1 次（ADR-012）。`gen_skins.py` 校验总长，新增测试
  `eat_clip_matches_bite_interval`（先红：carrier_husk_eat 30 ≠ 60）；只有 13 个 `*_eat.anim.ron` 变化，图集逐字节不变；
  `bite_sfx_on_eat_hit`、`determinism_golden_1_4`、`skin_switching_keeps_golden_1_4` 通过。测试 54 个全绿。提交 f088b42。
- ~~**待负责人决定**：Chill 下仍不对齐~~ → 2026-10-15 按裁决（四）落地：eat 相位 = `Biter.elapsed / 有效间隔`
  × 片段总长，帧序 `[30, 20, 10]`、`eat_hit` 在末帧（图集逐字节不变，13 个 eat 片段变化）。音效在伤害前
  ≤ 10 tick（Chill ≤ 20 tick）触发，同频同相。测试 `bite_sfx_on_eat_hit`（改写，先红：伤害前 10 tick 内无音效）、
  新增 `eat_anim_phase_follows_biter_timer`（先红：`eat_hit` 不在末帧）；golden 与 `skin_switching_keeps_golden_1_4`
  通过。实现与裁决前提的差异（`Biter` 无 `Timer`，呈现层需读 `StatusEffects` / `Rules`）见 ADR-012 后续条目。

### 其他

- `tmp/` 残留清理：前任的 G0 ～ G3 截图与 `tmp/shot.sh`（已由 `scripts/screenshot.sh` 取代）已删除；
  `tmp/venv/` 保留（`scripts/subset_font.py` 文档约定的 fontTools 环境，可重建，从未入库）。

## 里程碑

| 里程碑 | 状态 |
|---|---|
| G0 骨架 | ✅ 2026-10-13 |
| G1 核心规则 | ✅ 2026-10-13（E-2 已于 2026-10-14 落地） |
| G2 可玩对局 | ✅ 2026-10-13（手动试玩待负责人；E-3 已于 2026-10-14 落地） |
| G3 完整流程 | ✅ 2026-10-13（中文显示已于 2026-10-14 随 E-1 落地） |
| G4 全部关卡 | ✅ 2026-10-14（按 SPEC v1.1 返工；手动试玩待负责人） |
| G5 皮肤与打磨 | ✅ 2026-10-14（实机音频听感与 GPU 耗时待负责人） |

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

## G4 全部关卡（2026-10-14 关闭，手动试玩待负责人）

- [x] 1-4 ～ 1-8 的 RON 与脚本解（`every_level_has_scripted_solution` 8 关回放全部胜利，并检查卡组均已解锁）。
- [x] `docs/levels.md` 威胁曲线：每波实际威胁与预算逐一相等（脚本核对）。
- [x] `determinism_golden_1_4`：`tests/golden/level_1_4.snap`（1588 字节，sha256 e3c7a4b1…），debug 与 release 一致。
- [x] devtools 扩展：`RootguardDebugWindow`（F1），Dew +100、下一波、立即胜负、敌人列表；DebugDraw 格子与碰撞区间。
- [x] 1-9：~~第三次调整后得到脚本解（33097 tick，用掉 5 个 Hose）~~ → 按 SPEC v1.1 第 2 档重做，见下节。
- [x] 1-10：~~第三次调整后仍无脚本解~~ → 按 SPEC v1.1 第 1 档重做，见下节。
- [ ] 手动通关全部关卡并记录用时与感受：需真人。

决策：ADR-005。

### G4 返工（SPEC v1.1，2026-10-14 完成，手动试玩待负责人）

验收（SPEC.md 17 G4 + 10.2 / 10.3，AGENT.md 第 9 章）：

- [x] `scripted_solution_uses_at_most_one_hose`（SPEC 16.2）：先运行为红（1-3: 2，1-7: 2，1-8: 2，1-9: 5）；
  返工后 10 关全部 ≤ 1。计数方式（车道 `hose_available`）以 `sfx.hose` 音频请求交叉核对一致（ADR-006）。
- [x] `gen_levels.py` 新增 `--tier / --cap / --flag-cap`（默认第 1 档 40 / 70），超出上限的预算钳到上限；
  1-4 ～ 1-6 重新生成后与已提交 RON 逐字节一致。
- [x] 1-7 / 1-8 / 1-9 / 1-10 重做，每关一次提交；`docs/levels.md` 增加“档位”“Hose 用量”两列：

  | 关 | 档位 | 脚本解 tick | Hose | 搜索合格率（3 卡组 × 512 参数） |
  |---|---|---|---|---|
  | 1-3（只重录） | — | 14254 | 1 | 124 / 512（Forced 卡组） |
  | 1-7 | 1（上限未触发） | 21954 | 0 | 114 / 1536 |
  | 1-8 | 1 | 24137 | 0 | 84 / 1536 |
  | 1-9 | 2（第 1 档 0 / 1536） | 30565 | 1 | 4 / 1536 |
  | 1-10 | 1 | 31731 | 1 | 16 / 1536 |

- [x] 解锁调整：1-9 → `snapmaw` + `sunlantern`，1-10 → `gustbloom`；`every_level_has_scripted_solution` 的
  卡组解锁检查通过（1-10 脚本解使用 `sunlantern`，未使用 `gustbloom`）。
- [x] `every_level_has_scripted_solution`：10 关回放全部胜利。
- [x] `determinism_golden_1_4` 通过，`tests/golden/level_1_4.snap` 未变（sha256 e3c7a4b1…）。
- [x] ADR-002 第 14 条已确认；ADR-005 状态改为已接受并附重做结果；新增 ADR-006。
- [x] `tmp/`：`tmp/drafts/` 与前任的搜索日志 / 临时脚本已删除；`tmp/engine-patches/` 保留到负责人确认引擎合并；
  `tmp/g5-wip/gen_skins.py` 进入 G5 时移入 `scripts/`。`tmp/` 在 `.gitignore` 中（`/tmp/`），从未入库。
- [ ] 手动通关全部 10 关并记录用时与感受：待负责人。

规模：rootguard-core 4652 行、rootguard-content 577 行、rootguard-app 4655 行（含测试与文档注释）；
测试 42 个（core 23、content 6、app 13），另有 2 个 `#[ignore]` 工具（`record_scripts`、`search_plans`）。

遗留：1-9 / 1-10 的机器人合格解很少，难度需真人评估；机器人带咬夹草的卡组在 1-10 上全部不合格（战术实现较弱）。

## G5 皮肤与打磨（2026-10-14 完成）

验收（SPEC.md 17 G5，AGENT.md 第 10 章）：

- [x] `Skinned` 模式美术：`scripts/gen_skins.py`（自 `tmp/g5-wip/` 移入并按 ADR-007 修订造型）生成
  `assets/skins/default/`（图集 1536 × 1536、247 帧、46 个片段、`skin.ron` 清单），引擎 RON 格式；
  防御者 idle 4 帧（射手 / 咬夹草 attack 3 帧），敌人 walk 4 帧 + eat 3 帧（`eat_hit` 事件帧）+ 护甲破损版 + 受伤闪白帧；
  已登记 `assets/SOURCES.md`。证据：`skin_covers_all_content`；Xvfb 窗口截图中 1-10 皮肤模式正常显示。
- [x] **两种模式切换无差异**：`skin_switching_keeps_golden_1_4`（对局中每 500 帧切换一次模式，30000 帧后快照与
  golden 逐字节一致）；`skinned_mode_draws_sprites`；占位模式未改动（`placeholder_mode_draws_all_entities` 通过）。
- [x] Dew 收集飞向 HUD 的 Tween、横幅入场动画、受伤闪白（两种模式）：`tests/visual_fx.rs` 3 个测试（ADR-008）。
- [x] E-5 音乐交叉淡化：引擎 24c4d71（引擎 ADR-011）；游戏场景切换 BGM 交叉淡化 60 tick，首次播放仍为
  `PlayMusic`（`audio_events_mapped` 断言两者，ADR-009）。听感未经人耳确认。
- [x] 引擎 bump 后续：E-1 中文显示（`chinese_hud_when_lang_zh`，Xvfb 截图确认）、E-3 行遮挡改用 `sort_y`
  （`rows_occlude_by_sort_y`）、E-2 加载辅助；golden 不变。
- [x] 三平台打包脚本 `scripts/package.sh`（ADR-008）：本机 Linux 包解压到仓库外可独立运行（Xvfb 截图）；
  CI run 38523777097（1c59eed）三平台检查与打包全部通过，产物 rootguard-ubuntu-latest 7.68 MB、
  rootguard-macos-latest 6.23 MB、rootguard-windows-latest 5.88 MB（macOS / Windows 包未在实机启动）。
- [x] 性能 SPEC 16.3（`cargo run --release -p rootguard-app --example perf_1_10 [-- --skinned] [--stress]`，
  AMD EPYC 9K65，headless CPU 帧耗时，测量方法见 ADR-008）：

  | 场景 | 模式 | 峰值（敌人 / 防御者 / 弹丸） | 最大场面 p50 | p99 | 最大 |
  |---|---|---|---|---|---|
  | 1-10 脚本解 | Placeholder | 31 / 31 / 10 | 43 µs | 69 µs | 554 µs |
  | 1-10 脚本解 | Skinned | 31 / 31 / 10 | 45 µs | 69 µs | 180 µs |
  | 压力场景 | Placeholder | 43 / 45 / 94 | 64 µs | 83 µs | 889 µs |
  | 压力场景 | Skinned | 43 / 45 / 94 | 67 µs | 100 µs | 1.4 ms |

  均远低于 4 ms。devtools 面板的 “frame ms” 是真实帧间隔（垂直同步下约 16.7 ms），不反映负载；
  实机 GPU / egui 绘制耗时需负责人在桌面环境补充。
- [x] `docs/game-v1-report.md` 已提交。
- [x] `tmp/engine-patches/` 已删除（补丁均已合入引擎 main）。

规模：rootguard-core 4652 行、rootguard-content 577 行、rootguard-app 5718 行（含测试与文档注释）；
测试 51 个（core 23、content 6、app 22）。决策：ADR-007、ADR-008、ADR-009。
