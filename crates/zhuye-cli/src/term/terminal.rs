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

// ── F49:kitty-graphics / Sixel 图片输出协议(渐进式,不追求完整协议) ──
//
// 设计原则(诚实降级):
// - 只有检测为 Kitty 且文件确实是 PNG 时才输出 APC 转义帧;
// - Sixel 本迭代只有协议头/数据透传占位(没有 PNG→Sixel 编码器,
//   完整编解码待后续 TODO),因此输出通道对 Sixel 同样返回 None,
//   绝不把原始文件字节当 Sixel 数据倾倒进终端(乱码);
// - 检测为 None(未知终端 / dumb / CI 哑 pty)时维持现状 —— 输出图片
//   元数据行,绝不输出任何转义序列。

/// 图片输出协议能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphicsProtocol {
    /// 无图片协议(默认):图片只出元数据行,不输出任何转义。
    #[default]
    None,
    /// kitty graphics protocol(APC `ESC _ G ...;... ESC \`)。
    Kitty,
    /// Sixel(占位:透传 helper 已就绪,完整编解码 TODO)。
    Sixel,
}

/// 检测当前终端的图片输出协议(读取真实环境变量)。
pub fn detect_graphics_protocol() -> GraphicsProtocol {
    detect_graphics_protocol_with(&|key| std::env::var(key).ok())
}

/// 纯函数版协议检测(注入 env 读取闭包,供单测)。
///
/// 判定顺序(保守优先):
/// 1. `TERM == "dumb"` → None(哑终端输出转义必然乱码);
/// 2. kitty:`KITTY_WINDOW_ID` 非空 / `TERM` 含 "kitty" /
///    `TERM_PROGRAM` 含 "kitty" → Kitty;
/// 3. iTerm2 / WezTerm / tmux → Sixel「可尝试」(本迭代仅占位,
///    输出通道仍诚实降级,见 [`image_view_output`]);
/// 4. 其他 → None。
pub fn detect_graphics_protocol_with(env: &dyn Fn(&str) -> Option<String>) -> GraphicsProtocol {
    let term = env("TERM").unwrap_or_default();
    let term_lower = term.to_ascii_lowercase();
    if term_lower == "dumb" {
        return GraphicsProtocol::None;
    }
    let term_program = env("TERM_PROGRAM").unwrap_or_default();
    let program_lower = term_program.to_ascii_lowercase();

    let kitty_window_id = env("KITTY_WINDOW_ID")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    if kitty_window_id || term_lower.contains("kitty") || program_lower.contains("kitty") {
        return GraphicsProtocol::Kitty;
    }

    // iTerm2 / WezTerm:按任务规格归入「Sixel 可尝试」分组。
    if program_lower.contains("iterm")
        || program_lower.contains("wezterm")
        || env("WEZTERM_VERSION").is_some_and(|v| !v.trim().is_empty())
    {
        return GraphicsProtocol::Sixel;
    }
    // tmux:3.x 起支持透传,但 `$TMUX` 不携带版本号,无法可靠判定 ——
    // 只有 TERM 显式标注 sixel 能力时才标记为可尝试,否则诚实降级 None。
    if env("TMUX").is_some_and(|v| !v.trim().is_empty()) {
        if term_lower.contains("sixel") {
            return GraphicsProtocol::Sixel;
        }
        return GraphicsProtocol::None;
    }
    if term_lower.contains("sixel") {
        return GraphicsProtocol::Sixel;
    }
    GraphicsProtocol::None
}

/// 解析 PNG 的 IHDR 尺寸(宽, 高)。仅做头部校验,不解码像素。
pub fn png_dimensions(png: &[u8]) -> Option<(u32, u32)> {
    // 签名(8B) + chunk 头(4B len + 4B "IHDR") + 13B 数据 → 前 33 字节。
    const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if png.len() < 33 || png[..8] != PNG_SIGNATURE || &png[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let height = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    (width > 0 && height > 0).then_some((width, height))
}

/// Kitty 协议最小实现:把 PNG 原始字节组装为单个 APC 帧
/// `ESC _ G f=32,s=<w>,v=<h>,a=T;<base64> ESC \`。
///
/// - `f=32`:载荷为 PNG;`a=T`:传输并立即显示;
/// - `s`/`v`:像素尺寸(从 PNG IHDR 解析;解析失败返回 None,调用方
///   降级为元数据行 —— 宁可不显示也不发尺寸缺失的猜测帧);
/// - 尺寸小于阈值时才单帧发送(超 4MiB base64 的帧不拆块,直接放弃:
///   本迭代不实现 `m=1` 分块续传,诚实降级)。
pub fn kitty_png_frame(png: &[u8]) -> Option<String> {
    let (width, height) = png_dimensions(png)?;
    const MAX_PAYLOAD_BYTES: usize = 4 * 1024 * 1024;
    if png.len() > MAX_PAYLOAD_BYTES {
        return None;
    }
    use base64::Engine as _;
    let payload = base64::engine::general_purpose::STANDARD.encode(png);
    Some(format!(
        "\x1b_Gf=32,s={width},v={height},a=T,q=2;{payload}\x1b\\"
    ))
}

/// Sixel 透传占位:把已编码的 Sixel 数据包上协议头(`ESC P`)与终止符
/// (`ESC \`)。
///
/// TODO:完整 Sixel 编解码待后续 —— 当前仓库没有通用的 PNG→Sixel
/// 编码器(宠物模块的 [`crate::tui_core::pets::sixel`] 是小尺寸 RGBA
/// 专用编码,不承担通用图片职责),因此输出通道在 Sixel 协议下同样
/// 诚实降级,不会把未编码数据灌进终端。
pub fn sixel_passthrough(encoded_sixel: &str) -> String {
    format!("\x1bP{encoded_sixel}\x1b\\")
}

/// image_view 输出通道(协议门控 + 诚实降级)。
///
/// 返回 `Some(description)` 表示已向终端输出了内联图片,调用方应把
/// 该描述追加到 image_view 的元数据单元格里;返回 `None` 表示降级 ——
/// 调用方维持现状(只输出元数据行),此时绝不输出转义序列。
pub fn image_view_output(protocol: GraphicsProtocol, path: &std::path::Path) -> Option<String> {
    // Screen-reader 会话不输出图形(视觉内容对读屏器毫无意义)。
    if screen_reader_enabled() {
        return None;
    }
    match protocol {
        // Sixel:编解码 TODO,本迭代诚实降级(见 sixel_passthrough 文档)。
        GraphicsProtocol::None | GraphicsProtocol::Sixel => None,
        GraphicsProtocol::Kitty => {
            let png = std::fs::read(path).ok()?;
            let frame = kitty_png_frame(&png)?;
            // 直写 stdout(与 insert_history 同级:发生在 ratatui draw 之外,
            // APC 帧进原生 scrollback,后续重绘只处理文本区)。
            use std::io::Write;
            let written = std::io::stdout().write_all(frame.as_bytes()).is_ok()
                && std::io::stdout().flush().is_ok();
            if !written {
                return None;
            }
            let (w, h) = png_dimensions(&png)?;
            let kb = png.len().saturating_add(1023) / 1024;
            Some(format!(
                "kitty graphics: displayed inline ({w}x{h} px, {kb} KB)"
            ))
        }
    }
}

// ── F49:screen-reader(读屏)模式 ────────────────────────────────────
//
// 最小诚实集:开关开启时
// ① spinner 动画禁用 → 状态行输出静态「运行中」文本;
// ② 工具输出等富渲染折叠为单行摘要;
// ③ 进度条(上下文条块字形)改为纯文本百分比,动画帧省略。

/// screen-reader 模式是否启用(默认关)。
///
/// 优先级:
/// 1. 环境变量 `ZHUYE_SCREEN_READER`:`1`/`true` 强制开,`0`/`false` 强制关,
///    未知值按关(保守);
/// 2. 配置 `~/.zhuye/config.toml` 的 `[tui] screen_reader = true`;
/// 3. 缺省关。
pub fn screen_reader_enabled() -> bool {
    let env = std::env::var("ZHUYE_SCREEN_READER").ok();
    let cfg = config_tui_screen_reader();
    resolve_screen_reader_flag(env.as_deref(), cfg)
}

/// 纯函数版开关解析(供单测)。
fn resolve_screen_reader_flag(env: Option<&str>, cfg_screen_reader: Option<bool>) -> bool {
    match env.map(str::trim).filter(|s| !s.is_empty()) {
        Some("1" | "true" | "TRUE" | "True") => true,
        Some("0" | "false" | "FALSE" | "False") => false,
        // 未知值按关(保守)。
        Some(_) => false,
        None => cfg_screen_reader.unwrap_or(false),
    }
}

/// 读 `~/.zhuye/config.toml` 原始 TOML 里的 `[tui] screen_reader` 布尔值。
///
/// 引擎 schema 没有 `[tui]` 段时 serde 忽略未知键;TUI 侧直接读原始 TOML
/// 消费(与 `[tui] mouse` 同款路径)。读失败 / 未配置 → None(缺省关)。
fn config_tui_screen_reader() -> Option<bool> {
    let path = zhuye_config::default_config_path()?;
    let text = std::fs::read_to_string(path).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    value
        .get("tui")
        .and_then(|tui| tui.get("screen_reader"))
        .and_then(|m| m.as_bool())
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

#[cfg(test)]
mod graphics_protocol_tests {
    use super::{
        GraphicsProtocol, detect_graphics_protocol_with, image_view_output, kitty_png_frame,
        png_dimensions, sixel_passthrough,
    };
    use std::collections::HashMap;

    /// 便捷 env 注入表。
    fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        let map: HashMap<&str, String> =
            pairs.iter().map(|(k, v)| (*k, (*v).to_string())).collect();
        move |key| map.get(key).cloned()
    }

    /// 检测矩阵:kitty 系判定为 Kitty。
    #[test]
    fn detects_kitty_by_window_id_term_and_program() {
        let kitty = GraphicsProtocol::Kitty;
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[("KITTY_WINDOW_ID", "7")])),
            kitty
        );
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[("TERM", "xterm-kitty")])),
            kitty
        );
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[("TERM_PROGRAM", "kitty")])),
            kitty
        );
        // 空串 KITTY_WINDOW_ID 不算数(空 = 未设置)。
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[("KITTY_WINDOW_ID", "")])),
            GraphicsProtocol::None
        );
    }

    /// 检测矩阵:iTerm2 / WezTerm / tmux(+TERM sixel 标注)→ Sixel 可尝试。
    #[test]
    fn detects_sixel_for_iterm_wezterm_tmux_and_term_sixel() {
        let sixel = GraphicsProtocol::Sixel;
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[("TERM_PROGRAM", "iTerm.app")])),
            sixel
        );
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[("TERM_PROGRAM", "WezTerm")])),
            sixel
        );
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[("WEZTERM_VERSION", "20240203")])),
            sixel
        );
        // tmux:仅 TERM 显式标注 sixel 才可尝试(版本号拿不到,保守)。
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[("TMUX", "/tmp/tmux-501/default,123,0")])),
            GraphicsProtocol::None
        );
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[
                ("TMUX", "/tmp/tmux-501/default,123,0"),
                ("TERM", "tmux-256color"),
            ])),
            GraphicsProtocol::None
        );
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[
                ("TMUX", "/tmp/tmux-501/default,123,0"),
                ("TERM", "xterm-sixel"),
            ])),
            sixel
        );
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[("TERM", "xterm-sixel")])),
            sixel
        );
    }

    /// 检测矩阵:未知终端 / 无 env / dumb → None(诚实降级)。
    #[test]
    fn returns_none_for_unknown_dumb_and_empty_env() {
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[])),
            GraphicsProtocol::None
        );
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[("TERM", "xterm-256color")])),
            GraphicsProtocol::None
        );
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[("TERM", "vt100")])),
            GraphicsProtocol::None
        );
        // dumb 终端即使带 kitty 痕迹也绝不输出转义(安全优先)。
        assert_eq!(
            detect_graphics_protocol_with(&env_of(&[("TERM", "dumb"), ("TERM_PROGRAM", "kitty"),])),
            GraphicsProtocol::None
        );
    }

    /// 最小合法 PNG 头(签名 + IHDR 1x1)。
    fn tiny_png(width: u32, height: u32) -> Vec<u8> {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        png.extend_from_slice(&[0, 0, 0, 13]); // IHDR 数据长度
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&width.to_be_bytes());
        png.extend_from_slice(&height.to_be_bytes());
        png.extend_from_slice(&[8, 6, 0, 0, 0]); // bit depth / color type ...
        png.extend_from_slice(&[0, 0, 0, 0]); // CRC(占位,尺寸解析不校验)
        png
    }

    #[test]
    fn png_dimensions_parses_ihdr() {
        assert_eq!(png_dimensions(&tiny_png(3, 7)), Some((3, 7)));
        assert_eq!(png_dimensions(b"not a png"), None);
        assert_eq!(png_dimensions(&[]), None);
        // 尺寸为 0 的畸形 IHDR 拒绝。
        assert_eq!(png_dimensions(&tiny_png(0, 7)), None);
        // 截断的头拒绝。
        assert_eq!(png_dimensions(&tiny_png(3, 7)[..20]), None);
    }

    #[test]
    fn kitty_frame_assembles_apc_header_base64_and_terminator() {
        let png = tiny_png(2, 5);
        let frame = kitty_png_frame(&png).expect("frame built");
        assert!(frame.starts_with("\x1b_Gf=32,s=2,v=5,a=T,q=2;"));
        // base64 载荷以 PNG 签名开头:0x89PNG → "iVBORw0KGgo"。
        assert!(frame.contains("iVBORw0KGgo"));
        assert!(frame.ends_with("\x1b\\"));
        // 载荷里不允许再出现 ESC(APC 帧内嵌转义会破坏协议)。
        let payload = frame
            .strip_prefix("\x1b_Gf=32,s=2,v=5,a=T,q=2;")
            .unwrap()
            .strip_suffix("\x1b\\")
            .unwrap();
        assert!(!payload.contains('\x1b'), "base64 payload must be clean");
        // 非法输入 → None(诚实降级)。
        assert_eq!(kitty_png_frame(b"garbage"), None);
    }

    #[test]
    fn sixel_passthrough_wraps_protocol_header_and_terminator() {
        assert_eq!(sixel_passthrough("9;1;0q#0"), "\x1bP9;1;0q#0\x1b\\");
    }

    /// 输出通道门控:协议 None / Sixel / 路径不存在 / 非 PNG 一律 None,
    /// 绝不输出转义(诚实降级为元数据行)。
    #[test]
    fn image_view_output_gate_never_emits_without_kitty_and_png() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("img.png");
        std::fs::write(&path, tiny_png(4, 9)).unwrap();

        // None 协议:即使文件是合法 PNG 也不输出。
        assert_eq!(image_view_output(GraphicsProtocol::None, &path), None);
        // Sixel:编解码 TODO,本迭代同样降级。
        assert_eq!(image_view_output(GraphicsProtocol::Sixel, &path), None);
        // Kitty + 不存在的文件 → None。
        let missing = dir.path().join("missing.png");
        assert_eq!(image_view_output(GraphicsProtocol::Kitty, &missing), None);
        // Kitty + 非 PNG 内容 → None(不把乱数据当图片发)。
        let junk = dir.path().join("junk.png");
        std::fs::write(&junk, b"hello").unwrap();
        assert_eq!(image_view_output(GraphicsProtocol::Kitty, &junk), None);
    }
}

#[cfg(test)]
mod screen_reader_flag_tests {
    use super::resolve_screen_reader_flag;

    /// 默认关(与 mouse 同款硬要求:不改变现有用户行为)。
    #[test]
    fn screen_reader_defaults_off() {
        assert!(!resolve_screen_reader_flag(None, None));
        assert!(!resolve_screen_reader_flag(None, Some(false)));
    }

    /// env 显式开/关优先于配置。
    #[test]
    fn screen_reader_env_overrides_config() {
        assert!(resolve_screen_reader_flag(Some("1"), None));
        assert!(resolve_screen_reader_flag(Some("true"), Some(false)));
        assert!(!resolve_screen_reader_flag(Some("0"), Some(true)));
        assert!(!resolve_screen_reader_flag(Some("false"), Some(true)));
    }

    /// 配置 `[tui] screen_reader = true` 单独生效;未知值按关。
    #[test]
    fn screen_reader_config_enables_and_unknown_env_is_off() {
        assert!(resolve_screen_reader_flag(None, Some(true)));
        assert!(!resolve_screen_reader_flag(Some("yes"), None));
        assert!(!resolve_screen_reader_flag(Some("on"), None));
        assert!(
            resolve_screen_reader_flag(Some(""), Some(true)),
            "空串=未设置"
        );
        assert!(
            resolve_screen_reader_flag(Some(" 1 "), None),
            "值两侧空白被 trim"
        );
    }
}
