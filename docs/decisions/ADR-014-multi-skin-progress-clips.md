# ADR-014：多皮肤加载与计时类片段的进度驱动

- 编号：ADR-014
- 标题：多皮肤加载与计时类片段的进度驱动
- 日期：2026-10-15
- 状态：已接受；推广负责人裁决 2026-10-15（四）的啃食相位做法（ADR-012 后续条目）

## 背景

AGENT v1.3 第 9 章 A0 第 4 项：皮肤清单支持 `default` / `garden` 多皮肤，`SkinMode::Skinned` 可选皮肤名；
`attack` / `produce` / `fuse` / `arm` 片段按 ART-SPEC 3.1 由对应计时器进度驱动。未定之处：皮肤名放在哪里、
旧存档如何兼容、garden 尚未覆盖全部角色时（A0 / A1 只有部分角色）怎样显示、多图集清单的格式，以及
`attack` 的 `fire` 帧位置（ART-SPEC 4.1 写“`fire` 在 84”）。

## 决策

1. **皮肤名**：`rootguard-core` 新增 `SkinName { Default, Garden }`（`dir()` 给出 `assets/skins/<dir>/`），
   `Settings.skin_name` 以 `#[serde(default)]` 加入——旧存档没有该字段时取 `Default`，无需提升存档版本
   （测试 `save_without_skin_name_loads`）。新增 `UiAction::SetSkinName`；`SkinMode` 不变（`Skinned` 加
   皮肤名）。设置界面三个选项：占位 / 几何（`default`）/ 花园（`garden`）。A3 把默认值改为 `Garden`。
   模拟层与快照不读取设置，golden 不受影响。
2. **加载与回退**：启动时加载全部皮肤到资源 `Skins`；单个皮肤失败只警告。查找视图 `SkinView` 的顺序为
   “所选皮肤 → `default`”，再缺才画占位图形（SPEC 12.1 允许部分实体回退）。因此美术阶段中途选择花园
   皮肤时，已完成的角色用 garden、其余用 default 的几何造型，游戏始终完整可玩。动画事件（啃食音效）
   同样经所选皮肤的视图取片段，与渲染模式无关（ADR-010）。
3. **清单格式**：`skin.ron` 统一为 `Skin(atlases: [(atlas: "...", clips: [...]), ...])`，每个片段在其所属
   图集中解析帧名。`scripts/gen_skins.py` 改为输出同一格式（`default` 只有一张图集；图集与片段逐字节不变，
   只有 `skin.ron` 变化）。
4. **进度驱动**（`render::defender_pose_with` / `timed_elapsed`）：片段时间 = 计时器本周期已过 tick
   `duration − remaining`；片段不短于周期时按 `已过 / 周期 × 片段长` 映射（garden 片段长 = 规则时长，
   即逐 tick 对应），更短的片段（`default` 的 18 tick 一次性 attack）从周期起点播放一次后回到 idle——
   一个公式同时覆盖两套皮肤。对应关系：`attack` ← `Shooter.timer`（就绪且无目标时 idle）、`produce` ←
   `Producer.timer`、`arm` ← `Mine.arm`（武装后 `armed` 按时钟）、`fuse` ← `Blast.fuse` / `Gust.fuse`。
   `eat` 见 ADR-012 后续条目。
5. **`fire` 帧在片段时间 0**：射手计时器到期且有目标时，同一 tick 重置计时器并生成弹丸；该 tick 末计时器
   已过 0 tick，所以与弹丸“同 tick”（ART-SPEC 3.2）的是片段时间 0。若把 `fire` 放在 84，它只显示 1 tick
   且比弹丸早 1 tick；循环片段中 0 ≡ 85 ≡ 进度 1.0，与 3.1“`fire` 帧在 1.0”一致，只是 4.1 的数字写法不同
   （ART-SPEC 与模拟时序冲突时以 SPEC 为准，ART-SPEC 第 0 章）。石籽荚的 attack 因此为：0 ～ 11 发射与
   后坐回弹、12 ～ 59 idle 姿态、60 ～ 84 蓄力。双籽荚的两枚分别在 0 与 `volley_gap_ticks`（6），A2 落地。
6. **受损状态与闪白**：防御者 `hp × 2 ≤ max`（石籽荚 hp ≤ 3）且皮肤有 `<id>_hurt_<片段>` 时改用受损片段，
   此时不再叠加低生命着色（造型已表达受损，ART-SPEC 2.4）；没有受损片段的皮肤保持原着色。防御者没有
   受伤事件，呈现层以“生命值比上一帧少”判定并显示 `_flash` 帧 6 tick（ART-SPEC 2.3）；只有带闪白帧的
   皮肤（garden）可见，占位模式不变（SPEC 12.2 只规定敌人闪白）。

## 后果

- `garden` 可以逐个角色交付；每加一个角色，选花园皮肤时该角色立即换上新造型。
- 计时类片段的长度若与规则不符，artgen 拒绝生成（ADR-013），`art_clips_match_rules` 也会失败。
- `Settings` 多了一个字段，`UiAction` 多了一个变体；均不进入模拟与快照。

## 替代方案

- `SkinMode::Skinned(SkinName)`：RON 中单元变体 `Skinned` 无法反序列化为带值变体，旧存档会被当作损坏
  并备份重置，否决。
- garden 缺片段时直接画占位：A0 ～ A2 期间花园皮肤大部分角色是方块，不利于审核与实机试看，否决。
- 只在皮肤为 garden 时进度驱动：两套代码路径；`timed_elapsed` 的公式对 default 结果不变，否决。
