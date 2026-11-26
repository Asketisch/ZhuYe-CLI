//! `zhuye_*` 引擎 crate 的本地桩实现族(restructure 批 2)。
//!
//! 与 protocol_compat / app_server_protocol / config_compat 桩层同族:为
//! vendored tui_core 代码补引擎侧类型的本地最小实现,不追求与上游一一对应
//! (契约见 crate 篇 §桩层)。原为 src 根平铺;旧路径 `crate::<name>` 经
//! `lib.rs` 再导出垫片保持兼容,新代码请写 `crate::crate_stubs::<name>`。

pub mod arg0;
pub mod cloud_config;
pub mod connectors;
pub mod core_plugins;
pub mod exec_server;
pub mod features;
pub mod feedback;
pub mod file_search;
pub mod git_utils;
pub mod login;
pub mod mcp;
pub mod message_history;
pub mod model_provider;
pub mod model_provider_info;
pub mod models_manager;
pub mod otel;
pub mod plugin;
pub mod rollout;
pub mod state;
