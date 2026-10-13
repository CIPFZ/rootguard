# rootguard-core

Rootguard 的规则与模拟层（SPEC.md 第 4 ～ 6 章）：组件、资源、事件、FixedUpdate 系统与数据模型。

- 依赖：只允许 `trellia-core`、`serde`、`ron`、`log`、`thiserror`（SPEC.md 3.1 R1，`scripts/check_deps.sh` 检查）。
- 禁止出现任何渲染 / 音频 / UI 类型（R2）；呈现层通过事件响应模拟。
- 全部规则可在 headless 下单元测试：`cargo test -p rootguard-core`。
