//! Git 命令执行封装。
//!
//! 所有 Git 调用都通过 `std::process::Command`（tokio 封装）直接以参数数组方式执行，
//! 不经过 shell，前端无法传入任意命令字符串。

use crate::error::{map_git_failure, AppError, AppResult};
use std::ffi::OsStr;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

#[derive(Debug)]
pub struct GitOutput {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl GitOutput {
    pub fn ok(&self) -> bool {
        self.code == Some(0)
    }

    pub fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    pub fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

/// 在指定目录执行 Git 命令。
pub async fn run<S: AsRef<OsStr>>(
    dir: &Path,
    args: &[S],
    timeout: Duration,
) -> AppResult<GitOutput> {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir)
        // 关闭路径引号转义，保证非 ASCII 路径输出可解析
        .arg("-c")
        .arg("core.quotepath=false")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // 超时/取消时杀掉子进程，避免留下孤儿进程
        .kill_on_drop(true)
        // 图形应用中不允许 Git 等待终端输入，避免卡死
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");

    let child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::new(
                "git_not_found",
                "未检测到 Git，请先安装 Git 并确保 git 命令在系统 PATH 中可用",
            )
        } else {
            AppError::new("git_failed", "无法启动 Git 进程").with_detail(e.to_string())
        }
    })?;

    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(out)) => Ok(GitOutput {
            code: out.status.code(),
            stdout: out.stdout,
            stderr: out.stderr,
        }),
        Ok(Err(e)) => {
            Err(AppError::new("git_failed", "Git 命令执行失败").with_detail(e.to_string()))
        }
        Err(_) => Err(AppError::new(
            "git_timeout",
            format!("Git 命令超时（超过 {} 秒）", timeout.as_secs()),
        )),
    }
}

/// 执行 Git 命令，非零退出时返回映射后的用户可读错误（保留原始输出）。
pub async fn run_ok<S: AsRef<OsStr>>(
    dir: &Path,
    args: &[S],
    timeout: Duration,
    context: &str,
) -> AppResult<GitOutput> {
    let out = run(dir, args, timeout).await?;
    if out.ok() {
        Ok(out)
    } else {
        Err(map_git_failure(
            context,
            &out.stderr_text(),
            &out.stdout_text(),
        ))
    }
}

/// 当前目录是否位于 Git 工作树内。
pub async fn is_repo(dir: &Path) -> bool {
    matches!(
        run(dir, &["rev-parse", "--is-inside-work-tree"], Duration::from_secs(10)).await,
        Ok(out) if out.ok() && out.stdout_text().trim() == "true"
    )
}

/// 仓库是否已有提交（HEAD 是否存在）。
pub async fn has_head(dir: &Path) -> bool {
    matches!(
        run(
            dir,
            &["rev-parse", "--verify", "--quiet", "HEAD"],
            Duration::from_secs(10)
        )
        .await,
        Ok(out) if out.ok()
    )
}

/// 当前分支名；detached HEAD 或非仓库时返回 None。
pub async fn current_branch(dir: &Path) -> AppResult<Option<String>> {
    let out = run(dir, &["branch", "--show-current"], Duration::from_secs(10)).await?;
    if !out.ok() {
        return Ok(None);
    }
    let name = out.stdout_text().trim().to_string();
    Ok(if name.is_empty() { None } else { Some(name) })
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
