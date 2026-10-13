# rootguard-content

Rootguard 内置内容的加载与校验（SPEC.md 第 4 章）：读取 `assets/data/**/*.ron`，
按 4.5 校验后构建注册表，交给 `rootguard-core` 使用。任一文件失败即记为启动错误（含文件路径与字段名）。

- 依赖：`rootguard-core`、`trellia-assets`（SPEC.md 3.1，ADR-001）。
