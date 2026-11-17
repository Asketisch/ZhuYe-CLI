//! 补丁摘要与图片工具 transcript 辅助函数。

use super::*;
use crate::utils_path_uri::LegacyAppPathString;

#[derive(Debug)]
pub(crate) struct PatchHistoryCell {
    changes: HashMap<PathBuf, FileChange>,
    cwd: PathBuf,
}

impl HistoryCell for PatchHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        create_diff_summary(&self.changes, &self.cwd, width as usize)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        plain_lines(create_diff_summary(
            &self.changes,
            &self.cwd,
            RAW_DIFF_SUMMARY_WIDTH,
        ))
    }
}
/// 创建列出拟打补丁文件级摘要的 `PendingPatch` 单元格。
/// 摘要行应已格式化好（例如 "A path/to/file.rs"）。
pub(crate) fn new_patch_event(
    changes: HashMap<PathBuf, FileChange>,
    cwd: &Path,
) -> PatchHistoryCell {
    PatchHistoryCell {
        changes,
        cwd: cwd.to_path_buf(),
    }
}

pub(crate) fn new_patch_apply_failure(stderr: String) -> PlainHistoryCell {
    let mut lines: Vec<Line<'static>> = Vec::new();

    // 失败标题
    lines.push(Line::from("✘ Failed to apply patch".magenta().bold()));

    if !stderr.trim().is_empty() {
        let output = output_lines(
            Some(&CommandOutput::new(/*exit_code*/ 1, stderr)),
            OutputLinesParams {
                line_limit: TOOL_CALL_MAX_LINES,
                only_err: true,
                include_angle_pipe: true,
                include_prefix: true,
            },
        );
        lines.extend(output.lines);
    }

    PlainHistoryCell { lines }
}

pub(crate) fn new_view_image_tool_call(path: LegacyAppPathString, cwd: &Path) -> PlainHistoryCell {
    new_view_image_tool_call_with_note(path, cwd, None)
}

/// F49:image_view 单元格(可选携带图形通道说明行)。
///
/// `note` 来自 [`crate::terminal::image_view_output`]:仅当终端被检测为
/// kitty graphics 协议且文件是 PNG 时才有值(如
/// `"kitty graphics: displayed inline (WxH px, N KB)"`)。其余协议
/// (Sixel 编解码 TODO / None / screen-reader 会话)诚实降级为 None,
/// 单元格维持原样 —— 只输出元数据行,绝不输出转义序列。
pub(crate) fn new_view_image_tool_call_with_note(
    path: LegacyAppPathString,
    cwd: &Path,
    note: Option<String>,
) -> PlainHistoryCell {
    let display_path = path
        .to_inferred_path_uri()
        .map(|path| display_path_for(path.to_abs_path().as_path(), cwd))
        .unwrap_or_else(|| path.into_string());

    let mut lines: Vec<Line<'static>> = vec![
        vec!["• ".dim(), "Viewed Image".bold()].into(),
        vec!["  └ ".dim(), display_path.dim()].into(),
    ];
    if let Some(note) = note {
        lines.push(vec!["  └ ".dim(), note.dim()].into());
    }

    PlainHistoryCell { lines }
}

pub(crate) fn new_image_generation_call(
    call_id: String,
    status: &str,
    revised_prompt: Option<String>,
    saved_path: Option<AbsolutePathBuf>,
) -> PlainHistoryCell {
    let detail = revised_prompt.unwrap_or(call_id);
    let heading = if status == "failed" {
        vec!["✗ ".red().bold(), "Image generation failed".bold()].into()
    } else {
        vec!["• ".dim(), "Generated Image:".bold()].into()
    };
    let mut lines: Vec<Line<'static>> = vec![heading, vec!["  └ ".dim(), detail.dim()].into()];
    if let Some(saved_path) = saved_path {
        let saved_path = Url::from_file_path(saved_path.as_path())
            .map(|url| url.to_string())
            .unwrap_or_else(|_| saved_path.display().to_string());
        lines.push(vec!["  └ ".dim(), "Saved to: ".dim(), saved_path.into()].into());
    }

    PlainHistoryCell { lines }
}
