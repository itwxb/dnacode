//! 文件差异 / 提交差异生成。

use crate::error::{AppError, AppResult};
use crate::fs_ops::looks_binary;
use serde::Serialize;
use std::fs;
use std::path::Path;

use super::run::{has_head, run_ok};
use super::{validate_repo_path, validate_sha, T_NORMAL};

/// 单次返回给前端的 diff 文本上限（字节数，约 400 KB）。
pub const MAX_DIFF_CHARS: usize = 400_000;
/// 为未跟踪文件生成“相对空文件”差异时的体积上限。
pub const UNTRACKED_MAX_BYTES: u64 = 1_500_000;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiffPayload {
    /// text | binary | too_large | empty
    pub kind: String,
    pub text: Option<String>,
    pub truncated: bool,
    pub added: u32,
    pub removed: u32,
    pub message: Option<String>,
    pub path: String,
}

impl DiffPayload {
    fn status(path: &str, kind: &str, message: &str) -> Self {
        Self {
            kind: kind.to_string(),
            text: None,
            truncated: false,
            added: 0,
            removed: 0,
            message: Some(message.to_string()),
            path: path.to_string(),
        }
    }
}

fn count_changed_lines(text: &str) -> (u32, u32) {
    let mut added = 0u32;
    let mut removed = 0u32;
    for line in text.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if line.starts_with('+') {
            added += 1;
        } else if line.starts_with('-') {
            removed += 1;
        }
    }
    (added, removed)
}

fn truncate_text(text: &str, full: bool) -> (String, bool) {
    if full || text.len() <= MAX_DIFF_CHARS {
        return (text.to_string(), false);
    }
    let mut limit = MAX_DIFF_CHARS;
    while limit > 0 && !text.is_char_boundary(limit) {
        limit -= 1;
    }
    let slice = &text[..limit];
    let cut_at = slice.rfind('\n').map(|i| i + 1).unwrap_or(limit);
    let mut out = text[..cut_at].to_string();
    out.push_str("\n…（差异过大，已截断。可点击“加载完整差异”查看全部内容）\n");
    (out, true)
}

fn build_text_payload(path: &str, text: &str, full: bool) -> DiffPayload {
    if text.trim().is_empty() {
        return DiffPayload::status(path, "empty", "没有差异");
    }
    let is_binary = text.lines().any(|l| {
        l.starts_with("Binary files ")
            || l.trim() == "GIT binary patch"
            || l.starts_with("Cannot display:")
    });
    if is_binary {
        let (t, _) = truncate_text(text, full);
        return DiffPayload {
            kind: "binary".to_string(),
            text: Some(t),
            truncated: false,
            added: 0,
            removed: 0,
            message: Some("二进制文件，无法显示文本差异".to_string()),
            path: path.to_string(),
        };
    }
    let (added, removed) = count_changed_lines(text);
    let (t, truncated) = truncate_text(text, full);
    DiffPayload {
        kind: "text".to_string(),
        text: Some(t),
        truncated,
        added,
        removed,
        message: None,
        path: path.to_string(),
    }
}

/// 生成单个文件的差异。
///
/// `mode`：
/// - `staged`：暂存区 vs HEAD（`git diff --cached`）
/// - `worktree`：工作区 vs 暂存区（`git diff`）
/// - `head`：工作区+暂存区 vs HEAD（`git diff HEAD`，仓库无提交时退化为 --cached）
pub async fn file_diff(
    project: &Path,
    path: &str,
    mode: &str,
    untracked: bool,
    full: bool,
) -> AppResult<DiffPayload> {
    let rel = validate_repo_path(path)?;

    if untracked && mode == "worktree" {
        return untracked_diff(project, &rel, full).await;
    }

    let mut args: Vec<String> = vec![
        "diff".into(),
        "--no-color".into(),
        "--no-ext-diff".into(),
        "--unified=3".into(),
        "-M".into(),
    ];
    match mode {
        "staged" => args.push("--cached".into()),
        "worktree" => {}
        "head" => {
            if has_head(project).await {
                args.push("HEAD".into());
            } else {
                args.push("--cached".into());
            }
        }
        other => {
            return Err(AppError::new(
                "invalid_input",
                format!("不支持的差异模式：{other}"),
            ))
        }
    }
    args.push("--".into());
    args.push(rel.clone());

    let out = run_ok(project, &args, T_NORMAL, "生成文件差异失败").await?;
    Ok(build_text_payload(&rel, &out.stdout_text(), full))
}

/// 为未跟踪文件生成“相对于空文件”的新增差异（基于真实文件内容）。
async fn untracked_diff(project: &Path, rel: &str, full: bool) -> AppResult<DiffPayload> {
    let abs = project.join(rel);
    let meta = fs::metadata(&abs).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::new("not_found", "文件不存在（可能已被删除）")
        } else {
            AppError::from(e)
        }
    })?;

    if meta.len() > UNTRACKED_MAX_BYTES {
        return Ok(DiffPayload::status(
            rel,
            "too_large",
            &format!(
                "文件较大（{:.1} MB），未生成完整内容差异",
                meta.len() as f64 / 1_048_576.0
            ),
        ));
    }

    let bytes = fs::read(&abs)?;
    if looks_binary(&bytes) {
        return Ok(DiffPayload::status(
            rel,
            "binary",
            "二进制文件，无法显示文本差异",
        ));
    }
    let content = match String::from_utf8(bytes) {
        Ok(c) => c,
        Err(_) => {
            return Ok(DiffPayload::status(
                rel,
                "binary",
                "文件不是 UTF-8 文本，无法显示差异",
            ))
        }
    };
    if content.is_empty() {
        return Ok(DiffPayload::status(
            rel,
            "empty",
            "空文件（没有可显示的内容）",
        ));
    }

    let ends_with_newline = content.ends_with('\n');
    let line_count = content.matches('\n').count() + usize::from(!ends_with_newline);

    let mut text = String::new();
    text.push_str(&format!("diff --git a/{rel} b/{rel}\n"));
    text.push_str("new file mode 100644\n");
    text.push_str("--- /dev/null\n");
    text.push_str(&format!("+++ b/{rel}\n"));
    text.push_str(&format!("@@ -0,0 +1,{line_count} @@\n"));
    let mut lines: Vec<&str> = content.split('\n').collect();
    if ends_with_newline {
        lines.pop();
    }
    for line in &lines {
        text.push('+');
        text.push_str(line);
        text.push('\n');
    }
    if !ends_with_newline {
        text.push_str("\\ No newline at end of file\n");
    }

    Ok(build_text_payload(rel, &text, full))
}

/// 生成某次提交（或其中某个文件）的差异，来自真实的仓库历史。
pub async fn commit_diff(
    project: &Path,
    sha: &str,
    path: Option<&str>,
    full: bool,
) -> AppResult<DiffPayload> {
    let sha = validate_sha(sha)?;
    let mut args: Vec<String> = vec![
        "diff-tree".into(),
        "-r".into(),
        "--root".into(),
        "-p".into(),
        "--no-commit-id".into(),
        "-M".into(),
        "--no-color".into(),
        sha.clone(),
    ];
    let label = match path {
        Some(p) => {
            let rel = validate_repo_path(p)?;
            args.push("--".into());
            args.push(rel.clone());
            rel
        }
        None => sha.clone(),
    };
    let out = run_ok(project, &args, T_NORMAL, "读取提交差异失败").await?;
    Ok(build_text_payload(&label, &out.stdout_text(), full))
}
