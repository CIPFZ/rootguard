# Rootguard 开发进度

## 会话记录（新记录追加在最上方）

- 2026-10-13 会话 1：通读 SPEC.md / AGENT.md、引擎 v1 报告、ADR-001 ～ 007 与 `examples/lane_demo`；
  计划完成 G0 骨架并开始 G1。

## 给项目负责人的待办

- [ ] 填写 `scripts/ip_denylist.txt`（AGENT.md 第 6 章；实现者不得自行添加第三方作品名）。
- [ ] 仓库原有目录 `assests/icon/` 疑为笔误，已移动为 SPEC / AGENT / README 所述的 `assets/icon/`（文件未改动）。

## 里程碑

| 里程碑 | 状态 |
|---|---|
| G0 骨架 | ✅ 2026-10-13 |
| G1 核心规则 | 未开始 |
| G2 可玩对局 | 未开始 |
| G3 完整流程 | 未开始 |
| G4 全部关卡 | 未开始 |
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
