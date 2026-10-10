//! 推送：独立操作，仅在用户明确点击时执行；沿用系统 Git 配置与凭据。

use crate::error::{map_git_failure, AppError, AppResult};
use serde::Serialize;
use std::path::Path;

use super::run::{current_branch, run};
use super::{T_FAST, T_SLOW};

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PushOutcome {
    pub branch: String,
    pub summary: String,
}

/// 推送到远程。
///
/// `set_upstream == true` 时执行 `git push -u <remote> <branch>` 建立上游分支，
/// 否则执行普通的 `git push`（要求已配置上游）。
pub async fn push(project: &Path, set_upstream: bool) -> AppResult<PushOutcome> {
    let branch = current_branch(project).await?.ok_or_else(|| {
        AppError::new(
            "detached_head",
            "当前处于 detached HEAD 状态，无法推送。请先切换到具体分支",
        )
    })?;

    let remotes_out = run(project, &["remote"], T_FAST).await?;
    let remotes: Vec<String> = remotes_out
        .stdout_text()
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if remotes.is_empty() {
        return Err(AppError::new(
            "no_remote",
            "当前仓库没有配置远程仓库，无法推送。可先添加远程仓库，例如：git remote add origin <仓库地址>",
        ));
    }

    let upstream = run(
        project,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
        T_FAST,
    )
    .await?;
    let has_upstream = upstream.ok() && !upstream.stdout_text().trim().is_empty();

    let out = if !has_upstream {
        if !set_upstream {
            return Err(AppError::new(
                "no_upstream",
                format!(
                    "本地分支 {branch} 还没有上游分支。可点击「发布分支并推送」把该分支推送到远程并建立跟踪关系"
                ),
            ));
        }
        let remote = if remotes.iter().any(|r| r == "origin") {
            "origin".to_string()
        } else {
            remotes[0].clone()
        };
        let args: Vec<String> = vec!["push".into(), "-u".into(), remote, branch.clone()];
        run(project, &args, T_SLOW).await?
    } else {
        run(project, &["push"], T_SLOW).await?
    };

    if !out.ok() {
        return Err(map_git_failure(
            "推送",
            &out.stderr_text(),
            &out.stdout_text(),
        ));
    }

    // git push 的进度信息写在 stderr
    let summary = {
        let s = out.stderr_text().trim().to_string();
        if s.is_empty() {
            out.stdout_text().trim().to_string()
        } else {
            s
        }
    };

    Ok(PushOutcome { branch, summary })
}
