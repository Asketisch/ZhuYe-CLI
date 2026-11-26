//! 为 vendored tui_core 代码兜上游 API 的兼容垫片(restructure 批 3)。
//!
//! 原为 src 根平铺;旧路径 `crate::<name>` 经 `lib.rs` 再导出垫片保持
//! 兼容,新代码请写 `crate::compat::<name>`。

pub mod random_range_compat;
pub mod tui_compat;
