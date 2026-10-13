//! # rootguard-core
//!
//! 职责：Rootguard 的规则与模拟层——组件、资源、事件、FixedUpdate 系统、
//! 防御者 / 敌人 / 弹丸 / 关卡的数据模型（SPEC.md 第 4 ～ 6 章）。全部逻辑可在 headless 下测试。
//!
//! 依赖边界（SPEC.md 3.1 R1 / R2）：只依赖 `trellia-core`、`serde`、`ron`、`log`、`thiserror`；
//! 禁止出现任何呈现类型。渲染与音效由 `rootguard-app` 响应本 crate 发出的事件完成。
//!
//! 不负责：资产文件读取（`rootguard-content`）、场景、UI、渲染、音频、存档（`rootguard-app`）。

#![forbid(unsafe_code)]
