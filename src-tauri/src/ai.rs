//! AI 修改总结：仅接受用户主动触发，严格基于真实 Git 差异，失败不影响其他功能。
//!
//! 安全约定：
//! - API Key 只从系统安全凭据存储读取，绝不写入日志或配置文件；
//! - 发送前对疑似密钥/令牌做脱敏；
//! - 二进制、过大、冲突文件会被排除并说明原因；
//! - 请求支持超时与取消。

use crate::config::AiSettings;
use crate::error::{short, AppError, AppResult};
use crate::git;
use crate::state::AppState;
use regex::Regex;
use serde::Serialize;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// 单个文件进入请求的最大字符数
pub const PER_FILE_CHARS: usize = 20_000;
/// 单次请求最多包含的文件数
pub const MAX_FILES: usize = 120;

pub const SYSTEM_PROMPT: &str =
    "你是一名严谨的代码变更总结助手。你只能依据用户提供的 Git 差异内容进行总结，\
不得编造未出现的改动、功能、测试结论或文件。请使用简体中文，输出 Markdown，\
包含以下小节：## 修改目的、## 主要改动、## 关键文件变更、## 风险与注意事项。\
最后单独输出一行，格式为 `COMMIT: <提交信息>`：提交信息不超过 72 个字，动词开头的祈使句，\
不要包含句号，不要使用引号包裹。";

// ---------------------------------------------------------------------------
// 敏感信息脱敏
// ---------------------------------------------------------------------------

fn redaction_rules() -> &'static [(Regex, &'static str)] {
    static RULES: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    RULES.get_or_init(|| {
        vec![
            (
                Regex::new(r"-----BEGIN [A-Z ]{0,40}PRIVATE KEY-----[\s\S]*?-----END [A-Z ]{0,40}PRIVATE KEY-----").unwrap(),
                "[REDACTED_PRIVATE_KEY]",
            ),
            (
                Regex::new(r#"(?i)(api[_-]?key|apikey|secret|token|password|passwd|credential)(["']?\s*[:=]\s*)(["']?)([A-Za-z0-9_\-./+=]{8,})"#).unwrap(),
                "$1$2$3[REDACTED]",
            ),
            (Regex::new(r"(?i)bearer\s+[A-Za-z0-9._\-]{8,}").unwrap(), "Bearer [REDACTED]"),
            (Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(), "[REDACTED_AWS_KEY]"),
            (Regex::new(r"gh[pousr]_[A-Za-z0-9]{20,}").unwrap(), "[REDACTED_GITHUB_TOKEN]"),
            (Regex::new(r"sk-[A-Za-z0-9]{20,}").unwrap(), "[REDACTED_API_KEY]"),
        ]
    })
}

/// 对文本中疑似敏感内容做脱敏，返回（处理后的文本，替换次数）。
pub fn redact(text: &str) -> (String, usize) {
    let mut out = text.to_string();
    let mut count = 0usize;
    for (re, replacement) in redaction_rules() {
        count += re.find_iter(&out).count();
        out = re.replace_all(&out, *replacement).into_owned();
    }
    (out, count)
}

// ---------------------------------------------------------------------------
// 变更收集
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BundlePart {
    pub path: String,
    pub status: String,
    pub diff: String,
    pub redactions: usize,
}

#[derive(Debug, Clone)]
pub struct Bundle {
    pub parts: Vec<BundlePart>,
    pub skipped: Vec<SkippedFile>,
    pub total_chars: usize,
    pub truncated: bool,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkippedFile {
    pub path: String,
    pub reason: String,
}

fn status_label(x: &str, y: &str, untracked: bool) -> String {
    if untracked {
        return "未跟踪".to_string();
    }
    let ch = if x != " " { x } else { y };
    match ch.chars().next().unwrap_or('?') {
        'A' => "新增",
        'M' => "修改",
        'D' => "删除",
        'R' => "重命名",
        'C' => "复制",
        'T' => "类型变更",
        _ => "变更",
    }
    .to_string()
}

/// 收集本次需要发送给模型的真实差异。
///
/// `scope`：
/// - `staged`：仅已暂存内容
/// - `all`：全部工作区变更（未跟踪文件包含内容，冲突文件跳过）
pub async fn collect(project: &Path, scope: &str, max_chars: usize) -> AppResult<Bundle> {
    if scope != "staged" && scope != "all" {
        return Err(AppError::new(
            "invalid_input",
            format!("不支持的总结范围：{scope}"),
        ));
    }

    let st = git::status(project).await?;
    if !st.is_repo {
        return Err(AppError::new(
            "not_a_repository",
            "当前项目不是 Git 仓库，无法总结修改",
        ));
    }

    let mut bundle = Bundle {
        parts: Vec::new(),
        skipped: Vec::new(),
        total_chars: 0,
        truncated: false,
    };

    for entry in &st.entries {
        // 选择该条目在本次范围中的处理方式
        let (mode, untracked): (&str, bool) = if scope == "staged" {
            if entry.conflicted {
                bundle.skipped.push(SkippedFile {
                    path: entry.path.clone(),
                    reason: "存在未解决的冲突".to_string(),
                });
                continue;
            }
            if !entry.staged() {
                continue;
            }
            ("staged", false)
        } else {
            if entry.conflicted {
                bundle.skipped.push(SkippedFile {
                    path: entry.path.clone(),
                    reason: "存在未解决的冲突".to_string(),
                });
                continue;
            }
            if entry.untracked {
                ("worktree", true)
            } else {
                ("head", false)
            }
        };

        if bundle.parts.len() >= MAX_FILES {
            bundle.skipped.push(SkippedFile {
                path: entry.path.clone(),
                reason: format!("超出单次请求的文件数量上限（{MAX_FILES}）"),
            });
            bundle.truncated = true;
            continue;
        }
        if bundle.total_chars >= max_chars {
            bundle.skipped.push(SkippedFile {
                path: entry.path.clone(),
                reason: "超出本次请求长度限制".to_string(),
            });
            bundle.truncated = true;
            continue;
        }

        let payload = git::diff::file_diff(project, &entry.path, mode, untracked, false).await?;
        match payload.kind.as_str() {
            "text" => {
                let raw = payload.text.unwrap_or_default();
                let (mut diff, redactions) = redact(&raw);
                let mut truncated_here = payload.truncated;
                if diff.chars().count() > PER_FILE_CHARS {
                    diff = truncate_chars(&diff, PER_FILE_CHARS);
                    truncated_here = true;
                }
                if truncated_here {
                    bundle.truncated = true;
                }
                let chars = diff.chars().count();
                bundle.total_chars += chars;
                let mut status = status_label(&entry.x, &entry.y, entry.untracked);
                if truncated_here {
                    status.push_str("（差异已截断）");
                }
                bundle.parts.push(BundlePart {
                    path: entry.path.clone(),
                    status,
                    diff,
                    redactions,
                });
            }
            "binary" => bundle.skipped.push(SkippedFile {
                path: entry.path.clone(),
                reason: "二进制文件".to_string(),
            }),
            "too_large" => bundle.skipped.push(SkippedFile {
                path: entry.path.clone(),
                reason: "文件过大".to_string(),
            }),
            "empty" => {}
            other => bundle.skipped.push(SkippedFile {
                path: entry.path.clone(),
                reason: format!("无法预览（{other}）"),
            }),
        }
    }

    Ok(bundle)
}

fn truncate_chars(text: &str, max: usize) -> String {
    let mut out: String = text.chars().take(max).collect();
    out.push_str("\n…（单文件差异过长，已截断）");
    out
}

fn build_prompt(st: &git::status::GitStatus, bundle: &Bundle, scope: &str) -> String {
    let mut out = String::new();
    out.push_str("以下是来自真实 Git 仓库的变更数据，请据此总结。\n\n");
    out.push_str("## 仓库状态\n");
    out.push_str(&format!(
        "- 分支：{}\n",
        st.branch.as_deref().unwrap_or("(未知)")
    ));
    out.push_str(&format!(
        "- 总结范围：{}\n",
        if scope == "staged" {
            "仅已暂存内容"
        } else {
            "全部工作区变更"
        }
    ));
    out.push_str(&format!("- 本次包含文件数：{}\n", bundle.parts.len()));
    for part in &bundle.parts {
        out.push_str(&format!("- [{}] {}\n", part.status, part.path));
    }
    if !bundle.skipped.is_empty() {
        out.push_str("- 未包含的文件：\n");
        for s in &bundle.skipped {
            out.push_str(&format!("  - {}（{}）\n", s.path, s.reason));
        }
    }
    if bundle.truncated {
        out.push_str("- 注意：部分差异因长度限制被截断。\n");
    }

    out.push_str("\n## 文件差异（疑似密钥/令牌已脱敏）\n");
    for part in &bundle.parts {
        out.push_str(&format!(
            "\n### {} [{}]\n```diff\n{}\n```\n",
            part.path, part.status, part.diff
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// OpenAI 兼容 API 调用
// ---------------------------------------------------------------------------

/// 根据用户填写的 Base URL 推断候选的 Chat Completions 地址。
///
/// 兼容常见写法：以 `/v1` 结尾、直接包含 `/chat/completions`、或不带版本号。
pub fn endpoint_candidates(base_url: &str) -> Vec<String> {
    let b = base_url.trim().trim_end_matches('/');
    if b.contains("/chat/completions") {
        vec![b.to_string()]
    } else if b.ends_with("/v1") {
        vec![format!("{b}/chat/completions")]
    } else {
        vec![
            format!("{b}/v1/chat/completions"),
            format!("{b}/chat/completions"),
        ]
    }
}

#[derive(Debug, Clone)]
pub struct ChatReply {
    pub content: String,
    pub endpoint: String,
}

fn extract_content(v: &serde_json::Value) -> Option<String> {
    let message = v.get("choices")?.get(0)?.get("message")?;
    if let Some(s) = message.get("content").and_then(|c| c.as_str()) {
        if !s.trim().is_empty() {
            return Some(s.to_string());
        }
    }
    // 部分供应商返回 content 为内容片段数组
    if let Some(parts) = message.get("content").and_then(|c| c.as_array()) {
        let mut out = String::new();
        for p in parts {
            if let Some(t) = p.get("text").and_then(|t| t.as_str()) {
                out.push_str(t);
            }
        }
        if !out.trim().is_empty() {
            return Some(out);
        }
    }
    // 推理模型可能只有 reasoning_content
    if let Some(r) = message.get("reasoning_content").and_then(|c| c.as_str()) {
        if !r.trim().is_empty() {
            return Some(r.to_string());
        }
    }
    None
}

fn extract_error_message(text: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    if let Some(m) = v
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
    {
        return Some(m.to_string());
    }
    v.get("message")
        .and_then(|m| m.as_str())
        .map(|m| m.to_string())
}

/// 从模型输出中提取建议的提交信息（最后一行 `COMMIT: xxx`）。
fn extract_commit_message(text: &str) -> (String, Option<String>) {
    let mut lines: Vec<&str> = text.lines().collect();
    let mut commit = None;
    for i in (0..lines.len()).rev() {
        let line = lines[i].trim();
        let lower = line.to_ascii_lowercase();
        let value = if lower.starts_with("commit:") {
            line[7..].trim()
        } else if lower.starts_with("commit：") {
            line["commit：".len()..].trim()
        } else {
            continue;
        };
        let cleaned = value
            .trim_matches(|c| c == '"' || c == '`' || c == '“' || c == '”' || c == '*' || c == ' ')
            .trim();
        if !cleaned.is_empty() {
            commit = Some(cleaned.to_string());
            lines.remove(i);
        }
        break;
    }
    (lines.join("\n").trim().to_string(), commit)
}

/// 发送一次 Chat Completions 请求（OpenAI 兼容）。
pub async fn chat_completion(
    settings: &AiSettings,
    api_key: &str,
    system: &str,
    user: &str,
) -> AppResult<ChatReply> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(settings.timeout_secs))
        .build()
        .map_err(|e| {
            AppError::new("ai_request_failed", "无法创建 HTTP 客户端").with_detail(e.to_string())
        })?;

    let body = serde_json::json!({
        "model": settings.model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user}
        ],
        "stream": false
    });

    let candidates = endpoint_candidates(&settings.base_url);
    let mut last_err: Option<AppError> = None;

    for (idx, url) in candidates.iter().enumerate() {
        let resp = client
            .post(url)
            .header("Authorization", format!("Bearer {api_key}"))
            .json(&body)
            .send()
            .await;

        match resp {
            Ok(r) => {
                let status = r.status();
                let text = r.text().await.unwrap_or_default();
                if status.is_success() {
                    let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
                        let body_excerpt = short(&redact(&text).0, 500);
                        AppError::new("ai_request_failed", "AI 服务返回了无法解析的响应")
                            .with_detail(format!("{e}；响应片段：{body_excerpt}"))
                    })?;
                    if let Some(content) = extract_content(&value) {
                        return Ok(ChatReply {
                            content,
                            endpoint: url.clone(),
                        });
                    }
                    return Err(AppError::new(
                        "ai_request_failed",
                        "AI 服务返回成功，但没有文本内容（模型可能只返回了推理内容或空响应）",
                    )
                    .with_detail(short(&redact(&text).0, 1500)));
                }

                if (status.as_u16() == 404 || status.as_u16() == 405) && idx + 1 < candidates.len()
                {
                    last_err = Some(
                        AppError::new(
                            "ai_request_failed",
                            format!("AI 服务返回 HTTP {}（尝试其他地址）", status.as_u16()),
                        )
                        .with_detail(short(&redact(&text).0, 800)),
                    );
                    continue;
                }

                let detail = extract_error_message(&text)
                    .map(|m| short(&redact(&m).0, 1500))
                    .unwrap_or_else(|| short(&redact(&text).0, 1500));
                return Err(AppError::new(
                    "ai_request_failed",
                    format!("AI 服务返回错误（HTTP {}）", status.as_u16()),
                )
                .with_detail(detail));
            }
            Err(e) => {
                let err = if e.is_timeout() {
                    AppError::new(
                        "ai_timeout",
                        format!(
                            "AI 请求超时（{} 秒），可在设置中调大超时时间",
                            settings.timeout_secs
                        ),
                    )
                    .with_detail(e.to_string())
                } else if e.is_connect() {
                    AppError::new(
                        "ai_request_failed",
                        "无法连接到 AI 服务，请检查 Base URL 与网络连接",
                    )
                    .with_detail(e.to_string())
                } else {
                    AppError::new("ai_request_failed", "AI 请求失败").with_detail(e.to_string())
                };
                last_err = Some(err);
                break;
            }
        }
    }

    Err(last_err.unwrap_or_else(|| AppError::new("ai_request_failed", "AI 请求失败")))
}

// ---------------------------------------------------------------------------
// Tauri 命令
// ---------------------------------------------------------------------------

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PreviewFile {
    pub path: String,
    pub status: String,
    pub chars: usize,
    pub redactions: usize,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiPreview {
    pub scope: String,
    pub files: Vec<PreviewFile>,
    pub skipped: Vec<SkippedFile>,
    pub total_chars: usize,
    pub truncated: bool,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiSummary {
    pub summary: String,
    pub commit_message: Option<String>,
    pub files_included: usize,
    pub files_skipped: usize,
    pub total_chars: usize,
    pub truncated: bool,
    pub model: String,
    pub redactions: usize,
    pub elapsed_ms: u64,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiTestResult {
    pub ok: bool,
    pub model: String,
    pub endpoint: String,
    pub latency_ms: u64,
}

fn ensure_configured(settings: &AiSettings) -> AppResult<()> {
    if settings.base_url.trim().is_empty() || settings.model.trim().is_empty() {
        return Err(AppError::new(
            "ai_not_configured",
            "尚未配置 AI（Base URL 与模型名称），请在设置中完成配置后再试",
        ));
    }
    Ok(())
}

fn require_api_key() -> AppResult<String> {
    crate::credentials::get_api_key()?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| {
            AppError::new(
                "ai_not_configured",
                "尚未配置 API Key，请在设置中保存 API Key（存储于系统安全凭据中）",
            )
        })
}

/// 预览即将发送给模型的内容范围（不发起任何网络请求）。
#[tauri::command]
pub async fn ai_preview(state: tauri::State<'_, AppState>, scope: String) -> AppResult<AiPreview> {
    let root = state.project_root()?;
    let settings = state.settings_snapshot();
    let bundle = collect(&root, &scope, settings.ai.max_diff_chars).await?;
    Ok(AiPreview {
        scope,
        files: bundle
            .parts
            .iter()
            .map(|p| PreviewFile {
                path: p.path.clone(),
                status: p.status.clone(),
                chars: p.diff.chars().count(),
                redactions: p.redactions,
            })
            .collect(),
        skipped: bundle.skipped,
        total_chars: bundle.total_chars,
        truncated: bundle.truncated,
    })
}

/// 执行 AI 总结（用户主动触发；支持取消与超时）。
#[tauri::command]
pub async fn ai_summarize(
    state: tauri::State<'_, AppState>,
    scope: String,
) -> AppResult<AiSummary> {
    let root = state.project_root()?;
    let settings = state.settings_snapshot();
    ensure_configured(&settings.ai)?;
    let key = require_api_key()?;

    let st = git::status(&root).await?;
    if !st.is_repo {
        return Err(AppError::new(
            "not_a_repository",
            "当前项目不是 Git 仓库，无法总结修改",
        ));
    }

    let bundle = collect(&root, &scope, settings.ai.max_diff_chars).await?;
    if bundle.parts.is_empty() {
        return Err(AppError::new(
            "invalid_input",
            "没有可用于总结的变更内容（可能全部为二进制、冲突或超出限制）",
        ));
    }

    let prompt = build_prompt(&st, &bundle, &scope);

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    state.set_ai_cancel(Some(tx));
    let started = Instant::now();
    let result = tokio::select! {
        r = chat_completion(&settings.ai, &key, SYSTEM_PROMPT, &prompt) => r,
        _ = rx => Err(AppError::new("ai_cancelled", "AI 请求已取消")),
    };
    state.set_ai_cancel(None);

    let reply = result?;
    let (summary, commit_message) = extract_commit_message(&reply.content);
    if summary.trim().is_empty() && commit_message.is_none() {
        return Err(AppError::new("ai_request_failed", "AI 返回的内容为空"));
    }

    Ok(AiSummary {
        summary,
        commit_message,
        files_included: bundle.parts.len(),
        files_skipped: bundle.skipped.len(),
        total_chars: bundle.total_chars,
        truncated: bundle.truncated,
        model: settings.ai.model.clone(),
        redactions: bundle.parts.iter().map(|p| p.redactions).sum(),
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

/// 取消进行中的 AI 请求。
#[tauri::command]
pub fn ai_cancel(state: tauri::State<'_, AppState>) -> bool {
    state.fire_ai_cancel()
}

/// 测试与 AI 服务的连通性（使用已保存的配置与 Key）。
#[tauri::command]
pub async fn ai_test_connection(state: tauri::State<'_, AppState>) -> AppResult<AiTestResult> {
    let settings = state.settings_snapshot();
    ensure_configured(&settings.ai)?;
    let key = require_api_key()?;

    let started = Instant::now();
    let reply = chat_completion(
        &settings.ai,
        &key,
        "你是一个连接测试助手，只输出简短回复。",
        "连接测试：请只回复 ok。",
    )
    .await?;

    Ok(AiTestResult {
        ok: true,
        model: settings.ai.model.clone(),
        endpoint: reply.endpoint,
        latency_ms: started.elapsed().as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_common_secret_patterns() {
        let text = r#"
const apiKey = "sk-abcdefghijklmnopqrstuvwxyz123456";
password: "hunter2secret"
Authorization: Bearer abcdefgh12345678
AWS_ACCESS_KEY = AKIAIOSFODNN7EXAMPLE
token = ghp_abcdefghijklmnopqrstuvwxyz0123
"#;
        let (out, count) = redact(text);
        assert!(count >= 4, "应至少替换 4 处，实际 {count}");
        assert!(!out.contains("sk-abcdefghijklmnopqrstuvwxyz123456"));
        assert!(!out.contains("hunter2secret"));
        assert!(!out.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!out.contains("ghp_abcdefghijklmnopqrstuvwxyz0123"));
        assert!(out.contains("[REDACTED"));
    }

    #[test]
    fn redacts_private_key_blocks() {
        let text = "-----BEGIN RSA PRIVATE KEY-----\nMIIEow...\n-----END RSA PRIVATE KEY-----";
        let (out, count) = redact(text);
        assert_eq!(count, 1);
        assert!(out.contains("[REDACTED_PRIVATE_KEY]"));
        assert!(!out.contains("MIIEow"));
    }

    #[test]
    fn does_not_redact_normal_code() {
        let text = "const userName = \"zhangsan\";\nfunction tokenizer(input) { return input; }\n";
        let (out, count) = redact(text);
        assert_eq!(count, 0);
        assert_eq!(out, text);
    }

    #[test]
    fn builds_endpoint_candidates() {
        assert_eq!(
            endpoint_candidates("https://api.openai.com"),
            vec![
                "https://api.openai.com/v1/chat/completions",
                "https://api.openai.com/chat/completions"
            ]
        );
        assert_eq!(
            endpoint_candidates("https://api.deepseek.com/v1/"),
            vec!["https://api.deepseek.com/v1/chat/completions"]
        );
        assert_eq!(
            endpoint_candidates("http://localhost:11434/v1/chat/completions"),
            vec!["http://localhost:11434/v1/chat/completions"]
        );
    }

    #[test]
    fn extracts_commit_message_from_answer() {
        let text = "## 修改目的\n修复登录问题\n\n## 主要改动\n- 调整 token 校验\n\nCOMMIT: 修复登录 token 过期校验错误";
        let (summary, commit) = extract_commit_message(text);
        assert_eq!(commit.as_deref(), Some("修复登录 token 过期校验错误"));
        assert!(summary.contains("## 修改目的"));
        assert!(!summary.contains("COMMIT:"));
    }

    #[test]
    fn extracts_commit_message_with_fullwidth_colon_and_quotes() {
        let text = "总结内容\nCOMMIT：“修复配置加载失败”";
        let (_, commit) = extract_commit_message(text);
        assert_eq!(commit.as_deref(), Some("修复配置加载失败"));
    }

    #[test]
    fn handles_answers_without_commit_line() {
        let (summary, commit) = extract_commit_message("只有总结，没有建议的提交信息");
        assert!(commit.is_none());
        assert_eq!(summary, "只有总结，没有建议的提交信息");
    }

    #[test]
    fn parses_openai_style_content() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"choices":[{"message":{"role":"assistant","content":"hello"}}]}"#,
        )
        .unwrap();
        assert_eq!(extract_content(&v).as_deref(), Some("hello"));
    }

    #[test]
    fn parses_content_parts_and_reasoning_fallback() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"choices":[{"message":{"content":[{"type":"text","text":"part1"},{"type":"text","text":"part2"}]}}]}"#,
        )
        .unwrap();
        assert_eq!(extract_content(&v).as_deref(), Some("part1part2"));

        let v2: serde_json::Value = serde_json::from_str(
            r#"{"choices":[{"message":{"content":"","reasoning_content":"thinking..."}}]}"#,
        )
        .unwrap();
        assert_eq!(extract_content(&v2).as_deref(), Some("thinking..."));
    }
}
