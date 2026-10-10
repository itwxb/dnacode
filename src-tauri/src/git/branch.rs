//! 分支列表与切换。

use crate::error::{map_git_failure, AppError, AppResult};
use serde::Serialize;
use std::path::Path;

use super::run::{run, run_ok};
use super::{T_FAST, T_NORMAL};

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BranchList {
    pub current: Option<String>,
    pub detached: bool,
    pub branches: Vec<String>,
}

pub async fn list(project: &Path) -> AppResult<BranchList> {
    let current_out = run(project, &["branch", "--show-current"], T_FAST).await?;
    let current = if current_out.ok() {
        let name = current_out.stdout_text().trim().to_string();
        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    } else {
        None
    };

    let branches_out = run_ok(
        project,
        &["branch", "--list", "--format=%(refname:short)"],
        T_NORMAL,
        "读取分支列表失败",
    )
    .await?;
    let branches: Vec<String> = branches_out
        .stdout_text()
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    Ok(BranchList {
        detached: current.is_none(),
        current,
        branches,
    })
}

/// 校验分支名：拒绝空值、以 `-` 开头（防止被当作命令行选项）及 Git 禁止的字符。
pub fn validate_branch_name(name: &str) -> AppResult<String> {
    let n = name.trim();
    if n.is_empty() {
        return Err(AppError::new("invalid_input", "分支名不能为空"));
    }
    if n.starts_with('-') {
        return Err(AppError::new("invalid_input", "分支名不合法"));
    }
    if n.chars().any(|c| {
        c.is_control() || matches!(c, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\' | '"')
    }) || n.contains("..")
        || n.starts_with('/')
        || n.ends_with('/')
    {
        return Err(AppError::new("invalid_input", "分支名包含非法字符"));
    }
    Ok(n.to_string())
}

/// 切换分支。
///
/// `allow_dirty == false` 时，如果工作区存在未提交修改（暂存、未暂存或冲突），
/// 返回 `dirty_worktree` 错误，由前端向用户确认后再以 `allow_dirty = true` 重试。
pub async fn checkout(project: &Path, branch: &str, allow_dirty: bool) -> AppResult<String> {
    let name = validate_branch_name(branch)?;

    let verify = run(
        project,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{name}"),
        ],
        T_FAST,
    )
    .await?;
    if !verify.ok() {
        return Err(AppError::new(
            "invalid_input",
            format!("分支不存在：{name}"),
        ));
    }

    if !allow_dirty {
        let st = super::status(project).await?;
        if st.is_repo && (st.staged_count > 0 || st.unstaged_count > 0 || st.conflicted_count > 0) {
            return Err(AppError::new(
                "dirty_worktree",
                format!(
                    "工作区还有未提交的修改（已暂存 {} 个，未暂存 {} 个，冲突 {} 个）。切换分支可能失败或影响这些修改，是否继续？",
                    st.staged_count, st.unstaged_count, st.conflicted_count
                ),
            ));
        }
    }

    let out = run(project, &["checkout", name.as_str()], T_NORMAL).await?;
    if !out.ok() {
        return Err(map_git_failure(
            "切换分支",
            &out.stderr_text(),
            &out.stdout_text(),
        ));
    }
    Ok(name)
}
