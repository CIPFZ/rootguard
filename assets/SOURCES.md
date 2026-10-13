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
| `assets/audio/bgm_title.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 标题 / 菜单 BGM（循环） |
| `assets/audio/bgm_battle.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 对局 BGM（循环） |
| `assets/audio/bgm_outcome.ogg` | 音频 | 程序化合成：`scripts/gen_audio.py`（标准库波形合成 + oggenc 编码，旋律由固定种子在五声音阶上生成） | Rootguard 项目 | 项目自有 | 结算 BGM（循环） |
| `assets/fonts/RootguardCJK-Regular.otf` | 字体 | Noto Sans CJK SC Regular 子集，`scripts/subset_font.py` 生成（字符取自 `assets/data/i18n/*.ron`），已更名为 “Rootguard CJK Subset” | Adobe / Google（Noto CJK） | SIL OFL 1.1，全文见 `assets/fonts/LICENSE-OFL.txt` | 中文显示（E-1） |
