//! 暂存 / 取消暂存 / 丢弃修改 / 删除未跟踪文件。
//!
//! 所有操作都真实执行 Git 命令；危险操作（丢弃、删除）仅接受当前确实处于
//! 对应状态的文件，防止误伤。

use crate::error::{AppError, AppResult};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use super::run::{has_head, run, run_ok};
use super::{validate_repo_path, T_NORMAL};

fn validate_paths(paths: &[String]) -> AppResult<Vec<String>> {
    if paths.is_empty() {
        return Err(AppError::new("invalid_input", "未选择任何文件"));
    }
    paths.iter().map(|p| validate_repo_path(p)).collect()
}

/// 暂存指定文件（单个或批量）。使用 `-A` 以便同时记录删除。
pub async fn stage_paths(project: &Path, paths: &[String]) -> AppResult<()> {
    let rels = validate_paths(paths)?;
    let mut args: Vec<String> = vec!["add".into(), "-A".into(), "--".into()];
    args.extend(rels);
    run_ok(project, &args, T_NORMAL, "暂存文件失败").await?;
    Ok(())
}

/// 全部暂存（包含未跟踪文件与删除，自动遵守 .gitignore）。
pub async fn stage_all(project: &Path) -> AppResult<()> {
    run_ok(
        project,
        &["add", "-A", "--", "."],
        T_NORMAL,
        "暂存全部修改失败",
    )
    .await?;
    Ok(())
}

/// 取消暂存指定文件。
pub async fn unstage_paths(project: &Path, paths: &[String]) -> AppResult<()> {
    let rels = validate_paths(paths)?;
    unstage(project, &rels).await
}

/// 取消全部暂存。
pub async fn unstage_all(project: &Path) -> AppResult<()> {
    unstage(project, &[".".to_string()]).await
}

async fn unstage(project: &Path, rels: &[String]) -> AppResult<()> {
    if has_head(project).await {
        let mut args: Vec<String> = vec!["restore".into(), "--staged".into(), "--".into()];
        args.extend(rels.iter().cloned());
        if let Ok(out) = run(project, &args, T_NORMAL).await {
            if out.ok() {
                return Ok(());
            }
        }
        // 兼容不支持 restore 的旧版本 Git
        let mut args2: Vec<String> = vec!["reset".into(), "-q".into(), "HEAD".into(), "--".into()];
        args2.extend(rels.iter().cloned());
        run_ok(project, &args2, T_NORMAL, "取消暂存失败").await?;
        Ok(())
    } else {
        // 尚无提交（unborn HEAD）：直接从索引移除，保留工作区文件
        let mut args: Vec<String> = vec![
            "rm".into(),
            "--cached".into(),
            "-r".into(),
            "-q".into(),
            "--".into(),
        ];
        args.extend(rels.iter().cloned());
        run_ok(project, &args, T_NORMAL, "取消暂存失败").await?;
        Ok(())
    }
}

/// 丢弃指定文件的未暂存修改（危险操作，前端必须先确认影响范围）。
pub async fn discard_unstaged(project: &Path, paths: &[String]) -> AppResult<()> {
    let rels = validate_paths(paths)?;
    let mut args: Vec<String> = vec!["restore".into(), "--worktree".into(), "--".into()];
    args.extend(rels.iter().cloned());
    if let Ok(out) = run(project, &args, T_NORMAL).await {
        if out.ok() {
            return Ok(());
        }
    }
    // 兼容旧版本 Git
    let mut args2: Vec<String> = vec!["checkout".into(), "--".into()];
    args2.extend(rels);
    run_ok(project, &args2, T_NORMAL, "丢弃修改失败").await?;
    Ok(())
}

/// 删除未跟踪文件（危险操作，前端必须先确认影响范围）。
///
/// 只接受当前真实处于“未跟踪”状态的文件，其余一律拒绝，防止误删。
pub async fn delete_untracked(project: &Path, paths: &[String]) -> AppResult<usize> {
    let rels = validate_paths(paths)?;

    let st = super::status(project).await?;
    if !st.is_repo {
        return Err(AppError::new("not_a_repository", "当前项目不是 Git 仓库"));
    }
    let untracked: HashSet<&str> = st
        .entries
        .iter()
        .filter(|e| e.untracked)
        .map(|e| e.path.as_str())
        .collect();

    for rel in &rels {
        if !untracked.contains(rel.as_str()) {
            return Err(AppError::new(
                "invalid_input",
                format!("文件不在未跟踪列表中，已取消删除：{rel}"),
            ));
        }
    }

    let mut deleted = 0usize;
    for rel in &rels {
        let abs = project.join(rel);
        match fs::remove_file(&abs) {
            Ok(()) => deleted += 1,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(AppError::from(e).with_detail(format!("删除失败：{rel}")));
            }
        }
    }
    Ok(deleted)
}
