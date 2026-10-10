use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 系统安全凭据存储中的服务名（与 tauri identifier 保持一致）。
pub const KEYRING_SERVICE: &str = "com.dnacode.app";
/// 系统安全凭据存储中的条目名。
pub const KEYRING_ACCOUNT: &str = "ai_api_key";

pub const MAX_RECENT_PROJECTS: usize = 10;
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;
pub const DEFAULT_MAX_DIFF_CHARS: usize = 60_000;
pub const MIN_TIMEOUT_SECS: u64 = 5;
pub const MAX_TIMEOUT_SECS: u64 = 600;
pub const MIN_MAX_DIFF_CHARS: usize = 2_000;
pub const MAX_MAX_DIFF_CHARS: usize = 400_000;

/// AI 相关设置（不含 API Key，Key 单独存储在系统安全凭据中）。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct AiSettings {
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
    pub max_diff_chars: usize,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            model: String::new(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            max_diff_chars: DEFAULT_MAX_DIFF_CHARS,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub path: String,
    pub name: String,
    pub last_opened_ms: u64,
}

/// 普通设置（仅存储非敏感信息，禁止出现 API Key / Token / 密码）。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: String,
    pub ai: AiSettings,
    pub recent_projects: Vec<RecentProject>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            ai: AiSettings::default(),
            recent_projects: Vec::new(),
        }
    }
}

impl Settings {
    /// 打开项目后更新最近项目列表：去重、置顶、截断。
    pub fn touch_recent(&mut self, path: &str, name: &str, now_ms: u64) {
        self.recent_projects.retain(|p| p.path != path);
        self.recent_projects.insert(
            0,
            RecentProject {
                path: path.to_string(),
                name: name.to_string(),
                last_opened_ms: now_ms,
            },
        );
        self.recent_projects.truncate(MAX_RECENT_PROJECTS);
    }

    /// 规整配置值并校验，非法值直接返回错误（由调用方决定是否提示用户）。
    pub fn validate(&mut self) -> AppResult<()> {
        if self.theme != "dark" && self.theme != "light" {
            self.theme = "dark".to_string();
        }

        let ai = &mut self.ai;
        ai.base_url = ai.base_url.trim().trim_end_matches('/').to_string();
        ai.model = ai.model.trim().to_string();
        if !ai.base_url.is_empty()
            && !(ai.base_url.starts_with("http://") || ai.base_url.starts_with("https://"))
        {
            return Err(AppError::new(
                "invalid_input",
                "AI Base URL 必须以 http:// 或 https:// 开头",
            ));
        }
        ai.timeout_secs = ai.timeout_secs.clamp(MIN_TIMEOUT_SECS, MAX_TIMEOUT_SECS);
        ai.max_diff_chars = ai
            .max_diff_chars
            .clamp(MIN_MAX_DIFF_CHARS, MAX_MAX_DIFF_CHARS);
        self.recent_projects.retain(|p| !p.path.trim().is_empty());
        self.recent_projects.truncate(MAX_RECENT_PROJECTS);
        Ok(())
    }
}

/// 读取设置文件。文件不存在时返回默认配置；解析失败时返回默认配置并附带警告，
/// 不覆盖用户原文件（写回前会先备份）。
pub fn load_settings(path: &Path) -> (Settings, Option<String>) {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<Settings>(&text) {
            Ok(mut s) => {
                let _ = s.validate();
                (s, None)
            }
            Err(e) => (
                Settings::default(),
                Some(format!(
                    "设置文件解析失败，已临时使用默认设置（原文件未被修改）：{e}"
                )),
            ),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Settings::default(), None),
        Err(e) => (
            Settings::default(),
            Some(format!("设置文件读取失败，已临时使用默认设置：{e}")),
        ),
    }
}

/// 保存设置到磁盘（先备份旧文件，再写临时文件并原子替换）。仅写入非敏感信息。
pub fn save_settings(path: &Path, settings: &Settings) -> AppResult<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    if path.exists() {
        let backup = path.with_extension("json.bak");
        let _ = fs::copy(path, &backup);
    }
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| AppError::new("config_error", "设置序列化失败").with_detail(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_valid() {
        let mut s = Settings::default();
        assert!(s.validate().is_ok());
        assert_eq!(s.theme, "dark");
        assert_eq!(s.ai.timeout_secs, DEFAULT_TIMEOUT_SECS);
    }

    #[test]
    fn validates_and_clamps_ai_settings() {
        let mut s = Settings::default();
        s.ai.base_url = "  https://api.example.com/v1/  ".to_string();
        s.ai.timeout_secs = 100_000;
        s.ai.max_diff_chars = 1;
        s.validate().unwrap();
        assert_eq!(s.ai.base_url, "https://api.example.com/v1");
        assert_eq!(s.ai.timeout_secs, MAX_TIMEOUT_SECS);
        assert_eq!(s.ai.max_diff_chars, MIN_MAX_DIFF_CHARS);
    }

    #[test]
    fn rejects_non_http_base_url() {
        let mut s = Settings::default();
        s.ai.base_url = "ftp://example.com".to_string();
        assert_eq!(s.validate().unwrap_err().code, "invalid_input");
    }

    #[test]
    fn settings_json_never_contains_api_key_field() {
        let mut s = Settings::default();
        s.ai.base_url = "https://api.example.com".to_string();
        s.ai.model = "some-model".to_string();
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("apiKey"));
        assert!(!json.contains("api_key"));
        assert!(!json.contains("sk-"));
    }

    #[test]
    fn load_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let (s, warning) = load_settings(&path);
        assert!(warning.is_none());
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn load_corrupt_file_returns_default_with_warning_and_keeps_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, "{ this is not json").unwrap();
        let (s, warning) = load_settings(&path);
        assert_eq!(s, Settings::default());
        assert!(warning.is_some());
        // 原文件未被覆盖
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ this is not json");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut s = Settings {
            theme: "light".to_string(),
            ..Settings::default()
        };
        s.ai.model = "test-model".to_string();
        s.touch_recent("D:\\项目 空格", "项目 空格", 42);
        save_settings(&path, &s).unwrap();
        let (loaded, warning) = load_settings(&path);
        assert!(warning.is_none());
        assert_eq!(loaded, s);
    }

    #[test]
    fn touch_recent_dedupes_and_limits() {
        let mut s = Settings::default();
        for i in 0..15 {
            s.touch_recent(&format!("D:\\p{i}"), &format!("p{i}"), i as u64);
        }
        assert_eq!(s.recent_projects.len(), MAX_RECENT_PROJECTS);
        s.touch_recent("D:\\p5", "p5-updated", 999);
        assert_eq!(s.recent_projects[0].path, "D:\\p5");
        assert_eq!(s.recent_projects[0].name, "p5-updated");
        assert_eq!(
            s.recent_projects
                .iter()
                .filter(|p| p.path == "D:\\p5")
                .count(),
            1
        );
    }
}
