# ADR-011：v0.1.0 发布清单的实现方式

- 编号：ADR-011
- 标题：v0.1.0 发布清单的实现方式
- 日期：2026-10-14
- 状态：已接受；取代 ADR-008 第 3 条中“不附项目许可证”的部分
- 备注：编号 010 按 AGENT.md 第 9 章第 2 项留给 `eat_hit` 音效；本 ADR 先于其提交

## 背景

SPEC 20.3 与负责人裁决 `docs/owner-decisions/2026-10-14-g5-closure.md` 给出了发布清单（双许可、CHANGELOG、
README 截图、tag 触发 Release 上传），以下细节未写明：

1. SPEC 20.3 字面写单个 `LICENSE`，裁决与 AGENT.md 要求 `LICENSE-MIT` + `LICENSE-APACHE`；Apache 文本的附录
   含 `[yyyy] [name of copyright owner]` 占位。
2. Release 由哪个工具创建、发布说明从哪里来、tag 与版本号不一致时怎么办。
3. README 截图如何生成（“CI Xvfb 截图”），以及皮肤模式如何在不改游戏代码的前提下打开。

## 决策

1. **许可证文件**以裁决为准（裁决优先于 SPEC 字面，二者意图一致）：`LICENSE-MIT` 版权行
   “Copyright (c) 2026 Rootguard contributors”；`LICENSE-APACHE` 为 apache.org 的 LICENSE-2.0.txt 原文
   （sha256 cfc7749b…），附录是给使用者的套用说明，保持原样不填。`license` 字段写在 `[workspace.package]`，
   各 crate `license.workspace = true`。`package.sh` 把两份文件放在安装包根目录并纳入布局自检。
2. **Release 作业**（`.github/workflows/ci.yml` 的 `release`）：只在 `v*` tag 上运行，`needs: check`，
   即三平台检查与打包全部通过后才发布。使用运行器自带的 `gh` CLI，不引入第三方 Action。先核对
   tag 与 `Cargo.toml` 的 `[workspace.package] version` 一致；发布说明取自 `CHANGELOG.md` 中该版本的条目
   （含“已知限制”，满足 SPEC 20.3“未验证项在 Release 说明中标注”），没有条目则失败。
   Release 已存在（重跑）时改为 `gh release upload --clobber`。
3. **截图**：`scripts/screenshot.sh` 在空闲显示号上启动 1280 × 720 的 Xvfb，以 `--autoplay` 回放 1-5 脚本解
   约 150 秒后截图；渲染模式通过隔离的 `ROOTGUARD_SAVE_DIR` 中预置的 `progress.ron` 设置（SPEC 14），
   不新增命令行参数、不读写用户存档。CI 在 Linux 的 push 运行中执行同一脚本并上传为 `rootguard-screenshots`
   产物；仓库内 `docs/screenshots/` 的两张图由该脚本在本地 Xvfb 生成，需要更新时用 CI 产物或重跑脚本替换。

## 后果

- 负责人打 tag 前需把 `CHANGELOG.md` 中 `[0.1.0]` 的日期由 `Unreleased` 改为实际日期（发布说明会原样带出）。
- 截图时刻按真实时间，两次运行的画面可能相差几帧；模拟本身确定，内容基本一致。
- CI 的 Linux 作业因截图增加约 5 分钟，与 macOS / Windows 作业并行。

## 替代方案

- （2）`softprops/action-gh-release` 等第三方 Action：多一个外部依赖，`gh` 已能完成，否决。
- （2）打 tag 时自动生成发布说明（提交列表）：不含已知限制，不满足 SPEC 20.3，否决。
- （3）给游戏加 `--skin` 命令行参数：为截图改动游戏入口，收尾期应克制，否决。
