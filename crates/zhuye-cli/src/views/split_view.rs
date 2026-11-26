//! F49:`/split` 并行分屏视图 —— **快照分屏**(按需刷新,非实时双流)。
//!
//! 数据源:
//! - **状态**(role / running / current_tool / iteration / tokens):来自
//!   `SubagentStatus` 事件的 CLI 消费点 —— 引擎 `Op::QuerySubagents` 应答
//!   (`SubagentStatusSnapshot`)经 [`crate::adapter`] 投影为
//!   `UiEventKind::SubagentStatusUpdate`,在 `apply_event` 里 upsert 进
//!   [`SplitFeedRegistry`](自维护的 subagent_registry 快照,即
//!   "agentStore 等价物")。
//! - **最近输出行**:来自 `SubagentProgress` 推送事件(child_id + role + text),
//!   每个子代理保留一个滚动缓冲(最近 [`SPLIT_FEED_LINE_CAP`] 行)。
//!
//! 诚实标注:`/split` 渲染的是**按下命令那一刻**的快照 —— 最近 2 个活跃
//! 子代理各自的最近输出行,左右两栏拼进 transcript pager。不跟随事件实时
//! 刷新(再按一次 `/split` 重新快照),也不是真正的双流并行渲染。

use crate::adapter::SubagentView;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// 每个子代理保留的最近输出行数上限(滚动缓冲)。
pub(crate) const SPLIT_FEED_LINE_CAP: usize = 24;

/// 注册表最多跟踪的子代理数(防长会话无界增长;超限淘汰最旧的)。
pub(crate) const SPLIT_FEEDS_CAP: usize = 8;

/// `/split` 快照每个子代理展示的最近输出行数。
pub(crate) const SPLIT_SNAPSHOT_LINES_PER_FEED: usize = 10;

/// 单个子代理的最近输出快照(自维护的 subagent_registry 条目)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SubagentFeed {
    pub id: String,
    pub role: String,
    pub running: bool,
    pub current_tool: Option<String>,
    pub iteration: u32,
    pub total_tokens: u64,
    /// 最近输出行(滚动缓冲,新行在尾部,上限 [`SPLIT_FEED_LINE_CAP`])。
    pub lines: Vec<String>,
}

impl SubagentFeed {
    fn new(id: &str, role: &str) -> Self {
        Self {
            id: id.to_string(),
            role: role.to_string(),
            ..Default::default()
        }
    }

    /// 追加一行输出,超出上限时丢弃最旧行。
    pub(crate) fn push_line(&mut self, line: String) {
        if line.is_empty() {
            return;
        }
        self.lines.push(line);
        if self.lines.len() > SPLIT_FEED_LINE_CAP {
            let drop = self.lines.len() - SPLIT_FEED_LINE_CAP;
            self.lines.drain(..drop);
        }
    }

    /// 状态栏摘要:`explorer · Bash · 3 iters · 1.2k tok`。
    fn status_summary(&self) -> String {
        let state = if self.running { "running" } else { "done" };
        let tool = self.current_tool.as_deref().unwrap_or("-");
        format!(
            "{state} · {tool} · {} iters · {} tok",
            self.iteration, self.total_tokens
        )
    }
}

/// 子代理输出注册表(subagent_registry 快照的 CLI 侧等价物)。
#[derive(Debug, Clone, Default)]
pub(crate) struct SplitFeedRegistry {
    feeds: Vec<SubagentFeed>,
}

impl SplitFeedRegistry {
    /// 记录一条子代理进度输出(child_id 未知则新建条目)。
    pub(crate) fn push_progress(&mut self, child_id: &str, role: &str, text: &str) {
        if child_id.is_empty() {
            return;
        }
        let feed = match self.feeds.iter_mut().find(|f| f.id == child_id) {
            Some(feed) => feed,
            None => {
                self.feeds.push(SubagentFeed::new(child_id, role));
                self.cap_feeds();
                self.feeds
                    .iter_mut()
                    .find(|f| f.id == child_id)
                    .expect("feed just inserted")
            }
        };
        if !role.is_empty() {
            feed.role = role.to_string();
        }
        feed.push_line(text.to_string());
    }

    /// 应用 QuerySubagents 快照投影(upsert 状态;不触碰输出缓冲)。
    pub(crate) fn apply_status(&mut self, children: &[SubagentView]) {
        for c in children {
            let feed = match self.feeds.iter_mut().find(|f| f.id == c.id) {
                Some(feed) => feed,
                None => {
                    self.feeds.push(SubagentFeed::new(&c.id, &c.role));
                    self.cap_feeds();
                    self.feeds
                        .iter_mut()
                        .find(|f| f.id == c.id)
                        .expect("feed just inserted")
                }
            };
            feed.role = c.role.clone();
            feed.running = c.running;
            feed.current_tool = c.current_tool.clone();
            feed.iteration = c.iteration;
            feed.total_tokens = c.total_tokens;
        }
    }

    /// 最近 `n` 个子代理(默认 2),快照渲染取用。
    ///
    /// 「最近」的定义:**运行中优先**,其余按注册先后倒序(越新越靠前)。
    /// running 与否都可见(刚完成的子代理最后一次输出也值得看),但
    /// running 的排在前面(左栏)。
    pub(crate) fn recent_active(&self, n: usize) -> Vec<SubagentFeed> {
        // (插入序, 快照克隆) —— running 优先;同组内插入序倒序(越新越靠前)。
        let mut indexed: Vec<(usize, SubagentFeed)> = self
            .feeds
            .iter()
            .enumerate()
            .map(|(i, f)| (i, f.clone()))
            .collect();
        indexed.sort_by(|a, b| b.1.running.cmp(&a.1.running).then(b.0.cmp(&a.0)));
        indexed.truncate(n);
        indexed.into_iter().map(|(_, f)| f).collect()
    }

    /// 注册表条目数(测试用)。
    pub(crate) fn len(&self) -> usize {
        self.feeds.len()
    }

    /// 超限淘汰最旧条目(保序截断到 [`SPLIT_FEEDS_CAP`])。
    fn cap_feeds(&mut self) {
        if self.feeds.len() > SPLIT_FEEDS_CAP {
            let drop = self.feeds.len() - SPLIT_FEEDS_CAP;
            self.feeds.drain(..drop);
        }
    }
}

/// 单列宽度截断:按字符计(CJK 安全的粗略近似 —— ratatui 半宽渲染时
/// 每列各自 wrap,这里只防溢出),超出加 `…`。
fn truncate_to_width(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= width {
        return s.to_string();
    }
    let mut out: String = chars.iter().take(width.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// 把快照渲染成左右两栏行(供 TranscriptPager)。
///
/// - `width`:终端宽度(减去边框余量前);每栏 = `width/2 - gutter`。
/// - 每个子代理栏:标题行(role + 状态摘要)+ 最近
///   [`SPLIT_SNAPSHOT_LINES_PER_FEED`] 行输出(每行截断到半宽)。
/// - 没有子代理 → 单行说明(诚实告知快照为空)。
/// - 只有一个子代理 → 右栏为空占位。
pub(crate) fn render_snapshot_lines(feeds: &[SubagentFeed], width: usize) -> Vec<Line<'static>> {
    let gutter = " │ ";
    let half = (width.saturating_sub(gutter.chars().count()) / 2).max(1);
    let dim = Style::default().fg(Color::DarkGray);
    let title_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let mut header: Vec<Span<'static>> = vec![Span::styled(
        "Subagent split view (SNAPSHOT — rerun /split to refresh, /split off to close)",
        title_style,
    )];
    header.push(Span::styled(
        "  [快照分屏:非实时双流]",
        Style::default().fg(Color::DarkGray),
    ));
    let mut lines = vec![Line::from(header)];

    if feeds.is_empty() {
        lines.push(Line::from(Span::styled(
            "(no subagent activity recorded in this session)",
            dim,
        )));
        return lines;
    }

    // 每栏预渲染为定宽字符串行(截断到半宽),再逐行左右拼接。
    let mut columns: [Vec<String>; 2] = [Vec::new(), Vec::new()];
    for (col, feed) in feeds.iter().take(2).enumerate() {
        columns[col].push(truncate_to_width(
            &format!("── [{}] {}", feed.role, feed.status_summary()),
            half,
        ));
        let start = feed
            .lines
            .len()
            .saturating_sub(SPLIT_SNAPSHOT_LINES_PER_FEED);
        for line in &feed.lines[start..] {
            columns[col].push(truncate_to_width(&format!("  {line}"), half));
        }
        if feed.lines.is_empty() {
            columns[col].push(truncate_to_width("  (no output yet)", half));
        }
    }

    let rows = columns[0].len().max(columns[1].len());
    for row in 0..rows {
        let left = columns[0].get(row).map(String::as_str).unwrap_or("");
        let right = columns[1].get(row).map(String::as_str).unwrap_or("");
        // 左栏 pad 到半宽,保证右栏起始列对齐。
        let left_pad = {
            let len = left.chars().count();
            " ".repeat(half.saturating_sub(len))
        };
        lines.push(Line::from(Span::from(format!(
            "{left}{left_pad}{gutter}{right}"
        ))));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(id: &str, role: &str, running: bool, lines: &[&str]) -> SubagentFeed {
        SubagentFeed {
            id: id.to_string(),
            role: role.to_string(),
            running,
            current_tool: None,
            iteration: 2,
            total_tokens: 1200,
            lines: lines.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// 进度输出按 child_id 聚合,未知 child 新建条目;最近输出排序 =
    /// running 优先,同组按注册倒序(越新越靠前)。
    #[test]
    fn push_progress_aggregates_by_child_id() {
        let mut reg = SplitFeedRegistry::default();
        reg.push_progress("c1", "explorer", "reading src/lib.rs");
        reg.push_progress("c2", "tester", "running cargo test");
        reg.push_progress("c1", "explorer", "found the bug");
        assert_eq!(reg.len(), 2);
        let feeds = reg.recent_active(2);
        // 无 running → 越新越靠前(c2 最后注册,排第一)。
        assert_eq!(feeds[0].id, "c2");
        let c1 = feeds.iter().find(|f| f.id == "c1").unwrap();
        assert_eq!(c1.lines, vec!["reading src/lib.rs", "found the bug"]);
    }

    /// 输出缓冲滚动:超过上限丢最旧行。
    #[test]
    fn push_line_caps_to_recent_lines() {
        let mut f = SubagentFeed::new("c", "r");
        for i in 0..(SPLIT_FEED_LINE_CAP + 5) {
            f.push_line(format!("line{i}"));
        }
        assert_eq!(f.lines.len(), SPLIT_FEED_LINE_CAP);
        assert_eq!(f.lines[0], format!("line{}", 5));
        assert_eq!(
            f.lines[SPLIT_FEED_LINE_CAP - 1],
            format!("line{}", SPLIT_FEED_LINE_CAP + 4)
        );
    }

    /// QuerySubagents 快照 upsert 状态、不触碰输出缓冲;running 排前。
    #[test]
    fn apply_status_upserts_and_recent_active_prefers_running() {
        let mut reg = SplitFeedRegistry::default();
        reg.push_progress("c1", "explorer", "old line");
        reg.apply_status(&[
            SubagentView {
                id: "c1".into(),
                role: "explorer".into(),
                running: false,
                current_tool: None,
                iteration: 1,
                total_tokens: 10,
            },
            SubagentView {
                id: "c2".into(),
                role: "tester".into(),
                running: true,
                current_tool: Some("Bash".into()),
                iteration: 3,
                total_tokens: 99,
            },
        ]);
        let feeds = reg.recent_active(2);
        assert!(feeds[0].running, "running feed first");
        assert_eq!(feeds[0].id, "c2");
        assert_eq!(
            feeds[0].status_summary(),
            "running · Bash · 3 iters · 99 tok"
        );
        // 输出缓冲不被状态快照覆盖。
        let c1 = reg
            .recent_active(8)
            .into_iter()
            .find(|f| f.id == "c1")
            .unwrap();
        assert_eq!(c1.lines, vec!["old line"]);
    }

    /// 快照渲染:两栏并排,每行截断到半宽。
    #[test]
    fn render_snapshot_two_columns_with_truncation() {
        let feeds = vec![
            feed(
                "c1",
                "explorer",
                true,
                &["reading a very long line that must be truncated to half width"],
            ),
            feed("c2", "tester", true, &["testing"]),
        ];
        let lines = render_snapshot_lines(&feeds, 80);
        // 标题 + 左右两栏行。
        let joined: Vec<String> = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.clone().into_owned())
                    .collect()
            })
            .collect();
        assert!(
            joined[0].contains("SNAPSHOT"),
            "snapshot label: {}",
            joined[0]
        );
        assert!(
            joined.iter().any(|l| l.contains("[explorer]")),
            "left col role"
        );
        assert!(
            joined.iter().any(|l| l.contains("[tester]")),
            "right col role"
        );
        assert!(
            joined.iter().any(|l| l.contains(" │ ")),
            "gutter: {joined:?}"
        );
        // 半宽截断:两栏拼接后的行不超过 width(80)。标题行是通栏,不在此列。
        for l in joined.iter().skip(1) {
            assert!(l.chars().count() <= 80, "line overflow: {l}");
        }
        // 长行被截断到半宽(40 - 3 gutter ≈ 38;按字符数计,UTF-8 字节序不同)。
        let truncated = joined
            .iter()
            .find(|l| l.contains('…'))
            .expect("long line truncated");
        let half_idx = truncated.find(" │ ").unwrap();
        let left_chars = truncated[..half_idx].chars().count();
        assert!(left_chars <= 40, "left column kept within half width");
    }

    /// 快照渲染:无子代理 / 单子代理的诚实占位。
    #[test]
    fn render_snapshot_handles_empty_and_single_feed() {
        let empty = render_snapshot_lines(&[], 80);
        let s: String = empty[1]
            .spans
            .iter()
            .map(|s| s.content.clone().into_owned())
            .collect();
        assert!(s.contains("no subagent activity"), "empty placeholder: {s}");

        let single = render_snapshot_lines(&[feed("c1", "solo", true, &["hi"])], 80);
        let s: String = single
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.clone().into_owned())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(s.contains("[solo]"), "single feed: {s}");
    }

    #[test]
    fn truncate_to_width_appends_ellipsis() {
        assert_eq!(truncate_to_width("short", 10), "short");
        assert_eq!(truncate_to_width("abcdefghij", 5), "abcd…");
        assert_eq!(truncate_to_width("任意宽度", 3), "任意…");
        assert_eq!(truncate_to_width("abc", 0), "");
    }
}
