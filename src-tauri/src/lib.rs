//! DNACode 后端入口：注册 Tauri 命令、管理全局状态。
//!
//! 模块划分：
//! - `fs_ops`：项目文件树与文件读写（路径校验、二进制/大文件保护、外部修改检测）
//! - `git`：Git 命令执行与状态/Diff/暂存/提交/推送/分支/历史
//! - `ai`：可选的 AI 修改总结（OpenAI 兼容 API，支持超时与取消）
//! - `config` / `credentials`：非敏感设置与系统安全凭据存储
//! - `error` / `state`：统一错误与全局状态

pub mod ai;
pub mod config;
pub mod credentials;
pub mod error;
pub mod fs_ops;
pub mod git;
pub mod state;

use error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use state::AppState;
use std::path::PathBuf;
use tauri::Manager;

fn project_root(state: &AppState) -> AppResult<PathBuf> {
    state.project_root()
}

// ---------------------------------------------------------------------------
// 项目与文件系统
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfoDto {
    pub path: String,
    pub name: String,
}

#[tauri::command]
async fn open_project(
    state: tauri::State<'_, AppState>,
    path: String,
) -> AppResult<ProjectInfoDto> {
    let raw = PathBuf::from(&path);
    if !raw.is_dir() {
        return Err(AppError::new(
            "invalid_input",
            "目录不存在或不是一个文件夹，请重新选择",
        ));
    }
    let root = dunce::canonicalize(&raw)
        .map_err(|e| AppError::from(e).with_detail(format!("无法打开目录：{path}")))?;
    let name = root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string_lossy().to_string());

    state.set_project_root(Some(root.clone()));

    let now = git::run::now_ms();
    state.update_settings(|s| s.touch_recent(&root.to_string_lossy(), &name, now));
    let snapshot = state.settings_snapshot();
    // 最近项目写入失败不阻塞打开项目
    let _ = config::save_settings(&state.config_path, &snapshot);

    Ok(ProjectInfoDto {
        path: root.to_string_lossy().to_string(),
        name,
    })
}

#[tauri::command]
fn close_project(state: tauri::State<'_, AppState>) {
    state.set_project_root(None);
}

#[tauri::command]
async fn list_directory(
    state: tauri::State<'_, AppState>,
    path: String,
) -> AppResult<Vec<fs_ops::DirEntryDto>> {
    let root = project_root(&state)?;
    tokio::task::spawn_blocking(move || fs_ops::list_directory(&path, &root))
        .await
        .map_err(|e| AppError::new("io_error", "目录读取任务失败").with_detail(e.to_string()))?
}

#[tauri::command]
async fn read_text_file(
    state: tauri::State<'_, AppState>,
    path: String,
) -> AppResult<fs_ops::FileContentDto> {
    let root = project_root(&state)?;
    tokio::task::spawn_blocking(move || fs_ops::read_text_file(&path, &root))
        .await
        .map_err(|e| AppError::new("io_error", "文件读取任务失败").with_detail(e.to_string()))?
}

#[tauri::command]
async fn write_text_file(
    state: tauri::State<'_, AppState>,
    path: String,
    content: String,
    expected_mtime_ms: Option<u64>,
    force: bool,
) -> AppResult<fs_ops::WriteResultDto> {
    let root = project_root(&state)?;
    tokio::task::spawn_blocking(move || {
        fs_ops::write_text_file(&path, &root, &content, expected_mtime_ms, force)
    })
    .await
    .map_err(|e| AppError::new("io_error", "文件写入任务失败").with_detail(e.to_string()))?
}

#[tauri::command]
async fn file_meta(
    state: tauri::State<'_, AppState>,
    path: String,
) -> AppResult<fs_ops::FileMetaDto> {
    let root = project_root(&state)?;
    tokio::task::spawn_blocking(move || fs_ops::file_meta(&path, &root))
        .await
        .map_err(|e| AppError::new("io_error", "文件状态读取任务失败").with_detail(e.to_string()))?
}

// ---------------------------------------------------------------------------
// Git
// ---------------------------------------------------------------------------

#[tauri::command]
async fn git_status(state: tauri::State<'_, AppState>) -> AppResult<git::status::GitStatus> {
    let root = project_root(&state)?;
    git::status(&root).await
}

#[tauri::command]
async fn git_file_diff(
    state: tauri::State<'_, AppState>,
    path: String,
    staged: bool,
    untracked: bool,
    full: bool,
) -> AppResult<git::diff::DiffPayload> {
    let root = project_root(&state)?;
    let mode = if staged { "staged" } else { "worktree" };
    git::diff::file_diff(&root, &path, mode, untracked, full).await
}

#[tauri::command]
async fn git_commit_diff(
    state: tauri::State<'_, AppState>,
    sha: String,
    path: Option<String>,
    full: bool,
) -> AppResult<git::diff::DiffPayload> {
    let root = project_root(&state)?;
    git::diff::commit_diff(&root, &sha, path.as_deref(), full).await
}

#[tauri::command]
async fn git_stage(state: tauri::State<'_, AppState>, paths: Vec<String>) -> AppResult<()> {
    let root = project_root(&state)?;
    git::stage::stage_paths(&root, &paths).await
}

#[tauri::command]
async fn git_unstage(state: tauri::State<'_, AppState>, paths: Vec<String>) -> AppResult<()> {
    let root = project_root(&state)?;
    git::stage::unstage_paths(&root, &paths).await
}

#[tauri::command]
async fn git_stage_all(state: tauri::State<'_, AppState>) -> AppResult<()> {
    let root = project_root(&state)?;
    git::stage::stage_all(&root).await
}

#[tauri::command]
async fn git_unstage_all(state: tauri::State<'_, AppState>) -> AppResult<()> {
    let root = project_root(&state)?;
    git::stage::unstage_all(&root).await
}

#[tauri::command]
async fn git_discard_unstaged(
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
) -> AppResult<()> {
    let root = project_root(&state)?;
    git::stage::discard_unstaged(&root, &paths).await
}

#[tauri::command]
async fn git_delete_untracked(
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
) -> AppResult<usize> {
    let root = project_root(&state)?;
    git::stage::delete_untracked(&root, &paths).await
}

#[tauri::command]
async fn git_commit(
    state: tauri::State<'_, AppState>,
    message: String,
) -> AppResult<git::commit::CommitOutcome> {
    let root = project_root(&state)?;
    git::commit::commit(&root, &message).await
}

#[tauri::command]
async fn git_push(
    state: tauri::State<'_, AppState>,
    set_upstream: bool,
) -> AppResult<git::push::PushOutcome> {
    let root = project_root(&state)?;
    git::push::push(&root, set_upstream).await
}

#[tauri::command]
async fn git_branches(state: tauri::State<'_, AppState>) -> AppResult<git::branch::BranchList> {
    let root = project_root(&state)?;
    git::branch::list(&root).await
}

#[tauri::command]
async fn git_checkout(
    state: tauri::State<'_, AppState>,
    branch: String,
    allow_dirty: bool,
) -> AppResult<String> {
    let root = project_root(&state)?;
    git::branch::checkout(&root, &branch, allow_dirty).await
}

#[tauri::command]
async fn git_log(
    state: tauri::State<'_, AppState>,
    skip: u32,
    limit: u32,
) -> AppResult<Vec<git::history::CommitInfo>> {
    let root = project_root(&state)?;
    git::history::log(&root, skip, limit).await
}

#[tauri::command]
async fn git_commit_files(
    state: tauri::State<'_, AppState>,
    sha: String,
) -> AppResult<Vec<git::history::CommitFile>> {
    let root = project_root(&state)?;
    git::history::commit_files(&root, &sha).await
}

#[tauri::command]
async fn git_init_repository(state: tauri::State<'_, AppState>) -> AppResult<()> {
    let root = project_root(&state)?;
    git::init_repository(&root).await
}

// ---------------------------------------------------------------------------
// 设置与凭据
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsDto {
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
    pub max_diff_chars: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub theme: String,
    pub ai: AiSettingsDto,
    pub recent_projects: Vec<config::RecentProject>,
    pub has_api_key: bool,
    pub keyring_error: Option<String>,
    pub config_warning: Option<String>,
}

fn settings_dto(state: &AppState) -> SettingsDto {
    let s = state.settings_snapshot();
    let (has_api_key, keyring_error) = credentials::api_key_status();
    SettingsDto {
        theme: s.theme,
        ai: AiSettingsDto {
            base_url: s.ai.base_url,
            model: s.ai.model,
            timeout_secs: s.ai.timeout_secs,
            max_diff_chars: s.ai.max_diff_chars,
        },
        recent_projects: s.recent_projects,
        has_api_key,
        keyring_error,
        config_warning: state.config_warning(),
    }
}

#[tauri::command]
fn config_get(state: tauri::State<'_, AppState>) -> SettingsDto {
    settings_dto(&state)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsInput {
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
    pub max_diff_chars: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInput {
    pub theme: String,
    pub ai: AiSettingsInput,
}

#[tauri::command]
async fn config_save(
    state: tauri::State<'_, AppState>,
    input: SettingsInput,
) -> AppResult<SettingsDto> {
    let mut next = state.settings_snapshot();
    next.theme = input.theme;
    next.ai = config::AiSettings {
        base_url: input.ai.base_url,
        model: input.ai.model,
        timeout_secs: input.ai.timeout_secs,
        max_diff_chars: input.ai.max_diff_chars,
    };
    next.validate()?;
    config::save_settings(&state.config_path, &next)?;
    state.update_settings(|s| *s = next);
    Ok(settings_dto(&state))
}

/// 保存 API Key 到系统安全凭据存储。失败时直接报错，绝不写入配置文件。
#[tauri::command]
fn api_key_set(state: tauri::State<'_, AppState>, key: String) -> AppResult<SettingsDto> {
    let k = key.trim();
    if k.is_empty() {
        return Err(AppError::new("invalid_input", "API Key 不能为空"));
    }
    credentials::save_api_key(k)?;
    Ok(settings_dto(&state))
}

#[tauri::command]
fn api_key_delete(state: tauri::State<'_, AppState>) -> AppResult<SettingsDto> {
    credentials::delete_api_key()?;
    Ok(settings_dto(&state))
}

#[tauri::command]
async fn recent_project_remove(
    state: tauri::State<'_, AppState>,
    path: String,
) -> AppResult<SettingsDto> {
    state.update_settings(|s| s.recent_projects.retain(|p| p.path != path));
    let snapshot = state.settings_snapshot();
    config::save_settings(&state.config_path, &snapshot)?;
    Ok(settings_dto(&state))
}

#[tauri::command]
async fn recent_project_clear(state: tauri::State<'_, AppState>) -> AppResult<SettingsDto> {
    state.update_settings(|s| s.recent_projects.clear());
    let snapshot = state.settings_snapshot();
    config::save_settings(&state.config_path, &snapshot)?;
    Ok(settings_dto(&state))
}

// ---------------------------------------------------------------------------
// 应用入口
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let (config_dir, warning) = match app.path().app_config_dir() {
                Ok(dir) => (dir, None),
                Err(e) => (
                    std::env::temp_dir().join("dnacode-config"),
                    Some(format!("无法获取系统配置目录，设置将保存到临时目录：{e}")),
                ),
            };
            let config_path = config_dir.join("settings.json");
            let (settings, load_warning) = config::load_settings(&config_path);
            let warning = load_warning.or(warning);
            app.manage(AppState::new(config_path, settings, warning));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_project,
            close_project,
            list_directory,
            read_text_file,
            write_text_file,
            file_meta,
            git_status,
            git_file_diff,
            git_commit_diff,
            git_stage,
            git_unstage,
            git_stage_all,
            git_unstage_all,
            git_discard_unstaged,
            git_delete_untracked,
            git_commit,
            git_push,
            git_branches,
            git_checkout,
            git_log,
            git_commit_files,
            git_init_repository,
            config_get,
            config_save,
            api_key_set,
            api_key_delete,
            recent_project_remove,
            recent_project_clear,
            ai::ai_preview,
            ai::ai_summarize,
            ai::ai_cancel,
            ai::ai_test_connection
        ])
        .run(tauri::generate_context!())
        .expect("error while running DNACode");
}
