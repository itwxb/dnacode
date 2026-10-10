//! Git 功能模块：命令执行、状态解析、Diff、暂存、提交、推送、分支与历史。

pub mod branch;
pub mod commit;
pub mod diff;
pub mod history;
pub mod push;
pub mod run;
pub mod stage;
pub mod status;

use crate::error::{map_git_failure, AppError, AppResult};
use status::GitStatus;
use std::path::Path;
use std::time::Duration;

pub const T_FAST: Duration = Duration::from_secs(10);
pub const T_NORMAL: Duration = Duration::from_secs(30);
pub const T_SLOW: Duration = Duration::from_secs(300);

/// 校验来自前端的仓库相对路径：拒绝空值、绝对路径与 `..` 片段。
pub fn validate_repo_path(path: &str) -> AppResult<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(AppError::new("invalid_input", "路径不能为空"));
    }
    let p = std::path::Path::new(trimmed);
    if p.is_absolute() {
        return Err(AppError::new("invalid_input", "必须使用仓库相对路径"));
    }
    if p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(AppError::new(
            "invalid_input",
            "路径不合法：包含上级目录引用",
        ));
    }
    Ok(trimmed.replace('\\', "/"))
}

/// 校验提交 ID：只允许 4-40 位十六进制字符。
pub fn validate_sha(sha: &str) -> AppResult<String> {
    let s = sha.trim();
    if s.len() < 4 || s.len() > 40 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::new("invalid_input", "提交 ID 不合法"));
    }
    Ok(s.to_lowercase())
}

/// 获取仓库状态（始终以真实仓库数据为准）。
///
/// 非 Git 仓库不会报错，而是返回 `is_repo == false`，由界面展示对应状态。
pub async fn status(project: &Path) -> AppResult<GitStatus> {
    let out = run::run(
        project,
        &[
            "status",
            "--branch",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ],
        T_NORMAL,
    )
    .await?;

    if !out.ok() {
        let stderr = out.stderr_text();
        if stderr.to_lowercase().contains("not a git repository") {
            return Ok(GitStatus::not_a_repo());
        }
        return Err(map_git_failure(
            "获取 Git 状态失败",
            &stderr,
            &out.stdout_text(),
        ));
    }

    let mut st = status::parse_porcelain_v1_z(&out.stdout);

    // 提交身份（局部或全局配置）
    if let Ok(o) = run::run(project, &["config", "--get", "user.name"], T_FAST).await {
        if o.ok() {
            let v = o.stdout_text().trim().to_string();
            if !v.is_empty() {
                st.identity_name = Some(v);
            }
        }
    }
    if let Ok(o) = run::run(project, &["config", "--get", "user.email"], T_FAST).await {
        if o.ok() {
            let v = o.stdout_text().trim().to_string();
            if !v.is_empty() {
                st.identity_email = Some(v);
            }
        }
    }
    // 远程仓库列表
    if let Ok(o) = run::run(project, &["remote"], T_FAST).await {
        if o.ok() {
            st.remotes = o
                .stdout_text()
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
        }
    }

    Ok(st)
}

/// 在项目目录初始化一个新的 Git 仓库（仅用户显式触发）。
pub async fn init_repository(project: &Path) -> AppResult<()> {
    run::run_ok(project, &["init"], T_NORMAL, "初始化 Git 仓库失败").await?;
    Ok(())
}
