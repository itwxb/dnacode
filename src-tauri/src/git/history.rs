//! 提交历史与提交内容。

use crate::error::AppResult;
use serde::Serialize;
use std::path::Path;

use super::run::{has_head, run_ok};
use super::{validate_sha, T_NORMAL};

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    pub sha: String,
    pub short_sha: String,
    pub summary: String,
    pub author: String,
    pub email: String,
    pub time_ms: u64,
}

/// 读取最近提交历史（分页，避免一次性加载庞大仓库的全部历史）。
pub async fn log(project: &Path, skip: u32, limit: u32) -> AppResult<Vec<CommitInfo>> {
    if !has_head(project).await {
        return Ok(Vec::new());
    }
    let limit = limit.clamp(1, 200);
    let args: Vec<String> = vec![
        "log".into(),
        format!("--max-count={limit}"),
        format!("--skip={skip}"),
        "--format=%H%x1f%an%x1f%ae%x1f%ct%x1f%s".into(),
        "--no-color".into(),
    ];
    let out = run_ok(project, &args, T_NORMAL, "读取提交历史失败").await?;

    let mut commits = Vec::new();
    for line in out.stdout_text().lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(5, '\u{1f}').collect();
        if parts.len() < 5 {
            continue;
        }
        let sha = parts[0].to_string();
        commits.push(CommitInfo {
            short_sha: sha.chars().take(8).collect(),
            sha,
            author: parts[1].to_string(),
            email: parts[2].to_string(),
            time_ms: parts[3].parse::<u64>().unwrap_or(0).saturating_mul(1000),
            summary: parts[4].to_string(),
        });
    }
    Ok(commits)
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommitFile {
    pub status: String,
    pub path: String,
    pub orig_path: Option<String>,
}

/// 读取某次提交涉及的文件列表（真实历史数据）。
pub async fn commit_files(project: &Path, sha: &str) -> AppResult<Vec<CommitFile>> {
    let sha = validate_sha(sha)?;
    let args: Vec<String> = vec![
        "diff-tree".into(),
        "-r".into(),
        "--root".into(),
        "--no-commit-id".into(),
        "--name-status".into(),
        "-z".into(),
        "-M".into(),
        sha,
    ];
    let out = run_ok(project, &args, T_NORMAL, "读取提交文件列表失败").await?;

    let fields: Vec<&[u8]> = out.stdout.split(|b| *b == 0).collect();
    let mut files = Vec::new();
    let mut i = 0;
    while i < fields.len() {
        let field = fields[i];
        if field.is_empty() {
            i += 1;
            continue;
        }
        let status = String::from_utf8_lossy(field).into_owned();
        let letter = status.chars().next().unwrap_or('?');

        if matches!(letter, 'R' | 'C') {
            // 重命名/复制：旧路径在前，新路径在后
            if i + 2 < fields.len() {
                let old = String::from_utf8_lossy(fields[i + 1]).into_owned();
                let new = String::from_utf8_lossy(fields[i + 2]).into_owned();
                files.push(CommitFile {
                    status: letter.to_string(),
                    path: new,
                    orig_path: Some(old),
                });
                i += 3;
                continue;
            }
            i += 1;
        } else if i + 1 < fields.len() {
            let path = String::from_utf8_lossy(fields[i + 1]).into_owned();
            files.push(CommitFile {
                status: letter.to_string(),
                path,
                orig_path: None,
            });
            i += 2;
        } else {
            i += 1;
        }
    }
    Ok(files)
}
