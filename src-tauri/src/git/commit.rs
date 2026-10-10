//! 提交：严格遵守 Git 暂存区语义，只提交已暂存内容。

use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::path::Path;

use super::run::{run, run_ok};
use super::{T_NORMAL, T_SLOW};

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommitOutcome {
    pub sha: String,
    pub summary: String,
}

/// 执行提交。
///
/// 安全约束：
/// - 提交信息不能为空白；
/// - 没有已暂存内容时拒绝提交（绝不会自动暂存）；
/// - 未配置提交身份时给出明确引导；
/// - 失败时保留原始错误输出供诊断。
pub async fn commit(project: &Path, message: &str) -> AppResult<CommitOutcome> {
    let msg = message.trim().to_string();
    if msg.is_empty() {
        return Err(AppError::new("invalid_input", "提交信息不能为空"));
    }

    // 1) 必须存在已暂存内容
    let staged = run(project, &["diff", "--cached", "--quiet"], T_NORMAL).await?;
    if staged.ok() {
        return Err(AppError::new(
            "no_staged_changes",
            "没有已暂存的修改，提交已取消。请先暂存需要提交的文件（不会自动暂存未暂存内容）",
        ));
    }

    // 2) 提交身份检查（预先给出友好提示）
    let name = run(project, &["config", "--get", "user.name"], T_NORMAL).await;
    let email = run(project, &["config", "--get", "user.email"], T_NORMAL).await;
    let name_ok = matches!(&name, Ok(o) if o.ok() && !o.stdout_text().trim().is_empty());
    let email_ok = matches!(&email, Ok(o) if o.ok() && !o.stdout_text().trim().is_empty());
    if !name_ok || !email_ok {
        return Err(AppError::new(
            "no_identity",
            "未配置 Git 提交身份（user.name / user.email），无法提交。可在系统终端执行：\n  git config --global user.name \"你的名字\"\n  git config --global user.email \"你的邮箱\"",
        ));
    }

    // 3) 提交（只提交已暂存内容）
    run_ok(project, &["commit", "-m", &msg], T_SLOW, "提交失败").await?;

    let sha = run_ok(
        project,
        &["rev-parse", "--short", "HEAD"],
        T_NORMAL,
        "读取提交结果失败",
    )
    .await?
    .stdout_text()
    .trim()
    .to_string();
    let summary = run_ok(
        project,
        &["log", "-1", "--format=%s"],
        T_NORMAL,
        "读取提交结果失败",
    )
    .await?
    .stdout_text()
    .trim()
    .to_string();

    Ok(CommitOutcome { sha, summary })
}
