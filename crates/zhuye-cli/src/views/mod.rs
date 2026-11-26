//! 人读渲染 / overlay 视图族(restructure 批 5):待办、目标态、分屏、
//! 路由状态、模型清单、任务/转录分页器、工具参数流、审批弹窗富渲染。
//! 消费点集中在 `tui/mod.rs::draw` 叠加。原为 src 根平铺;旧路径经
//! `lib.rs` 再导出垫片保持兼容,新代码请写 `crate::views::<name>`。

pub mod approval_scope;
pub mod goal_state_view;
pub mod model_list;
pub mod routing_status;
pub mod split_view;
pub mod tasks_pager;
pub mod todo_view;
pub mod tool_args;
pub mod transcript_pager;
