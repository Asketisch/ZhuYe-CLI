//! `utils_*` 辅助函数族的归组目录(restructure 批 1)。
//!
//! 原为 src 根平铺的 `utils_*.rs`;旧路径 `crate::utils_*` 经 `lib.rs`
//! 再导出垫片保持兼容,新代码请写 `crate::utils::<name>`。

pub mod absolute_path;
pub mod approval_presets;
pub mod cli;
pub mod elapsed;
pub mod fuzzy_match;
pub mod oss;
pub mod path;
pub mod path_uri;
pub mod plugins;
pub mod sandbox_summary;
pub mod sleep_inhibitor;
pub mod string;
