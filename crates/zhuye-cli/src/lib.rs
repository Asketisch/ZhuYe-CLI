#![allow(clippy::collapsible_if)]
#![allow(dead_code)]
// 宽松头(沿用 Reflect-Agent 工作区惯例):以下 lint 命中的是仓库既有代码
// 风格,批量改写收益低、风险高(rust 1.98 clippy 新增/收紧后全量暴露),
// 统一在此豁免;新代码仍以 `cargo clippy -D warnings` 全绿为门槛。
#![allow(
    clippy::field_reassign_with_default,
    clippy::result_large_err,
    clippy::doc_lazy_continuation,
    clippy::empty_line_after_doc_comments,
    clippy::unnecessary_fallible_conversions,
    clippy::too_many_arguments
)]
//! ZhuYe-CLI —— ZhuYe 的终端交互界面(ratatui)。
//!
//! 架构:核心引擎(zhuye-core / AgentThread 等)由 zhuye-agent submodule 提供,
//! 本 crate 只负责表现层 —— 事件循环、布局、ratatui 渲染、协议事件适配。
//!
//! `tui_core` 下是本仓库自包含的呈现原语;ratatui/crossterm fork 的来源说明
//! 见根 `Cargo.toml` `[patch.crates-io]` 段注释。外层 `tui` 模块拥有本地
//! 事件循环、布局与 event/adapter 边界。

mod adapter;
mod bootstrap;
mod bootstrap_wiring;
mod config_reload;
mod events;
mod history_render;
mod logging;
mod picker;
mod term;
mod tui;
mod tui_core;
mod views;

pub mod ansi_escape;
pub mod app_server_client;
pub mod app_server_protocol;
/// Headless 子命令(fork / rename / export / traces),由顶层二进制 dispatch。
pub mod cli;
/// 为 vendored tui_core 兜上游 API 的兼容垫片族(原 src 根平铺,restructure 批 3)。
pub mod compat;
pub mod config_compat;
/// `zhuye_*` 引擎 crate 的本地桩实现族(原 src 根平铺,restructure 批 2)。
pub mod crate_stubs;
pub mod install_context;
pub mod prompt_history;
pub mod protocol_compat;
pub mod resume;
pub mod shell_command;
pub mod utils;

// ── restructure 垫片 ──
// 原 src 根 `utils_*.rs` 已归入 `src/utils/`;旧路径经再导出保持兼容,
// 属临时物,待 tui_core / 根文件引用全部迁往 `crate::utils::` 新路径后移除。
pub use utils::absolute_path as utils_absolute_path;
pub use utils::approval_presets as utils_approval_presets;
pub use utils::cli as utils_cli;
pub use utils::elapsed as utils_elapsed;
pub use utils::fuzzy_match as utils_fuzzy_match;
pub use utils::oss as utils_oss;
pub use utils::path as utils_path;
pub use utils::path_uri as utils_path_uri;
pub use utils::plugins as utils_plugins;
pub use utils::sandbox_summary as utils_sandbox_summary;
pub use utils::sleep_inhibitor as utils_sleep_inhibitor;
pub use utils::string as utils_string;

// 原 src 根 `zhuye_*` crate 桩 19 个已归入 `src/crate_stubs/`;旧路径
// `crate::<name>` / `zhuye_cli::<name>` 经再导出保持兼容(临时垫片,
// 引用迁往 `crate::crate_stubs::` 后移除)。
// v0.1.8-develop 接线:/checkpoint 的 git 实现收编进 submodule 的 zhuye-git
// crate,本仓库删除 git_checkpoint / branch_summary / git_action_directives
// 三份本地实现(git_utils 现为桩)。
pub use crate_stubs::{
    arg0, cloud_config, connectors, core_plugins, exec_server, features, feedback, file_search,
    git_utils, login, mcp, message_history, model_provider, model_provider_info, models_manager,
    otel, plugin, rollout, state,
};

// 原 src 根 `tui_compat.rs` / `random_range_compat.rs` 已归入 `src/compat/`;
// 旧路径经再导出保持兼容(临时垫片,引用迁往 `crate::compat::` 后移除)。
pub use compat::{random_range_compat, tui_compat};

// 原 src 根终端基础设施 5 个已归入 `src/term/`;旧路径 `crate::<name>` 经
// 再导出保持兼容(临时垫片,引用迁往 `crate::term::` 后移除)。四个原本私有
// 的模块(keymap / terminal / terminal_probe / viewport)用 crate 内 `use`
// 保可见性不变,theme 原为 pub mod,仍 pub use 对外导出。
pub use term::theme;
use term::{keymap, terminal, terminal_probe, viewport};

// 原 src 根人读渲染/overlay 视图 9 个已归入 `src/views/`(原均为私有
// mod);旧路径 `crate::<name>` 经 crate 内 use 保持兼容(临时垫片,
// 引用迁往 `crate::views::` 后移除)。
use views::{
    approval_scope, goal_state_view, model_list, routing_status, split_view, tasks_pager,
    todo_view, tool_args, transcript_pager,
};

use clap::Args;

pub use adapter::{UiEvent, UiEventKind, UiHistoryItem};
pub use events::UiState;
pub use tui_core::custom_terminal::Terminal as ReflectTerminal;
pub use tui_core::insert_history::{HistoryLineWrapPolicy, insert_history_lines};
pub use tui_core::live_wrap::{Row, RowBuilder, take_prefix_by_width};
pub use tui_core::markdown_render::render_markdown_text;
pub use tui_core::public_widgets::composer_input::{ComposerAction, ComposerInput};
pub use tui_core::wrapping::RtOptions;

#[derive(Debug, Default, Args)]
#[group(multiple = false, required = false)]
pub struct TuiArgs {
    #[arg(long)]
    pub prompt: Option<String>,
    #[arg(long, short = 'c', group = "input")]
    pub continue_last: bool,
    #[arg(long, short = 'r', value_name = "N", group = "input")]
    pub resume_by: Option<usize>,
    #[arg(long, group = "input")]
    pub resume: Option<String>,
    #[arg(long)]
    pub plan_mode: bool,
    #[arg(long, default_value_t = false)]
    pub ephemeral_tasks: bool,
    #[arg(long, default_value_t = false)]
    pub ephemeral_teams: bool,
    #[arg(long, default_value_t = false)]
    pub auto_root: bool,
    #[arg(long, default_value_t = false)]
    pub fullscreen: bool,
}

/// 启动稳定的 Reflect 风格表现层,启用完整的 Reflect 运行时。
pub fn run(args: TuiArgs) -> anyhow::Result<()> {
    bootstrap::run(args)
}

// 被 vendored 的 `tui_core::terminal_detection` 以原名 `terminal_detection` 重新导出,
// 供使用未限定 crate 名的 vendored 代码使用。
pub use crate::tui_core::terminal_detection;

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Debug, clap::Parser)]
    #[command(name = "reflect", disable_help_flag = true)]
    struct TestCli {
        #[arg(long)]
        prompt: Option<String>,
        #[arg(long)]
        plan_mode: bool,
    }

    #[test]
    fn cli_arguments_keep_reflect_compatibility() {
        let args = TestCli::try_parse_from(["reflect", "--plan-mode", "--prompt", "hello"])
            .expect("compatible TUI args");
        assert!(args.plan_mode);
        assert_eq!(args.prompt.as_deref(), Some("hello"));
    }

    #[test]
    fn zhuye_live_wrap_preserves_cjk_width() {
        let mut rb = RowBuilder::new(6);
        rb.push_fragment("😀😀 你好");
        assert_eq!(
            rb.rows().to_vec(),
            vec![Row {
                text: "😀😀 ".to_string(),
                explicit_break: false
            }]
        );
    }

    #[test]
    fn zhuye_take_prefix_respects_unicode_width() {
        let (prefix, suffix, width) = take_prefix_by_width("你好世界", 4);
        assert_eq!(prefix, "你好");
        assert_eq!(suffix, "世界");
        assert_eq!(width, 4);
    }

    #[test]
    fn zhuye_usable_content_width_exhausted_returns_none() {
        use crate::tui_core::width::usable_content_width;
        assert_eq!(usable_content_width(2, 2), None);
        assert_eq!(usable_content_width(5, 4), Some(1));
    }

    #[test]
    fn zhuye_adaptive_wrap_breaks_long_token() {
        use crate::tui_core::wrapping::adaptive_wrap_line;
        let line = ratatui::text::Line::from("aaaaaa");
        let lines = adaptive_wrap_line(&line, RtOptions::new(3));
        assert!(
            lines
                .iter()
                .all(|l: &ratatui::text::Line<'_>| l.width() <= 3)
        );
        assert!(lines.len() >= 2);
    }

    #[test]
    fn zhuye_markdown_render_basic() {
        let text = render_markdown_text("hello world");
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn zhuye_markdown_render_respects_width() {
        let text = render_markdown_text("a very long sentence without strong breaks");
        assert!(text.lines.iter().all(|l| l.width() <= 200));
    }

    // ── ComposerInput 集成测试 ─────────────────────────────
    // 通过 ComposerInput 使用 VT100 测试后端验证完整的 ChatComposer。
    // 它们通过直接读取屏幕缓冲区来绕过 PTY 增量渲染问题。
    use crate::tui_core::custom_terminal::Frame;
    use crate::tui_core::test_backend::VT100Backend;
    use crossterm::event::{KeyCode, KeyEventKind, KeyEventState, KeyModifiers};

    fn composer_key(ch: char) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn composer_keycode(code: KeyCode) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn composer_render(
        terminal: &mut ReflectTerminal<VT100Backend>,
        composer: &ComposerInput,
    ) -> String {
        terminal
            .draw(|frame: &mut Frame| {
                composer.render_ref(frame.area(), frame.buffer_mut());
                if let Some((x, y)) = composer.cursor_pos(frame.area()) {
                    frame.set_cursor_position((x, y));
                }
            })
            .unwrap();
        terminal.backend().vt100().screen().contents()
    }

    /// 向 composer 输入字符串。使用禁用粘贴突发的 composer,
    /// 这样字符可以立即插入,无需突发缓冲。
    fn composer_type(composer: &mut ComposerInput, text: &str) {
        for ch in text.chars() {
            composer.input(composer_key(ch));
        }
    }

    /// 创建一个禁用粘贴突发的测试 composer,以实现确定性输入。
    fn test_composer() -> ComposerInput {
        ComposerInput::new_with_config(
            "Compose new task".to_string(),
            /*disable_paste_burst*/ true,
        )
    }

    #[test]
    fn composer_empty_renders_content() {
        let backend = VT100Backend::new(60, 10);
        let mut terminal = ReflectTerminal::with_options(backend).unwrap();
        terminal.set_viewport_area(ratatui::layout::Rect::new(0, 0, 60, 10));
        let composer = test_composer();
        let text = composer_render(&mut terminal, &composer);
        assert!(!text.is_empty(), "Composer should render even when empty");
    }

    #[test]
    fn composer_typing_shows_text() {
        let backend = VT100Backend::new(60, 10);
        let mut terminal = ReflectTerminal::with_options(backend).unwrap();
        terminal.set_viewport_area(ratatui::layout::Rect::new(0, 0, 60, 10));
        let mut composer = test_composer();
        composer_type(&mut composer, "hello");
        let text = composer_render(&mut terminal, &composer);
        assert!(
            text.contains("hello"),
            "Typed text should be visible: {text}"
        );
    }

    #[test]
    fn composer_slash_popup_shows_commands() {
        let backend = VT100Backend::new(80, 20);
        let mut terminal = ReflectTerminal::with_options(backend).unwrap();
        terminal.set_viewport_area(ratatui::layout::Rect::new(0, 0, 80, 20));
        let mut composer = test_composer();
        composer_type(&mut composer, "/");
        let text = composer_render(&mut terminal, &composer);
        let has_cmd = ["model", "status", "new", "quit", "clear", "plan"]
            .iter()
            .any(|w| text.to_lowercase().contains(w));
        assert!(has_cmd, "Slash popup should show command names: {text}");
    }

    #[test]
    fn composer_slash_popup_filters() {
        let backend = VT100Backend::new(80, 20);
        let mut terminal = ReflectTerminal::with_options(backend).unwrap();
        terminal.set_viewport_area(ratatui::layout::Rect::new(0, 0, 80, 20));
        let mut composer = test_composer();
        composer_type(&mut composer, "/st");
        let text = composer_render(&mut terminal, &composer);
        assert!(
            text.to_lowercase().contains("status"),
            "Filtered popup should show status: {text}"
        );
    }

    #[test]
    fn composer_enter_submits() {
        let mut composer = test_composer();
        composer_type(&mut composer, "test");
        let action = composer.input(composer_keycode(KeyCode::Enter));
        assert!(
            matches!(action, ComposerAction::Submitted(ref t) if t == "test"),
            "Enter should submit 'test': {action:?}"
        );
    }

    #[test]
    fn composer_backspace_deletes() {
        let backend = VT100Backend::new(60, 10);
        let mut terminal = ReflectTerminal::with_options(backend).unwrap();
        terminal.set_viewport_area(ratatui::layout::Rect::new(0, 0, 60, 10));
        let mut composer = test_composer();
        composer_type(&mut composer, "hello");
        composer.input(composer_keycode(KeyCode::Backspace));
        let text = composer_render(&mut terminal, &composer);
        assert!(text.contains("hell"), "After backspace: 'hell': {text}");
        assert!(!text.contains("hello"), "Should NOT have 'hello': {text}");
    }

    #[test]
    fn composer_cursor_pos_set() {
        let mut composer = test_composer();
        for ch in "hi".chars() {
            composer.input(composer_key(ch));
        }
        let pos = composer.cursor_pos(ratatui::layout::Rect::new(0, 0, 60, 10));
        assert!(pos.is_some(), "Cursor position should be set");
    }

    #[test]
    fn composer_height_grows() {
        let mut composer = test_composer();
        let h0 = composer.desired_height(80);
        composer_type(
            &mut composer,
            "this is a very long line of text that should wrap to multiple lines in a narrow terminal",
        );
        let h1 = composer.desired_height(80);
        assert!(h1 >= h0, "Height should grow: {} -> {}", h0, h1);
    }
}
