use crate::config::Settings;
use crate::error::{AppError, AppResult};
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::sync::oneshot;

/// 应用全局状态：
/// - 当前打开的项目根目录（所有文件操作都会被限制在该目录内）
/// - 已加载的设置（内存缓存 + 落盘路径）
/// - AI 请求取消通道（同一时间只允许一个进行中的请求）
pub struct AppState {
    pub project_root: Mutex<Option<PathBuf>>,
    pub settings: Mutex<Settings>,
    pub config_path: PathBuf,
    pub config_warning: Mutex<Option<String>>,
    pub ai_cancel: Mutex<Option<oneshot::Sender<()>>>,
}

impl AppState {
    pub fn new(config_path: PathBuf, settings: Settings, warning: Option<String>) -> Self {
        Self {
            project_root: Mutex::new(None),
            settings: Mutex::new(settings),
            config_path,
            config_warning: Mutex::new(warning),
            ai_cancel: Mutex::new(None),
        }
    }

    pub fn project_root(&self) -> AppResult<PathBuf> {
        self.project_root
            .lock()
            .expect("project_root lock poisoned")
            .clone()
            .ok_or_else(|| AppError::new("no_project", "尚未打开项目"))
    }

    pub fn set_project_root(&self, root: Option<PathBuf>) {
        *self
            .project_root
            .lock()
            .expect("project_root lock poisoned") = root;
    }

    pub fn settings_snapshot(&self) -> Settings {
        self.settings
            .lock()
            .expect("settings lock poisoned")
            .clone()
    }

    pub fn update_settings<T>(&self, f: impl FnOnce(&mut Settings) -> T) -> T {
        let mut guard = self.settings.lock().expect("settings lock poisoned");
        f(&mut guard)
    }

    pub fn config_warning(&self) -> Option<String> {
        self.config_warning
            .lock()
            .expect("config_warning lock poisoned")
            .clone()
    }

    pub fn take_config_warning(&self) -> Option<String> {
        self.config_warning
            .lock()
            .expect("config_warning lock poisoned")
            .take()
    }

    pub fn set_ai_cancel(&self, sender: Option<oneshot::Sender<()>>) {
        *self.ai_cancel.lock().expect("ai_cancel lock poisoned") = sender;
    }

    pub fn fire_ai_cancel(&self) -> bool {
        let mut guard = self.ai_cancel.lock().expect("ai_cancel lock poisoned");
        match guard.take() {
            Some(tx) => tx.send(()).is_ok(),
            None => false,
        }
    }
}
