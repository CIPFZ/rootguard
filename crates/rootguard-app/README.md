# rootguard-app

Rootguard 应用：插件组装、场景、egui UI、占位 / 皮肤渲染、音效、存档。

```bash
cargo run --release -p rootguard-app                 # 窗口运行
cargo run -p rootguard-app --no-default-features     # 不带 devtools
```

插件顺序（SPEC.md 3.2）：`AssetsPlugin` → `GfxPlugin` → `AudioPlugin` → 游戏内容插件 →
`ScenePlugin::new(TitleScene)` → `DevtoolsPlugin`（feature `devtools`，默认开启）。
