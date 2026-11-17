//! 基于 Reflect 的 `custom_terminal::Terminal` 构建的终端生命周期 RAII。
//!
//! 镜像 Reflect 的 "inline viewport"（行内视口）模型:我们停留在 **主屏**
//! (不使用备用屏),以保留终端的原生 scrollback。已提交
//! 的历史行通过 `insert_history_lines` 推入原生 scrollback,
//! 用户可以使用终端自身的 scrollback 控件(鼠标滚轮、PgUp 等)查看。
//! 每帧只将活动区、composer 和状态行渲染到屏幕底部
//! 锚定的小视口。
//!
//! 启用 Bracketed paste(`?2004h`),使粘贴文本以 `Event::Paste` 形式到达。
//! 故意**不**启用 Alternate scroll(`?1007h`):启用后,
//! 终端会把滚轮/触控板滚动转换为 Up/Down 方向键事件,
//! 而 composer 会用它进行输入历史导航。不启用时,
//! 终端直接滚动其原生 scrollback ——这正是我们
//! 审查已提交历史时想要的行为。

use crate::terminal_probe::{self, DefaultColors};
use crate::tui_core::custom_terminal::Terminal as ReflectTerminal;
use crate::tui_core::terminal_palette;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::{Color as CrosstermColor, query_background_color, query_foreground_color};
use ratatui::backend::CrosstermBackend;
use std::io::{self, Stdout};

pub type Backend = CrosstermBackend<Stdout>;
pub type Terminal = ReflectTerminal<Backend>;

pub struct TerminalGuard {
    terminal: Terminal,
    /// 是否已进入备用屏（Ctrl+T transcript overlay）。
    alt_screen: bool,
    /// 是否全屏模式：启动即进备用屏且不再退回主屏（overlay 与之共享同一屏）。
    fullscreen: bool,
    /// F49:本会话是否启用了鼠标捕获（由 [`mouse_capture_enabled`] 决定）。
    /// 记下来供 Drop 精确恢复 —— 未启用时不该多发 DisableMouseCapture。
    mouse: bool,
}

impl TerminalGuard {
    /// 初始化终端。`fullscreen=true` 时直接进入备用屏（替代 inline viewport 的
    /// 主屏 + 原生 scrollback 模型，避免 shrink 时空行污染 scrollback）。
    pub fn enter(fullscreen: bool) -> anyhow::Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        // 主屏 + bracketed paste；不启 alternate scroll（与 Reflect 主界面一致）。
        crossterm::execute!(stdout, crossterm::event::EnableBracketedPaste)?;

        // F49:鼠标捕获是 opt-in（默认关,防止改变现有用户的滚轮行为）。
        // 开启后滚轮事件以 Event::Mouse 到达,tui/mod.rs 的 handle_event
        // 负责把它转成翻页/审批点击。
        let mouse = mouse_capture_enabled();
        if mouse {
            crossterm::execute!(stdout, crossterm::event::EnableMouseCapture)?;
            install_mouse_panic_hook();
        }

        // 启动时探测默认前景/背景色，供 user_message_style / TextArea 可见性使用。
        probe_and_cache_default_colors();

        let terminal = Terminal::with_options(CrosstermBackend::new(stdout))?;
        // 预热 palette 缓存（若 probe 已写入则直接命中）。
        let _ = terminal_palette::default_colors();
        let mut guard = Self {
            terminal,
            alt_screen: false,
            fullscreen,
            mouse,
        };
        if fullscreen {
            // 全屏模式启动即进备用屏，后续 overlay 共享此屏。
            let _ = guard.enter_alt_screen();
        }
        Ok(guard)
    }

    pub fn terminal(&mut self) -> &mut Terminal {
        &mut self.terminal
    }

    /// 进入备用屏（transcript overlay）。启用 alternate scroll 以便在 overlay 内滚轮翻页。
    pub fn enter_alt_screen(&mut self) -> io::Result<()> {
        if self.alt_screen {
            return Ok(());
        }
        crossterm::execute!(
            self.terminal.backend_mut(),
            crossterm::terminal::EnterAlternateScreen,
        )?;
        // Overlay 内滚轮 → 方向键，供 pager 消费。
        let _ = write_osc(self.terminal.backend_mut(), "\x1b[?1007h");
        self.alt_screen = true;
        let size = self.terminal.size()?;
        self.terminal
            .set_viewport_area(ratatui::layout::Rect::new(0, 0, size.width, size.height));
        self.terminal.clear()?;
        Ok(())
    }

    /// 离开备用屏，回到主屏 inline viewport。
    /// 真正离开备用屏（全屏模式退出 TUI 时也调用此内部方法恢复主屏）。
    fn leave_alt_screen_inner(&mut self) -> io::Result<()> {
        if !self.alt_screen {
            return Ok(());
        }
        let _ = write_osc(self.terminal.backend_mut(), "\x1b[?1007l");
        crossterm::execute!(
            self.terminal.backend_mut(),
            crossterm::terminal::LeaveAlternateScreen,
        )?;
        self.alt_screen = false;
        Ok(())
    }

    /// 挂起 TUI 运行外部程序（如 `$EDITOR`）：无论 inline/fullscreen 都真正
    /// 离开备用屏。
    ///
    /// [`Self::leave_alt_screen`] 在 fullscreen 下会短路（overlay 与全屏主界面
    /// 共享同一屏），而外部程序需要真实主屏，否则编辑器会跑在 alt-screen 上
    /// 被 TUI 的重绘覆盖、输入也落错屏。
    pub fn suspend_alt_screen(&mut self) -> io::Result<()> {
        self.leave_alt_screen_inner()
    }

    pub fn leave_alt_screen(&mut self) -> io::Result<()> {
        // 全屏模式下不退出备用屏：overlay 与之共享同一屏，关闭 overlay 只是清除
        // 状态并回到全屏主界面，不应真正 LeaveAlternateScreen。
        if self.fullscreen {
            return Ok(());
        }
        self.leave_alt_screen_inner()
    }

    pub fn is_alt_screen(&self) -> bool {
        self.alt_screen
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // 退出时始终真正离开备用屏（全屏模式也恢复主屏，避免终端残留）。
        let _ = self.leave_alt_screen_inner();
        let _ = self.terminal.show_cursor();
        // F49:只在本会话确实启用过鼠标捕获时才发恢复序列(幂等,但少写
        // 无意义转义)。panic 路径另有 install_mouse_panic_hook 兜底。
        if self.mouse {
            let _ = crossterm::execute!(
                self.terminal.backend_mut(),
                crossterm::event::DisableMouseCapture,
            );
        }
        let _ = crossterm::execute!(
            self.terminal.backend_mut(),
            crossterm::event::DisableBracketedPaste,
        );
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// F49:鼠标捕获是否启用（默认关 —— 硬要求,防止改变现有用户的滚轮行为）。
///
/// 优先级：
/// 1. 环境变量 `ZHUYE_TUI_MOUSE`：`1`/`true` 强制开，`0`/`false` 强制关；
/// 2. 配置 `~/.zhuye/config.toml` 的 `[tui] mouse = true`；
/// 3. 缺省关。
///
/// 引擎 schema（zhuye-config `ReflectConfig`）还没有 `[tui]` 段 —— serde 忽略
/// 未知键,所以用户写了该段不会破坏加载;TUI 侧直接读原始 TOML 消费,等引擎
/// schema 收编该段后可平滑迁移。
pub fn mouse_capture_enabled() -> bool {
    let env = std::env::var("ZHUYE_TUI_MOUSE").ok();
    let cfg = config_tui_mouse();
    resolve_mouse_flag(env.as_deref(), cfg)
}

/// 纯函数版开关解析（供单测）。
fn resolve_mouse_flag(env: Option<&str>, cfg_mouse: Option<bool>) -> bool {
    match env.map(str::trim).filter(|s| !s.is_empty()) {
        Some("1" | "true" | "TRUE" | "True") => true,
        Some("0" | "false" | "FALSE" | "False") => false,
        // 未知值按关（保守）。
        Some(_) => false,
        None => cfg_mouse.unwrap_or(false),
    }
}

/// 读 `~/.zhuye/config.toml` 原始 TOML 里的 `[tui] mouse` 布尔值。
///
/// 读失败 / 未配置 / 类型不对 → `None`（调用方按缺省关处理）。文件不存在是
/// 常态（首次运行）,静默返回 None,不产生任何告警。
fn config_tui_mouse() -> Option<bool> {
    let path = zhuye_config::default_config_path()?;
    let text = std::fs::read_to_string(path).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    value
        .get("tui")
        .and_then(|tui| tui.get("mouse"))
        .and_then(|m| m.as_bool())
}

/// F49:安装 panic hook,崩溃路径也恢复终端（禁用鼠标捕获 + 显示光标）。
///
/// 链式调用先前 hook：先恢复终端,再把 panic 信息交给原有处理（标准错误
/// 输出 / Sentry 等既有行为不被吞掉）。多次进入 TUI 时重复安装也无害 ——
/// 每次都把"当前 hook"包进新 hook,形成链,恢复序列幂等。
fn install_mouse_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crossterm::execute!(
            io::stdout(),
            crossterm::event::DisableMouseCapture,
            crossterm::cursor::Show
        );
        prev(info);
    }));
}

/// 写入 OSC 序列(如 `ESC]0;titleBEL`)。
fn write_osc(w: &mut impl io::Write, seq: &str) -> io::Result<()> {
    w.write_all(seq.as_bytes())?;
    w.flush()
}

/// 设置终端标题(OSC 0 `ESC]0;titleBEL`)。
///
/// 对齐老 TUI `terminal_title.rs` 的行为:对特殊字符做清洗(去除 ESC +
/// 控制字符,保留可打印字符),截断到 80 字符,避免溢出导致终端异常。
///
/// 在 `tui/mod.rs` 启动期 + 状态变化(busy 切换/回合完成/审批)时调用,
/// 让终端标签页始终显示当前上下文(`⠹ model · git:(branch) · ...`)。
/// 对不支持 OSC 0 的终端(iTerm/tmux/Kitty 均支持)是静默无操作。
pub fn set_terminal_title(title: &str) {
    // 对特殊字符做简单清洗:只保留可打印字符,去除 ESC。
    let sanitized: String = title
        .chars()
        .filter(|c| u32::from(*c) >= 0x20)
        .filter(|c| *c != '\x1b')
        .take(80)
        .collect();
    let _ = write_osc(&mut std::io::stdout(), &format!("\x1b]0;{}\x07", sanitized));
}

/// 通过 crossterm OSC 查询写入 palette，并同步到 terminal_probe 形状。
fn probe_and_cache_default_colors() {
    if !should_probe_default_colors() {
        return;
    }
    let fg = query_foreground_color()
        .ok()
        .flatten()
        .and_then(crossterm_rgb);
    let bg = query_background_color()
        .ok()
        .flatten()
        .and_then(crossterm_rgb);
    let colors = fg.zip(bg).map(|(fg, bg)| DefaultColors { fg, bg });
    terminal_palette::set_default_colors_from_startup_probe(colors);
    // 若 OSC 不可用，尝试 terminal_probe stub 路径（当前恒为 None，保留扩展点）。
    if colors.is_none() {
        if let Ok(Some(c)) = terminal_probe::default_colors(terminal_probe::DEFAULT_TIMEOUT) {
            terminal_palette::set_default_colors_from_startup_probe(Some(c));
        }
    }
}

/// 启动期是否探测终端默认前景/背景色（OSC 10/11 查询）。
///
/// crossterm fork 的颜色查询对无响应终端**每次固定等 2s 超时**，两个 slot
/// 共 4s —— macOS Terminal.app、无透传的 tmux/screen、CI/测试用的哑 pty
/// 都不应答，直接表现为 TUI 启动黑屏数秒（叠加 CPR 光标查询可达 6s）。
/// 因此只在已知会应答 OSC 颜色查询的终端上探测；探测失败本就有安全的
/// 样式兜底，跳过只是少了 fg/bg 精确配色。
///
/// `ZHUYE_CLI_COLOR_PROBE=0` 强制关闭，`=1` 强制开启。
fn should_probe_default_colors() -> bool {
    if let Ok(v) = std::env::var("ZHUYE_CLI_COLOR_PROBE") {
        return v != "0";
    }
    use crate::tui_core::terminal_detection::{TerminalName, terminal_info};
    matches!(
        terminal_info().name,
        TerminalName::ITerm2
            | TerminalName::WezTerm
            | TerminalName::Kitty
            | TerminalName::Alacritty
            | TerminalName::Ghostty
            | TerminalName::Rio
            | TerminalName::Foot
            | TerminalName::Konsole
            | TerminalName::Contour
            | TerminalName::Warp
            | TerminalName::VSCode
            | TerminalName::VSCodeInsiders
            | TerminalName::VSCodium
            | TerminalName::Windsurf
            | TerminalName::Cursor
    )
}

fn crossterm_rgb(color: CrosstermColor) -> Option<(u8, u8, u8)> {
    match color {
        CrosstermColor::Rgb { r, g, b } => Some((r, g, b)),
        _ => None,
    }
}

pub fn poll_event(timeout: std::time::Duration) -> anyhow::Result<Option<Event>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    Ok(Some(event::read()?))
}

pub fn is_exit_key(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL)
}

#[cfg(test)]
mod terminal_title_tests {

    #[test]
    fn terminal_title_sanitizes_esc_and_control_chars() {
        // 测试 sanitized 字符串逻辑:ESC 和控制字符被移除。
        let title = "Reflect · test\x1b\x00\x1f";
        let sanitized: String = title
            .chars()
            .filter(|c| u32::from(*c) >= 0x20)
            .filter(|c| *c != '\x1b')
            .take(80)
            .collect();
        assert!(!sanitized.contains('\x1b'), "ESC should be removed");
        assert!(!sanitized.contains('\x00'), "NUL should be removed");
        assert!(
            !sanitized.contains('\x1f'),
            "Control char should be removed"
        );
        assert_eq!(sanitized, "Reflect · test");
    }

    #[test]
    fn terminal_title_truncates_long_title() {
        let title = "a".repeat(100);
        let sanitized: String = title
            .chars()
            .filter(|c| u32::from(*c) >= 0x20)
            .filter(|c| *c != '\x1b')
            .take(80)
            .collect();
        assert_eq!(sanitized.len(), 80, "should be truncated to 80 chars");
    }
}

#[cfg(test)]
mod mouse_flag_tests {
    use super::resolve_mouse_flag;

    /// F49:鼠标捕获默认必须关 —— 硬要求,防止改变现有用户的滚轮行为。
    #[test]
    fn mouse_flag_defaults_off() {
        assert!(!resolve_mouse_flag(None, None));
        assert!(!resolve_mouse_flag(None, Some(false)));
    }

    /// 配置 `[tui] mouse = true` 单独生效;env 未设置时是唯一事实来源。
    #[test]
    fn mouse_flag_config_enables() {
        assert!(resolve_mouse_flag(None, Some(true)));
        // env 显式关可以压过配置开(逃生舱)。
        assert!(!resolve_mouse_flag(Some("0"), Some(true)));
        assert!(!resolve_mouse_flag(Some("false"), Some(true)));
    }

    /// env 显式开/关优先于配置。
    #[test]
    fn mouse_flag_env_overrides_config() {
        assert!(resolve_mouse_flag(Some("1"), Some(false)));
        assert!(resolve_mouse_flag(Some("true"), None));
        assert!(resolve_mouse_flag(Some("1"), Some(true)));
    }

    /// 未知 env 值按关(保守),空串视为未设置(回落配置)。
    #[test]
    fn mouse_flag_unknown_env_values_are_off() {
        assert!(!resolve_mouse_flag(Some("yes"), None));
        assert!(!resolve_mouse_flag(Some("on"), None));
        assert!(resolve_mouse_flag(Some(""), Some(true)), "空串=未设置");
        assert!(resolve_mouse_flag(Some(" 1 "), None), "值两侧空白被 trim");
    }
}
