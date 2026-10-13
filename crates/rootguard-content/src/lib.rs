//! # rootguard-content
//!
//! 职责：从 `assets/data/` 加载防御者 / 弹丸 / 敌人 / 关卡 RON，按 SPEC.md 4.5 校验，
//! 构建只读注册表并交给 `rootguard-core` 的 `ContentPlugin`（SPEC.md 3.3）。
//!
//! 依赖边界（SPEC.md 3.1）：只依赖 `rootguard-core` 与 `trellia-assets`（及通用库，ADR-001）。
//!
//! 不负责：规则与模拟（`rootguard-core`）、渲染与 UI（`rootguard-app`）。

#![forbid(unsafe_code)]
