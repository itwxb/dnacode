use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::fmt;

/// 统一的应用错误类型：面向前端返回 `{ code, message, detail }`。
///
/// - `code`：稳定的错误代码，供前端映射为交互（如需要确认、需要重试、需要引导）
/// - `message`：面向用户的中文说明
/// - `detail`：原始错误文本（Git stderr、IO 错误等），供诊断使用
///
/// 约定：detail 中禁止放入 API Key、Token、密码等敏感内容。
#[derive(Debug, Clone)]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        let detail = detail.into();
        if !detail.trim().is_empty() {
            self.detail = Some(detail);
        }
        self
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("AppError", 3)?;
        s.serialize_field("code", &self.code)?;
        s.serialize_field("message", &self.message)?;
        s.serialize_field("detail", &self.detail)?;
        s.end()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        use std::io::ErrorKind::*;
        let (code, message) = match e.kind() {
            NotFound => ("not_found", "文件或目录不存在"),
            PermissionDenied => (
                "permission_denied",
                "没有访问权限（文件可能被占用，或系统拒绝了本次访问）",
            ),
            _ => ("io_error", "文件操作失败"),
        };
        AppError::new(code, message).with_detail(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::new("config_error", "配置数据解析失败").with_detail(e.to_string())
    }
}

/// 截断长文本（按字符计数），保留开头内容并附加省略提示。
pub fn short(text: &str, max_chars: usize) -> String {
    let t = text.trim();
    if t.chars().count() <= max_chars {
        return t.to_string();
    }
    let cut: String = t.chars().take(max_chars).collect();
    format!("{cut}…（已截断）")
}

/// 将 Git 命令的失败输出映射为用户可理解的错误，同时完整保留原始输出供排查。
pub fn map_git_failure(context: &str, stderr: &str, stdout: &str) -> AppError {
    let lower = stderr.to_lowercase();
    let raw = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    let detail = short(raw, 4000);

    let (code, message) = if lower.contains("not a git repository") {
        (
            "not_a_repository",
            format!("{context}失败：当前项目不是 Git 仓库"),
        )
    } else if lower.contains("could not read username")
        || lower.contains("authentication failed")
        || lower.contains("permission denied (publickey)")
        || lower.contains("invalid username or password")
        || lower.contains("terminal prompts disabled")
        || lower.contains("could not read password")
        || lower.contains("no credential helper")
    {
        (
            "auth_failed",
            format!(
                "{context}失败：远程认证未通过。请检查系统的 Git 凭据助手、SSH 密钥或访问令牌配置"
            ),
        )
    } else if lower.contains("could not resolve host")
        || lower.contains("unable to access")
        || lower.contains("connection timed out")
        || lower.contains("operation timed out")
        || lower.contains("network is unreachable")
        || lower.contains("connection refused")
        || lower.contains("failed to connect")
        || lower.contains("timeout")
    {
        (
            "network_failed",
            format!("{context}失败：网络连接异常，无法访问远程仓库"),
        )
    } else if lower.contains("[rejected]")
        || lower.contains("non-fast-forward")
        || lower.contains("fetch first")
        || lower.contains("updates were rejected")
    {
        (
            "push_rejected",
            format!("{context}失败：远程仓库拒绝了本次推送（通常因为远程有新的提交）。请先获取远程更新后再推送"),
        )
    } else if lower.contains("please tell me who you are")
        || lower.contains("unable to auto-detect email address")
        || lower.contains("empty ident name")
    {
        (
            "no_identity",
            format!("{context}失败：未配置 Git 提交身份（user.name / user.email）"),
        )
    } else if lower.contains("would be overwritten")
        || lower.contains("please commit your changes or stash them")
    {
        (
            "checkout_blocked",
            format!("{context}失败：本地修改会被覆盖，Git 拒绝了本次操作。请先提交或处理这些修改"),
        )
    } else {
        ("git_failed", format!("{context}失败"))
    };

    AppError::new(code, message).with_detail(detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_not_a_repository() {
        let e = map_git_failure(
            "获取状态",
            "fatal: not a git repository (or any of the parent directories): .git\n",
            "",
        );
        assert_eq!(e.code, "not_a_repository");
        assert!(e.detail.unwrap().contains("not a git repository"));
    }

    #[test]
    fn maps_auth_failure() {
        let e = map_git_failure(
            "推送",
            "fatal: Authentication failed for 'https://example.com/repo.git/'",
            "",
        );
        assert_eq!(e.code, "auth_failed");
    }

    #[test]
    fn maps_network_failure() {
        let e = map_git_failure(
            "推送",
            "fatal: unable to access 'https://example.com/': Could not resolve host: example.com",
            "",
        );
        assert_eq!(e.code, "network_failed");
    }

    #[test]
    fn maps_rejected_push() {
        let e = map_git_failure(
            "推送",
            " ! [rejected]        main -> main (fetch first)\nerror: failed to push some refs",
            "",
        );
        assert_eq!(e.code, "push_rejected");
    }

    #[test]
    fn maps_missing_identity() {
        let e = map_git_failure(
            "提交",
            "*** Please tell me who you are.\n\nRun\n\n  git config --global user.email \"you@example.com\"\n",
            "",
        );
        assert_eq!(e.code, "no_identity");
    }

    #[test]
    fn maps_checkout_blocked() {
        let e = map_git_failure(
            "切换分支",
            "error: Your local changes to the following files would be overwritten by checkout:\n\tsrc/main.rs\nPlease commit your changes or stash them before you switch branches.\nAborting",
            "",
        );
        assert_eq!(e.code, "checkout_blocked");
    }

    #[test]
    fn falls_back_to_generic_git_error_and_keeps_detail() {
        let e = map_git_failure("提交", "fatal: some unexpected failure", "");
        assert_eq!(e.code, "git_failed");
        assert!(e.detail.unwrap().contains("unexpected failure"));
    }

    #[test]
    fn short_keeps_original_when_small_enough() {
        assert_eq!(short("  hello  ", 10), "hello");
        assert!(short(&"字".repeat(30), 10).contains("已截断"));
    }
}
