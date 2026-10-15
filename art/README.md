# art/ — garden 皮肤的源文件（ART-SPEC 9）

本目录是 `assets/skins/garden/` 的唯一来源。不要手改生成的资产，改这里然后运行：

```bash
cargo run --release -p artgen -- preview <id>   # 自查图写到 tmp/artgen/（不入库）
cargo run --release -p artgen -- build          # 生成 assets/skins/garden/
cargo run --release -p artgen -- check          # 只比较；CI 中由 art_assets_up_to_date 执行
cargo run --release -p artgen -- review [--partial]  # A1 审核表写到 docs/art/
```

设计依据：每个造型特征必须能指向 `LORE.md` 或 `ART-SPEC.md` 设计卡中的一句话，并在部件 SVG 的注释里
写明出处。`docs/art/*.png` 概念图只是参考板，禁止描摹或放进本目录。工具设计见 ADR-013。

## 目录

```
art/
├── palette.ron                 调色板（ART-SPEC 1.3）
└── characters/<id>/            id = 防御者 / 敌人 / 弹丸 id
    ├── parts/<部件>.svg        部件；变体为 <部件>_<变体>.svg
    ├── rig.ron                 类别、部件层级、枢轴、z 序、变体、受损状态
    └── clips.ron               片段：时长、帧划分、关键帧、变体切换、事件帧
```

## 部件 SVG

- 完整的 SVG 文件，`viewBox` 必须等于类别画布（ART-SPEC 2.1，如守卫 `0 0 128 128`、Husk `0 0 128 160`），
  所有部件在同一角色坐标系中绘制，因此默认位姿就是各部件原样叠放。
- 只允许 `g` / `path` / `circle` / `ellipse` / `rect` / `polygon` / `polyline` / `line`；可以用 `transform`
  与 `opacity`。
- 颜色只能写 `fill="@名"` / `stroke="@名"` 或 `none`；`@名.shade` 是一阶阴影色。名字来自 `palette.ron`
  的全局色与该角色的专属色。每个形状都要写 `fill`（或继承自外层 `g`），否则 SVG 默认黑色会被拒绝。
- 描边：守卫 `@outline`、Husk `@husk_outline`，轮廓 3 px（ART-SPEC 1.2）。
- 禁止 `style`、`id`、`class`、`href`、渐变、滤镜、蒙版、位图与文字。

## rig.ron

```ron
Rig(
    kind: Guardian,        // Guardian | GuardianGround | Overlay | Husk | HuskLarge | HuskSmall | Projectile | Dew
    rest: "idle",          // 默认位姿 = 该片段第 0 帧（尺寸校验、审核表、卡牌头像）
    portrait: Some((24, 30, 80)),  // 可选：卡牌头像取景 (x, y, 宽)，缺省按可见范围自动取景
    flash: true,           // 每帧另出 <帧名>_flash 闪白帧（ART-SPEC 2.3）
    parts: [
        // parent：叠加父部件的变换；pivot：旋转 / 缩放枢轴；z：绘制顺序；variants：可切换的变体文件
        (name: "body", pivot: (62, 114), z: 3),
        (name: "eyes", parent: Some("body"), pivot: (77, 65), z: 6, variants: ["closed"]),
        (name: "crumbs", pivot: (40, 78), z: 8, hidden: true),   // 默认隐藏，切到 "base" 时出现
    ],
    states: [              // 受损 / 护甲状态（ART-SPEC 2.4）：输出片段 <id>_<状态>_<片段>
        (name: "", set: {}),
        (name: "hurt", set: {"leaves": "droop"}),
    ],
)
```

类别决定画布、锚点、材质（阴影比例）、所属图集与可见尺寸上限（ART-SPEC 1.4，`artgen` 校验）。

## clips.ron

```ron
Clips([
    (
        name: "attack",
        looping: true,
        duration: Rule("defenders.pebble_pod.shooter.interval_ticks"),   // 或 Ticks(90)
        frames: List([4, 4, 4, 12, 12, 12, 12, 8, 8, 9]),                // 或 Every(10)
        events: [(0, "fire")],                                           // tick 必须是帧起点
        keys: { "body": [(t: 0, dx: 3, rot: 7, ease: Linear), (t: 76, dx: -6, rot: -6)] },
        swaps: { "mouth": [(0, "o"), (4, "base")] },                     // "base" 基础文件，"hidden" 隐藏
    ),
])
```

- 关键帧字段 `dx` / `dy`（px）、`rot`（度，顺时针）、`sx` / `sy`，缺省为恒等；`ease` 作用于到下一关键帧
  的区间（`Smooth` 默认 / `Linear` / `Step`）。每帧在其起始 tick 取样。
- 受损状态的变体优先于片段中的切换。
- 由计时器驱动的片段必须绑定规则（ART-SPEC 3.1）：`attack`、`produce`、`arm`、`fuse`、`eat`。
  `eat` 恰有一个 `eat_hit` 且在末帧；`attack` 的 `fire` 数等于连发数。
- 计时器驱动的片段在游戏中按计时器进度取帧（ADR-014）：片段时间 0 = 计时器刚重置的那个 tick。
