# 素材来源登记（SPEC.md 1.2.4 / 12.3 / 13）

每个图像 / 音频 / 字体文件一行。程序化生成的素材在“来源”列写生成脚本路径；
外部素材只允许 CC0（或自制），须写明原始链接与作者。

| 文件 | 类型 | 来源 | 作者 | 许可证 | 备注 |
|---|---|---|---|---|---|
| `assets/icon/rootguard-icon.png` | 图像 | 项目负责人提供 | 项目负责人 | 项目自有 | 应用图标，只读 |
| `assets/audio/sfx_shoot.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 发射 |
| `assets/audio/sfx_hit.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 命中 |
| `assets/audio/sfx_hit_armor.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 命中护甲 |
| `assets/audio/sfx_plant.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 放置 |
| `assets/audio/sfx_dig.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 铲除 |
| `assets/audio/sfx_dew.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 收集 Dew |
| `assets/audio/sfx_husk_down.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 敌人倒下 |
| `assets/audio/sfx_flag_wave.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 大波来袭 |
| `assets/audio/sfx_hose.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | Hose Burst |
| `assets/audio/sfx_win.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 胜利 |
| `assets/audio/sfx_lose.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 失败 |
| `assets/audio/sfx_bite.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，滤波噪声 + 下滑正弦） | Rootguard 项目 | 项目自有 | 啃食（eat_hit 动画帧，ADR-010） |
| `assets/audio/bgm_title.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 标题 / 菜单 BGM（循环） |
| `assets/audio/bgm_battle.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 对局 BGM（循环） |
| `assets/audio/bgm_outcome.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 结算 BGM（循环） |
| `assets/fonts/RootguardCJK-Regular.otf` | 字体 | Noto Sans CJK SC Regular 子集，`scripts/subset_font.py` 生成（字符取自 `assets/data/i18n/*.ron`），已更名为 “Rootguard CJK Subset” | Adobe / Google（Noto CJK） | SIL OFL 1.1，全文见 `assets/fonts/LICENSE-OFL.txt` | 中文显示（E-1） |
| `assets/skins/default/skin.png` | 图像 | 程序化生成：`scripts/gen_skins.py`（Pillow 绘制基本几何形状，4 倍超采样；造型原创，见 ADR-007） | Rootguard 项目 | 项目自有 | 皮肤图集：12 防御者、9 敌人（含受伤闪白帧）、2 弹丸、Dew |
| `assets/skins/default/skin.atlas.ron` | 数据 | 程序化生成：`scripts/gen_skins.py` | Rootguard 项目 | 项目自有 | 引擎图集 RON |
| `assets/skins/garden/*.png` | 图像 | 程序化生成：`tools/artgen` 以 resvg 渲染 `art/characters/*/parts/*.svg`（手写 SVG 部件，颜色只取 `art/palette.ron`；造型依据 `LORE.md` 与 `ART-SPEC.md` 设计卡，见 ADR-013） | Rootguard 项目 | 项目自有 | garden 皮肤图集（美术阶段，逐角色增加） |
| `assets/skins/garden/*.atlas.ron`、`anim/*.anim.ron`、`skin.ron` | 数据 | 程序化生成：`tools/artgen` | Rootguard 项目 | 项目自有 | 引擎图集 / 动画片段 RON 与皮肤清单 |
| `assets/skins/garden/bg_base.png` | 图像 | 程序化生成：`tools/artgen` 的 `bg.rs`（L0 天空渐变与 L2 草地格按 ART-SPEC 8.1 公式由调色板计算；L1 远景渲染 `art/bg/far.svg`，手写 SVG） | Rootguard 项目 | 项目自有 | 战场静态底图（ART-SPEC 8.1 L0 ～ L2，ADR-015） |
| `art/**/*.svg`、`art/**/*.ron` | 源文件 | 实现者手写（不描摹、不位图化概念图，ART-SPEC 9.3） | Rootguard 项目 | 项目自有 | garden 皮肤的唯一来源 |
| `assets/skins/default/skin.ron` | 数据 | 程序化生成：`scripts/gen_skins.py` | Rootguard 项目 | 项目自有 | 皮肤清单（图集路径与片段名） |
| `assets/skins/default/anim/*.anim.ron`（46 个） | 数据 | 程序化生成：`scripts/gen_skins.py` | Rootguard 项目 | 项目自有 | 引擎动画片段 RON（idle / attack / armed / walk / eat，eat 第 2 帧 `eat_hit` 事件） |
