use crate::config::{KEYRING_ACCOUNT, KEYRING_SERVICE};
use crate::error::{AppError, AppResult};

/// API Key 的读写封装：优先使用操作系统安全凭据存储（Windows Credential Manager /
/// macOS Keychain / Linux keyutils）。
///
/// 明确约束：
/// - 任何情况下都不得把 Key 明文写入设置文件或日志；
/// - 安全存储不可用时必须返回错误，由上层提示用户，不允许静默降级为明文存储。
#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
mod platform {
    use super::*;

    fn entry() -> AppResult<keyring::Entry> {
        keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).map_err(|e| {
            AppError::new(
                "credential_store_failed",
                "系统安全凭据存储不可用，API Key 未保存",
            )
            .with_detail(e.to_string())
        })
    }

    pub fn save_api_key(key: &str) -> AppResult<()> {
        entry()?.set_password(key).map_err(|e| {
            AppError::new(
                "credential_store_failed",
                "写入系统安全凭据存储失败，API Key 未保存（不会以明文保存到配置文件）",
            )
            .with_detail(e.to_string())
        })
    }

    pub fn get_api_key() -> AppResult<Option<String>> {
        match entry()?.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(
                AppError::new("credential_store_failed", "读取系统安全凭据存储失败")
                    .with_detail(e.to_string()),
            ),
        }
    }

    pub fn delete_api_key() -> AppResult<()> {
        match entry()?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AppError::new(
                "credential_store_failed",
                "删除系统安全凭据存储中的 API Key 失败",
            )
            .with_detail(e.to_string())),
        }
    }
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
mod platform {
    use super::*;

    const UNSUPPORTED: &str = "当前平台暂不支持系统安全凭据存储，API Key 不会被保存";

    pub fn save_api_key(_key: &str) -> AppResult<()> {
        Err(AppError::new("credential_store_failed", UNSUPPORTED))
    }

    pub fn get_api_key() -> AppResult<Option<String>> {
        Err(AppError::new("credential_store_failed", UNSUPPORTED))
    }

    pub fn delete_api_key() -> AppResult<()> {
        Err(AppError::new("credential_store_failed", UNSUPPORTED))
    }
}

pub use platform::{delete_api_key, get_api_key, save_api_key};

/// 返回 Key 是否已配置（读失败时把错误一并返回，供 UI 展示安全存储异常）。
pub fn api_key_status() -> (bool, Option<String>) {
    match get_api_key() {
        Ok(Some(k)) => (!k.trim().is_empty(), None),
        Ok(None) => (false, None),
        Err(e) => (false, Some(e.message)),
    }
}
