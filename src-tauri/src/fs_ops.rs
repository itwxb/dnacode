use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 单个可编辑文件的体积上限（5 MB）。超过后拒绝打开，避免编辑器卡死。
pub const MAX_EDITABLE_BYTES: u64 = 5 * 1024 * 1024;
const BINARY_SNIFF_BYTES: usize = 8192;

/// 文件树中默认忽略的目录名（集中配置，仅忽略目录本身，不影响同名普通文件）。
///
/// 这些目录属于依赖缓存、构建产物或工具元数据，通常不需要在编辑器中浏览；
/// 注意不要在此列表中加入用户源码目录（如 `src`、`vendor` 等易混淆名称）。
pub const IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "dist",
    "build",
    "coverage",
    "target",
    "__pycache__",
    ".next",
    ".nuxt",
    ".output",
    ".svelte-kit",
    ".turbo",
    ".parcel-cache",
    ".cache",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
    ".gradle",
    ".venv",
    "venv",
    ".idea",
];

pub fn is_ignored_dir(name: &str) -> bool {
    IGNORED_DIRS.iter().any(|i| name.eq_ignore_ascii_case(i))
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DirEntryDto {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    /// 以 `.` 开头的隐藏文件/目录（前端以较暗的样式展示，而不是直接隐藏）
    pub hidden: bool,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileContentDto {
    pub path: String,
    pub content: String,
    pub mtime_ms: u64,
    pub size: u64,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileMetaDto {
    pub exists: bool,
    pub mtime_ms: Option<u64>,
    pub size: Option<u64>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WriteResultDto {
    pub mtime_ms: u64,
    pub size: u64,
}

fn canonicalize_or_err(p: &Path) -> AppResult<PathBuf> {
    dunce::canonicalize(p).map_err(|e| {
        let detail = format!("{}: {e}", p.display());
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::new("not_found", "文件或目录不存在").with_detail(detail)
        } else {
            AppError::from(e).with_detail(detail)
        }
    })
}

fn has_git_component(rel: &Path) -> bool {
    rel.components()
        .any(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case(".git"))
}

/// 校验目标路径：必须是绝对路径、位于项目根目录内、且不在 `.git` 目录中。
///
/// `allow_missing_leaf` 为 true 时允许文件本身尚不存在（例如保存、探测元数据），
/// 此时会校验其上级目录。
fn ensure_inside(candidate: &str, root: &Path, allow_missing_leaf: bool) -> AppResult<PathBuf> {
    let raw = PathBuf::from(candidate);
    if !raw.is_absolute() {
        return Err(AppError::new("path_invalid", "路径必须是绝对路径"));
    }

    let canonical = if allow_missing_leaf {
        let parent = raw
            .parent()
            .ok_or_else(|| AppError::new("path_invalid", "路径缺少上级目录"))?;
        let file_name = raw
            .file_name()
            .ok_or_else(|| AppError::new("path_invalid", "路径缺少文件名"))?;
        canonicalize_or_err(parent)?.join(file_name)
    } else {
        canonicalize_or_err(&raw)?
    };

    if !canonical.starts_with(root) {
        return Err(AppError::new(
            "path_invalid",
            "路径超出当前项目范围，已拒绝访问",
        ));
    }
    if let Ok(rel) = canonical.strip_prefix(root) {
        if has_git_component(rel) {
            return Err(AppError::new("path_invalid", "不允许访问 .git 目录内部"));
        }
    }
    Ok(canonical)
}

fn mtime_ms(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(BINARY_SNIFF_BYTES).any(|b| *b == 0)
}

/// 列出目录下的直接子项（按需加载，不递归）。
pub fn list_directory(path: &str, root: &Path) -> AppResult<Vec<DirEntryDto>> {
    let dir = ensure_inside(path, root, false)?;
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let is_symlink = file_type.is_symlink();
        let is_dir = if is_symlink {
            fs::metadata(entry.path())
                .map(|m| m.is_dir())
                .unwrap_or(false)
        } else {
            file_type.is_dir()
        };
        if is_dir && is_ignored_dir(&name) {
            continue;
        }
        out.push(DirEntryDto {
            name: name.clone(),
            path: entry.path().to_string_lossy().to_string(),
            is_dir,
            is_symlink,
            hidden: name.starts_with('.'),
        });
    }
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

/// 读取文本文件，拒绝二进制文件和超大文件。
pub fn read_text_file(path: &str, root: &Path) -> AppResult<FileContentDto> {
    let p = ensure_inside(path, root, false)?;
    let meta = fs::metadata(&p)?;
    if !meta.is_file() {
        return Err(AppError::new("invalid_input", "目标不是普通文件"));
    }
    if meta.len() > MAX_EDITABLE_BYTES {
        return Err(AppError::new(
            "file_too_large",
            format!(
                "文件过大（{:.1} MB），超过 {} MB 的可编辑限制，已拒绝打开",
                meta.len() as f64 / 1_048_576.0,
                MAX_EDITABLE_BYTES / 1_048_576
            ),
        ));
    }
    let bytes = fs::read(&p)?;
    if looks_binary(&bytes) {
        return Err(AppError::new(
            "binary_file",
            "这是二进制文件，无法以文本方式打开",
        ));
    }
    let content = String::from_utf8(bytes)
        .map_err(|_| AppError::new("not_utf8", "文件不是 UTF-8 文本，暂不支持显示和编辑"))?;
    Ok(FileContentDto {
        path: p.to_string_lossy().to_string(),
        content,
        mtime_ms: mtime_ms(&meta),
        size: meta.len(),
    })
}

/// 写入文本文件。
///
/// 冲突保护：
/// - `force == false` 且磁盘修改时间与 `expected_mtime_ms` 不一致时返回 `stale_file`，
///   由前端询问用户后再以 `force = true` 覆盖；
/// - 文件被外部删除时返回 `file_missing`，同样需要用户确认后才重新创建。
pub fn write_text_file(
    path: &str,
    root: &Path,
    content: &str,
    expected_mtime_ms: Option<u64>,
    force: bool,
) -> AppResult<WriteResultDto> {
    let p = ensure_inside(path, root, true)?;

    match fs::metadata(&p).ok().filter(|m| m.is_file()) {
        Some(meta) => {
            if !force {
                if let Some(expected) = expected_mtime_ms {
                    let current = mtime_ms(&meta);
                    if current != expected {
                        return Err(AppError::new(
                            "stale_file",
                            "磁盘上的文件已被外部修改，本次保存已取消。请选择“覆盖”或“重新加载”，避免静默覆盖外部改动",
                        )
                        .with_detail(format!(
                            "磁盘修改时间戳：{current}，编辑器记录的修改时间戳：{expected}"
                        )));
                    }
                }
            }
        }
        None => {
            if !force {
                return Err(AppError::new(
                    "file_missing",
                    "文件已不存在（可能被外部删除或重命名），本次保存已取消。确认后可重新创建该文件",
                ));
            }
        }
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&p)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    let meta = fs::metadata(&p)?;
    Ok(WriteResultDto {
        mtime_ms: mtime_ms(&meta),
        size: meta.len(),
    })
}

/// 查询文件元数据（用于检测外部修改 / 外部删除）。
pub fn file_meta(path: &str, root: &Path) -> AppResult<FileMetaDto> {
    let p = match ensure_inside(path, root, true) {
        Ok(p) => p,
        Err(e) if e.code == "not_found" => {
            return Ok(FileMetaDto {
                exists: false,
                mtime_ms: None,
                size: None,
            })
        }
        Err(e) => return Err(e),
    };
    match fs::metadata(&p) {
        Ok(meta) if meta.is_file() => Ok(FileMetaDto {
            exists: true,
            mtime_ms: Some(mtime_ms(&meta)),
            size: Some(meta.len()),
        }),
        _ => Ok(FileMetaDto {
            exists: false,
            mtime_ms: None,
            size: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        (dir, root)
    }

    #[test]
    fn lists_directories_first_and_skips_ignored() {
        let (_dir, root) = setup();
        fs::create_dir(root.join("src")).unwrap();
        fs::create_dir(root.join("node_modules")).unwrap();
        fs::create_dir(root.join(".git")).unwrap();
        fs::write(root.join("b.txt"), "b").unwrap();
        fs::write(root.join("a.txt"), "a").unwrap();
        fs::write(root.join(".hidden"), "h").unwrap();

        let entries = list_directory(&root.to_string_lossy(), &root).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["src", ".hidden", "a.txt", "b.txt"]);
        assert!(entries[0].is_dir);
        assert!(entries.iter().any(|e| e.name == ".hidden" && e.hidden));
    }

    #[test]
    fn reads_utf8_with_chinese_and_spaces() {
        let (_dir, root) = setup();
        let sub = root.join("中文 目录");
        fs::create_dir(&sub).unwrap();
        let file = sub.join("示例 文件.txt");
        fs::write(&file, "你好，DNACode").unwrap();

        let content = read_text_file(&file.to_string_lossy(), &root).unwrap();
        assert_eq!(content.content, "你好，DNACode");
        assert!(content.mtime_ms > 0);
    }

    #[test]
    fn rejects_binary_and_non_utf8() {
        let (_dir, root) = setup();
        let bin = root.join("bin.dat");
        fs::write(&bin, [0u8, 1, 2, 3]).unwrap();
        assert_eq!(
            read_text_file(&bin.to_string_lossy(), &root)
                .unwrap_err()
                .code,
            "binary_file"
        );

        let gbk = root.join("gbk.txt");
        fs::write(&gbk, [0xC4u8, 0xE3, 0xBA, 0xC3]).unwrap(); // GBK 的“你好”
        assert_eq!(
            read_text_file(&gbk.to_string_lossy(), &root)
                .unwrap_err()
                .code,
            "not_utf8"
        );
    }

    #[test]
    fn rejects_paths_outside_root_and_git_dir() {
        let (dir, root) = setup();
        let outside = dir.path().parent().unwrap().join("outside.txt");
        fs::write(&outside, "x").unwrap();
        assert_eq!(
            read_text_file(&outside.to_string_lossy(), &root)
                .unwrap_err()
                .code,
            "path_invalid"
        );

        let git_file = root.join(".git").join("config");
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(&git_file, "x").unwrap();
        assert_eq!(
            read_text_file(&git_file.to_string_lossy(), &root)
                .unwrap_err()
                .code,
            "path_invalid"
        );
    }

    #[test]
    fn write_detects_stale_mtime_and_allows_force() {
        let (_dir, root) = setup();
        let file = root.join("note.txt");
        fs::write(&file, "v1").unwrap();
        let first = read_text_file(&file.to_string_lossy(), &root).unwrap();

        // 外部程序修改文件
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(&file, "external").unwrap();

        let err = write_text_file(
            &file.to_string_lossy(),
            &root,
            "v2",
            Some(first.mtime_ms),
            false,
        )
        .unwrap_err();
        assert_eq!(err.code, "stale_file");

        // 强制覆盖
        let res = write_text_file(
            &file.to_string_lossy(),
            &root,
            "v2",
            Some(first.mtime_ms),
            true,
        )
        .unwrap();
        assert!(res.mtime_ms > 0);
        assert_eq!(fs::read_to_string(&file).unwrap(), "v2");
    }

    #[test]
    fn write_detects_external_delete() {
        let (_dir, root) = setup();
        let file = root.join("gone.txt");
        fs::write(&file, "v1").unwrap();
        fs::remove_file(&file).unwrap();
        let err = write_text_file(&file.to_string_lossy(), &root, "v2", None, false).unwrap_err();
        assert_eq!(err.code, "file_missing");

        write_text_file(&file.to_string_lossy(), &root, "v2", None, true).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "v2");
    }

    #[test]
    fn file_meta_reports_missing_gracefully() {
        let (_dir, root) = setup();
        let missing = root.join("nope.txt");
        let meta = file_meta(&missing.to_string_lossy(), &root).unwrap();
        assert!(!meta.exists);
        assert!(meta.mtime_ms.is_none());
    }
}
