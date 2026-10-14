# ADR-008：G5 视觉效果、打包与性能测量方法

- 编号：ADR-008
- 标题：G5 视觉效果、打包与性能测量方法
- 日期：2026-10-14
- 状态：已接受

## 背景

实现 SPEC 17 G5（Dew 飘落 Tween、横幅动画、三平台打包、16.3 性能）时的未定之处：

1. SPEC 9 要求“收集后的飘向 HUD 动画为纯视觉”，但 `DewCollected { amount }` 不带位置，且被 `SfxPlugin`
   在当帧取走（ADR-004 第 4 条）。
2. SPEC 16.3 写“帧耗时 < 4 ms；以 devtools 面板读数为准”。引擎面板显示的是**真实帧间隔**的平均值
   （`real_dt`），开启垂直同步时恒约 16.7 ms，与负载无关，无法据此判断 4 ms 预算。
3. “三平台打包脚本”的形式未定义；仓库根目录没有项目许可证文件（项目许可证由负责人决定）。

## 决策

1. **Dew 收集判定在呈现层**：`RenderState` 记录上一帧可见的 Dew（位置与过期计时剩余）；实体消失且
   剩余 > 5 tick（一帧最多推进 5 tick）即视为被收集，生成从该位置到 HUD 图标的 `TweenVec2`（0.5 s，缓出）。
   不改 core 事件。横幅文字变化时以 `TweenF32`（0.35 s）滑入并淡入。补间按 `real_dt` 推进，暂停时
   `BattleScene::update` 不运行，补间随之静止。
2. **性能以 CPU 帧耗时衡量**：`examples/perf_1_10.rs` 在 release headless 下逐帧计时
   （FixedUpdate + Update + Render 的 DrawList 生成，不含 GPU 与 egui 绘制），提供 1-10 脚本解回放与
   达到 SPEC 假设负载（≈ 40 敌人、45 防御者、100 弹丸）的压力场景，两种渲染模式各测一次。
   窗口内的实机读数需在关闭垂直同步或使用外部分析器时由负责人补充。
3. **打包**：一个 bash 脚本 `scripts/package.sh` 覆盖 Linux / macOS / Windows（Git Bash；CI 的 Windows
   运行器自带），输出“可执行文件 + assets/ + README.txt”目录与 `.tar.gz` / `.zip`，带布局自检。
   发布构建关闭 `devtools`（AGENT 4.4）。不附项目许可证（由负责人决定）；素材许可证随 `assets/` 一起分发
   （`SOURCES.md`、`fonts/LICENSE-OFL.txt`）。CI 在 push 时三平台打包并上传产物。

## 后果

- Dew 判定是启发式的：恰在过期前 5 tick 内被收集的 Dew 不播放飞行动画（纯视觉，可接受）。
- 性能数字与机器相关；`docs/progress.md` 记录测量机器与结果。

## 替代方案

- （1）给 `DewCollected` 加位置字段：改变 SPEC 5.3 的事件定义，否决；在 `SfxPlugin` 之前插入读取事件的
  系统：事件仍无位置，否决。
- （2）在测试中断言耗时：测试禁止依赖真实时间（AGENT 4.5），否决。
- （3）为 Windows 另写 PowerShell 脚本：逻辑重复，否决。
