//! `git status --porcelain=v1 -z` 的解析。
//!
//! 严格按机器可解析格式处理，不通过文件名推测状态：
//! - 每个条目形如 `XY PATH`，字段之间以 NUL 分隔；
//! - 重命名/复制条目（X 或 Y 为 `R`/`C`）后跟一个额外字段（原路径）；
//! - 首行可能为分支头 `## branch...upstream [ahead N, behind M]`。

use serde::Serialize;

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatusEntry {
    /// 仓库相对路径（工作区路径）
    pub path: String,
    /// 重命名/复制时的原路径
    pub orig_path: Option<String>,
    /// 索引（暂存区）状态字符，来自 porcelain 的 X
    pub x: String,
    /// 工作区状态字符，来自 porcelain 的 Y
    pub y: String,
    /// `??`：未跟踪
    pub untracked: bool,
    /// 未合并冲突（UU / AA / DD / AU / UA / DU / UD）
    pub conflicted: bool,
}

impl StatusEntry {
    /// 该条目是否包含已暂存内容
    pub fn staged(&self) -> bool {
        !self.untracked && !self.conflicted && self.x != " "
    }

    /// 该条目是否包含未暂存内容
    pub fn unstaged(&self) -> bool {
        !self.untracked && !self.conflicted && self.y != " "
    }
}

#[derive(Serialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub upstream: Option<String>,
    /// 上游分支已被删除（`[gone]`）
    pub upstream_gone: bool,
    pub ahead: u32,
    pub behind: u32,
    pub detached: bool,
    /// 仓库尚无任何提交
    pub unborn: bool,
    pub entries: Vec<StatusEntry>,
    pub staged_count: usize,
    pub unstaged_count: usize,
    pub untracked_count: usize,
    pub conflicted_count: usize,
    /// 远程仓库名列表
    pub remotes: Vec<String>,
    pub identity_name: Option<String>,
    pub identity_email: Option<String>,
}

impl GitStatus {
    pub fn not_a_repo() -> Self {
        Self::default()
    }
}

fn is_conflict(x: char, y: char) -> bool {
    matches!(
        (x, y),
        ('D', 'D') | ('A', 'U') | ('U', 'D') | ('U', 'A') | ('D', 'U') | ('A', 'A') | ('U', 'U')
    )
}

fn parse_count(inner: &str, label: &str) -> Option<u32> {
    inner.find(label).map(|i| {
        let rest = &inner[i + label.len()..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse::<u32>().unwrap_or(0)
    })
}

fn parse_ahead_behind(inner: &str) -> (u32, u32) {
    let lower = inner.to_lowercase();
    if let Some(ahead) = parse_count(&lower, "ahead ") {
        let behind = parse_count(&lower, "behind ").unwrap_or(0);
        return (ahead, behind);
    }
    if let Some(behind) = parse_count(&lower, "behind ") {
        return (0, behind);
    }
    // 兜底：忽略措辞（本地化场景），按 git 输出顺序取数字
    let numbers: Vec<u32> = inner
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();
    match numbers.as_slice() {
        [a] => (*a, 0),
        [a, b, ..] => (*a, *b),
        [] => (0, 0),
    }
}

fn parse_branch_header(raw: &[u8], st: &mut GitStatus) {
    let text = String::from_utf8_lossy(raw);
    let mut main = text.trim();

    // 上游被删除：[gone]
    if let Some(stripped) = main.strip_suffix("[gone]").map(str::trim_end) {
        st.upstream_gone = true;
        main = stripped;
    }

    // 跟踪信息：[ahead N, behind M]
    if let Some(idx) = main.rfind(" [") {
        if main.ends_with(']') {
            let inner = &main[idx + 2..main.len() - 1];
            let (ahead, behind) = parse_ahead_behind(inner);
            st.ahead = ahead;
            st.behind = behind;
            main = main[..idx].trim_end();
        }
    }

    if let Some(rest) = main.strip_prefix("No commits yet on ") {
        st.branch = Some(rest.trim().to_string());
        st.unborn = true;
    } else if let Some(rest) = main.strip_prefix("Initial commit on ") {
        st.branch = Some(rest.trim().to_string());
        st.unborn = true;
    } else if main == "HEAD (no branch)" {
        st.detached = true;
    } else if let Some((branch, upstream)) = main.split_once("...") {
        st.branch = Some(branch.trim().to_string());
        st.upstream = Some(upstream.trim().to_string());
    } else if !main.is_empty() {
        st.branch = Some(main.trim().to_string());
    }
}

/// 解析 porcelain v1 -z 输出。
pub fn parse_porcelain_v1_z(bytes: &[u8]) -> GitStatus {
    let mut st = GitStatus {
        is_repo: true,
        ..GitStatus::default()
    };

    let fields: Vec<&[u8]> = bytes.split(|b| *b == 0).collect();
    let mut idx = 0;

    if let Some(first) = fields.first() {
        if let Some(header) = first.strip_prefix(b"## ") {
            parse_branch_header(header, &mut st);
            idx = 1;
        }
    }

    while idx < fields.len() {
        let field = fields[idx];
        if field.len() < 4 {
            idx += 1;
            continue;
        }
        let x = field[0] as char;
        let y = field[1] as char;
        // 规范格式为 "XY PATH"；容忍缺失分隔空格的输出
        let path_slice = if field[2] == b' ' {
            &field[3..]
        } else {
            &field[2..]
        };
        let path = String::from_utf8_lossy(path_slice).into_owned();
        let untracked = x == '?' && y == '?';
        let conflicted = is_conflict(x, y);

        // 重命名 / 复制：后一个 NUL 字段是原路径
        let mut orig_path = None;
        if matches!(x, 'R' | 'C') || matches!(y, 'R' | 'C') {
            if let Some(next) = fields.get(idx + 1) {
                if !next.is_empty() {
                    orig_path = Some(String::from_utf8_lossy(next).into_owned());
                    idx += 1;
                }
            }
        }

        let entry = StatusEntry {
            path,
            orig_path,
            x: x.to_string(),
            y: y.to_string(),
            untracked,
            conflicted,
        };
        if entry.untracked {
            st.untracked_count += 1;
        } else if entry.conflicted {
            st.conflicted_count += 1;
        } else {
            if entry.staged() {
                st.staged_count += 1;
            }
            if entry.unstaged() {
                st.unstaged_count += 1;
            }
        }
        st.entries.push(entry);
        idx += 1;
    }

    st
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_header_with_ahead_behind() {
        let raw = b"## main...origin/main [ahead 2, behind 1]\0";
        let st = parse_porcelain_v1_z(raw);
        assert_eq!(st.branch.as_deref(), Some("main"));
        assert_eq!(st.upstream.as_deref(), Some("origin/main"));
        assert_eq!(st.ahead, 2);
        assert_eq!(st.behind, 1);
        assert!(!st.unborn);
    }

    #[test]
    fn parses_unborn_repository() {
        let st = parse_porcelain_v1_z(b"## No commits yet on main\0");
        assert_eq!(st.branch.as_deref(), Some("main"));
        assert!(st.unborn);
        assert!(st.entries.is_empty());
    }

    #[test]
    fn parses_detached_head() {
        let st = parse_porcelain_v1_z(b"## HEAD (no branch)\0");
        assert!(st.detached);
        assert!(st.branch.is_none());
    }

    #[test]
    fn parses_gone_upstream() {
        let st = parse_porcelain_v1_z(b"## main...origin/main [gone]\0");
        assert!(st.upstream_gone);
        assert_eq!(st.branch.as_deref(), Some("main"));
    }

    #[test]
    fn parses_staged_unstaged_untracked() {
        // 注意：逐行拼接，保留 " M" 条目的前导空格
        let raw: Vec<u8> = [
            "## main...origin/main [ahead 1]\0",
            "M  staged.txt\0",
            " M unstaged.txt\0",
            "MM both.txt\0",
            "?? untracked file.txt\0",
        ]
        .concat()
        .into_bytes();
        let st = parse_porcelain_v1_z(&raw);
        assert_eq!(st.entries.len(), 4);
        assert_eq!(st.staged_count, 2);
        assert_eq!(st.unstaged_count, 2);
        assert_eq!(st.untracked_count, 1);

        let both = st.entries.iter().find(|e| e.path == "both.txt").unwrap();
        assert!(both.staged() && both.unstaged());

        let unstaged = st
            .entries
            .iter()
            .find(|e| e.path == "unstaged.txt")
            .unwrap();
        assert_eq!((unstaged.x.as_str(), unstaged.y.as_str()), (" ", "M"));

        let untracked = st.entries.iter().find(|e| e.untracked).unwrap();
        assert_eq!(untracked.path, "untracked file.txt");
        assert_eq!(untracked.x, "?");
        assert_eq!(untracked.y, "?");
    }

    #[test]
    fn parses_chinese_and_space_paths() {
        let raw = "?? 中文 目录/示例文件.txt\0".as_bytes();
        let st = parse_porcelain_v1_z(raw);
        assert_eq!(st.entries[0].path, "中文 目录/示例文件.txt");
    }

    #[test]
    fn parses_rename_with_following_orig_field() {
        // 重命名：第一个字段是工作区（新）路径，后一个字段是原路径
        let raw = b"R  new_name.txt\0old_name.txt\0";
        let st = parse_porcelain_v1_z(raw);
        assert_eq!(st.entries.len(), 1);
        let e = &st.entries[0];
        assert_eq!(e.path, "new_name.txt");
        assert_eq!(e.orig_path.as_deref(), Some("old_name.txt"));
        assert_eq!(st.staged_count, 1);
    }

    #[test]
    fn parses_conflicts() {
        let raw = b"UU conflict.txt\0DD deleted-both.txt\0";
        let st = parse_porcelain_v1_z(raw);
        assert_eq!(st.conflicted_count, 2);
        assert_eq!(st.staged_count, 0);
        assert_eq!(st.unstaged_count, 0);
    }

    #[test]
    fn parses_deleted_files() {
        let raw = b" D gone-unstaged.txt\0D  gone-staged.txt\0";
        let st = parse_porcelain_v1_z(raw);
        assert_eq!(st.staged_count, 1);
        assert_eq!(st.unstaged_count, 1);
    }

    #[test]
    fn localized_ahead_behind_fallback() {
        let st = parse_porcelain_v1_z("## main...origin/main [领先 3，落后 4]\0".as_bytes());
        assert_eq!(st.ahead, 3);
        assert_eq!(st.behind, 4);
    }
}
