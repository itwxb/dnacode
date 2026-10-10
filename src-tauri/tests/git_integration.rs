//! 真实 Git 仓库集成测试：在临时目录创建临时仓库，验证状态解析、暂存语义、
//! 提交、推送、分支、历史等功能。绝不触碰用户真实项目。

use dnacode_lib::git;
use std::path::{Path, PathBuf};
use std::process::Command;

fn git_ok(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("运行集成测试需要系统已安装 git");
    assert!(
        out.status.success(),
        "git {:?} 执行失败：\nstdout: {}\nstderr: {}",
        args,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

struct TestRepo {
    _dir: tempfile::TempDir,
    path: PathBuf,
}

impl TestRepo {
    fn init() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dunce::canonicalize(dir.path()).unwrap();
        git_ok(&path, &["init", "-b", "main"]);
        git_ok(&path, &["config", "user.name", "DNACode Test"]);
        git_ok(&path, &["config", "user.email", "dnacode@example.com"]);
        // 关闭行尾自动转换，保证测试在不同机器上结果一致
        git_ok(&path, &["config", "core.autocrlf", "false"]);
        Self { _dir: dir, path }
    }

    fn write(&self, rel: &str, content: &str) {
        let full = self.path.join(rel);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(full, content).unwrap();
    }
}

#[test]
fn repo_path_validation_rejects_traversal_and_absolute_paths() {
    assert!(git::validate_repo_path("src/main.rs").is_ok());
    assert_eq!(
        git::validate_repo_path("../etc/passwd").unwrap_err().code,
        "invalid_input"
    );
    assert_eq!(
        git::validate_repo_path("a/../../b").unwrap_err().code,
        "invalid_input"
    );
    assert_eq!(
        git::validate_repo_path("   ").unwrap_err().code,
        "invalid_input"
    );
    #[cfg(windows)]
    assert_eq!(
        git::validate_repo_path("C:\\Windows\\notepad.exe")
            .unwrap_err()
            .code,
        "invalid_input"
    );
    #[cfg(unix)]
    assert_eq!(
        git::validate_repo_path("/etc/passwd").unwrap_err().code,
        "invalid_input"
    );
}

#[tokio::test]
async fn status_reports_staged_unstaged_untracked() {
    let repo = TestRepo::init();
    repo.write("a.txt", "one\n");
    git_ok(&repo.path, &["add", "."]);
    git_ok(&repo.path, &["commit", "-m", "init"]);

    repo.write("a.txt", "one changed\n"); // 未暂存修改
    repo.write("b.txt", "staged\n");
    git_ok(&repo.path, &["add", "b.txt"]); // 已暂存新增
    repo.write("c.txt", "untracked\n"); // 未跟踪

    let st = git::status(&repo.path).await.unwrap();
    assert!(st.is_repo);
    assert_eq!(st.branch.as_deref(), Some("main"));
    assert!(!st.unborn);
    assert_eq!(st.staged_count, 1);
    assert_eq!(st.unstaged_count, 1);
    assert_eq!(st.untracked_count, 1);

    let a = st.entries.iter().find(|e| e.path == "a.txt").unwrap();
    assert_eq!((a.x.as_str(), a.y.as_str()), (" ", "M"));
    let b = st.entries.iter().find(|e| e.path == "b.txt").unwrap();
    assert_eq!((b.x.as_str(), b.y.as_str()), ("A", " "));
    let c = st.entries.iter().find(|e| e.path == "c.txt").unwrap();
    assert!(c.untracked);
}

#[tokio::test]
async fn non_repository_is_reported_as_flag_not_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dunce::canonicalize(dir.path()).unwrap();
    let st = git::status(&path).await.unwrap();
    assert!(!st.is_repo);
    assert!(st.branch.is_none());
    assert!(st.entries.is_empty());
}

#[tokio::test]
async fn handles_chinese_and_space_paths() {
    let repo = TestRepo::init();
    repo.write("中文 目录/示例 文件.txt", "你好\n");
    git_ok(&repo.path, &["add", "."]);

    let st = git::status(&repo.path).await.unwrap();
    assert_eq!(st.staged_count, 1);
    assert_eq!(st.entries[0].path, "中文 目录/示例 文件.txt");

    let diff = git::diff::file_diff(
        &repo.path,
        "中文 目录/示例 文件.txt",
        "staged",
        false,
        false,
    )
    .await
    .unwrap();
    assert_eq!(diff.kind, "text");
    assert!(diff.text.unwrap().contains("+\u{4f60}\u{597d}"));
}

#[tokio::test]
async fn commit_only_includes_staged_content() {
    let repo = TestRepo::init();
    repo.write("a.txt", "one\n");
    git_ok(&repo.path, &["add", "."]);
    git_ok(&repo.path, &["commit", "-m", "init"]);

    repo.write("a.txt", "changed\n");
    repo.write("b.txt", "never staged\n");
    git_ok(&repo.path, &["add", "a.txt"]);

    let outcome = git::commit::commit(&repo.path, "只提交已暂存内容")
        .await
        .unwrap();
    assert!(!outcome.sha.is_empty());
    assert_eq!(outcome.summary, "只提交已暂存内容");

    // 该次提交只包含 a.txt
    let files = git_ok(&repo.path, &["show", "--name-only", "--format=", "HEAD"]);
    assert!(files.contains("a.txt"));
    assert!(!files.contains("b.txt"));

    let st = git::status(&repo.path).await.unwrap();
    assert_eq!(st.staged_count, 0);
    assert_eq!(st.untracked_count, 1); // b.txt 仍未跟踪
    assert!(st.entries.iter().all(|e| e.path != "a.txt")); // a.txt 已干净
}

#[tokio::test]
async fn commit_rejects_blank_message_and_missing_staged_changes() {
    let repo = TestRepo::init();
    repo.write("a.txt", "one\n");

    let err = git::commit::commit(&repo.path, "   ").await.unwrap_err();
    assert_eq!(err.code, "invalid_input");

    let err = git::commit::commit(&repo.path, "首个提交")
        .await
        .unwrap_err();
    assert_eq!(err.code, "no_staged_changes");
    // 确认没有产生任何提交
    let st = git::status(&repo.path).await.unwrap();
    assert!(st.unborn);
    assert_eq!(st.untracked_count, 1);
}

#[tokio::test]
async fn commit_requires_git_identity() {
    let repo = TestRepo::init();
    repo.write("a.txt", "one\n");
    git_ok(&repo.path, &["add", "."]);
    git_ok(&repo.path, &["config", "user.name", ""]);
    git_ok(&repo.path, &["config", "user.email", ""]);

    let err = git::commit::commit(&repo.path, "无法提交")
        .await
        .unwrap_err();
    assert_eq!(err.code, "no_identity");
}

#[tokio::test]
async fn stage_and_unstage_single_file() {
    let repo = TestRepo::init();
    repo.write("a.txt", "one\n");
    git_ok(&repo.path, &["add", "."]);
    git_ok(&repo.path, &["commit", "-m", "init"]);

    repo.write("a.txt", "two\n");
    git::stage::stage_paths(&repo.path, &["a.txt".to_string()])
        .await
        .unwrap();
    let st = git::status(&repo.path).await.unwrap();
    assert_eq!(st.staged_count, 1);
    assert_eq!(st.unstaged_count, 0);

    git::stage::unstage_paths(&repo.path, &["a.txt".to_string()])
        .await
        .unwrap();
    let st = git::status(&repo.path).await.unwrap();
    assert_eq!(st.staged_count, 0);
    assert_eq!(st.unstaged_count, 1);
}

#[tokio::test]
async fn unstage_all_works_before_first_commit() {
    let repo = TestRepo::init();
    repo.write("f.txt", "content\n");
    git::stage::stage_all(&repo.path).await.unwrap();

    let st = git::status(&repo.path).await.unwrap();
    assert!(st.unborn);
    assert_eq!(st.staged_count, 1);

    git::stage::unstage_all(&repo.path).await.unwrap();
    let st = git::status(&repo.path).await.unwrap();
    assert_eq!(st.staged_count, 0);
    assert_eq!(st.untracked_count, 1);
}

#[tokio::test]
async fn rename_status_exposes_original_path() {
    let repo = TestRepo::init();
    repo.write("old_name.txt", "content\n");
    git_ok(&repo.path, &["add", "."]);
    git_ok(&repo.path, &["commit", "-m", "init"]);
    git_ok(&repo.path, &["mv", "old_name.txt", "new_name.txt"]);

    let st = git::status(&repo.path).await.unwrap();
    let e = st
        .entries
        .iter()
        .find(|e| e.path == "new_name.txt")
        .expect("状态里应包含重命名后的新路径");
    assert_eq!(e.orig_path.as_deref(), Some("old_name.txt"));
    assert_eq!(st.staged_count, 1);
}

#[tokio::test]
async fn history_log_files_and_diff_come_from_real_repo() {
    let repo = TestRepo::init();
    repo.write("a.txt", "one\n");
    git_ok(&repo.path, &["add", "."]);
    git_ok(&repo.path, &["commit", "-m", "第一个提交"]);
    repo.write("a.txt", "two\n");
    git_ok(&repo.path, &["add", "."]);
    git_ok(&repo.path, &["commit", "-m", "第二个提交"]);

    let commits = git::history::log(&repo.path, 0, 10).await.unwrap();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].summary, "第二个提交");
    assert_eq!(commits[1].summary, "第一个提交");
    assert!(commits[0].time_ms > 0);
    assert_eq!(commits[0].author, "DNACode Test");
    assert_eq!(commits[0].short_sha.len(), 8);

    let files = git::history::commit_files(&repo.path, &commits[0].sha)
        .await
        .unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].status, "M");
    assert_eq!(files[0].path, "a.txt");

    let diff = git::diff::commit_diff(&repo.path, &commits[0].sha, None, false)
        .await
        .unwrap();
    assert_eq!(diff.kind, "text");
    let text = diff.text.unwrap();
    assert!(text.contains("-one"));
    assert!(text.contains("+two"));

    // 根提交同样可读取
    let root_diff = git::diff::commit_diff(&repo.path, &commits[1].sha, None, false)
        .await
        .unwrap();
    assert_eq!(root_diff.kind, "text");
    assert!(root_diff.text.unwrap().contains("+one"));

    // 分页：跳过第一个取一个
    let page = git::history::log(&repo.path, 1, 1).await.unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].summary, "第一个提交");
}

#[tokio::test]
async fn commit_files_reports_renames_with_original_path() {
    let repo = TestRepo::init();
    repo.write("old.txt", "content\n");
    git_ok(&repo.path, &["add", "."]);
    git_ok(&repo.path, &["commit", "-m", "init"]);
    git_ok(&repo.path, &["mv", "old.txt", "new.txt"]);
    git_ok(&repo.path, &["commit", "-m", "rename"]);

    let commits = git::history::log(&repo.path, 0, 1).await.unwrap();
    let files = git::history::commit_files(&repo.path, &commits[0].sha)
        .await
        .unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].status, "R");
    assert_eq!(files[0].path, "new.txt");
    assert_eq!(files[0].orig_path.as_deref(), Some("old.txt"));
}

#[tokio::test]
async fn worktree_and_staged_diffs_are_distinct() {
    let repo = TestRepo::init();
    repo.write("a.txt", "one\n");
    git_ok(&repo.path, &["add", "."]);
    git_ok(&repo.path, &["commit", "-m", "init"]);

    repo.write("a.txt", "two\n");
    let unstaged = git::diff::file_diff(&repo.path, "a.txt", "worktree", false, false)
        .await
        .unwrap();
    assert_eq!(unstaged.kind, "text");
    let text = unstaged.text.unwrap();
    assert!(text.contains("-one"));
    assert!(text.contains("+two"));
    assert_eq!((unstaged.added, unstaged.removed), (1, 1));

    git_ok(&repo.path, &["add", "a.txt"]);
    let staged = git::diff::file_diff(&repo.path, "a.txt", "staged", false, false)
        .await
        .unwrap();
    assert!(staged.text.unwrap().contains("+two"));

    let worktree_after = git::diff::file_diff(&repo.path, "a.txt", "worktree", false, false)
        .await
        .unwrap();
    assert_eq!(worktree_after.kind, "empty");
}

#[tokio::test]
async fn untracked_file_diff_is_synthesized_from_real_content() {
    let repo = TestRepo::init();
    repo.write("new.txt", "line one\nline two\n");

    let diff = git::diff::file_diff(&repo.path, "new.txt", "worktree", true, false)
        .await
        .unwrap();
    assert_eq!(diff.kind, "text");
    assert_eq!(diff.added, 2);
    let text = diff.text.unwrap();
    assert!(text.contains("new file mode"));
    assert!(text.contains("+line one"));
    assert!(text.contains("+line two"));

    // 空文件
    repo.write("empty.txt", "");
    let empty = git::diff::file_diff(&repo.path, "empty.txt", "worktree", true, false)
        .await
        .unwrap();
    assert_eq!(empty.kind, "empty");

    // 二进制文件
    std::fs::write(repo.path.join("bin.dat"), [0u8, 1, 2, 3, 4]).unwrap();
    let bin = git::diff::file_diff(&repo.path, "bin.dat", "worktree", true, false)
        .await
        .unwrap();
    assert_eq!(bin.kind, "binary");
}

#[tokio::test]
async fn deleted_file_shows_removal_diff() {
    let repo = TestRepo::init();
    repo.write("gone.txt", "to be removed\n");
    git_ok(&repo.path, &["add", "."]);
    git_ok(&repo.path, &["commit", "-m", "init"]);
    std::fs::remove_file(repo.path.join("gone.txt")).unwrap();

    let st = git::status(&repo.path).await.unwrap();
    let e = st.entries.iter().find(|e| e.path == "gone.txt").unwrap();
    assert_eq!(e.y, "D");
    assert_eq!(st.unstaged_count, 1);

    let diff = git::diff::file_diff(&repo.path, "gone.txt", "worktree", false, false)
        .await
        .unwrap();
    assert_eq!(diff.kind, "text");
    assert!(diff.text.unwrap().contains("-to be removed"));
}

#[tokio::test]
async fn discard_restores_file_and_delete_untracked_is_guarded() {
    let repo = TestRepo::init();
    repo.write("a.txt", "original\n");
    git_ok(&repo.path, &["add", "."]);
    git_ok(&repo.path, &["commit", "-m", "init"]);

    repo.write("a.txt", "modified\n");
    git::stage::discard_unstaged(&repo.path, &["a.txt".to_string()])
        .await
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).unwrap(),
        "original\n"
    );

    repo.write("u.txt", "untracked\n");
    // 已跟踪文件受保护，不允许通过“删除未跟踪文件”删除
    let err = git::stage::delete_untracked(&repo.path, &["a.txt".to_string()])
        .await
        .unwrap_err();
    assert_eq!(err.code, "invalid_input");

    let n = git::stage::delete_untracked(&repo.path, &["u.txt".to_string()])
        .await
        .unwrap();
    assert_eq!(n, 1);
    assert!(!repo.path.join("u.txt").exists());
}

#[tokio::test]
async fn push_requires_remote_and_supports_set_upstream() {
    let repo = TestRepo::init();
    repo.write("a.txt", "one\n");
    git_ok(&repo.path, &["add", "."]);
    git_ok(&repo.path, &["commit", "-m", "init"]);

    // 没有远程仓库
    let err = git::push::push(&repo.path, false).await.unwrap_err();
    assert_eq!(err.code, "no_remote");

    // 添加本地裸仓库作为远程
    let bare_dir = tempfile::tempdir().unwrap();
    let bare_path = dunce::canonicalize(bare_dir.path()).unwrap();
    git_ok(&bare_path, &["init", "--bare", "-b", "main"]);
    let remote_url = bare_path.to_string_lossy().replace('\\', "/");
    git_ok(&repo.path, &["remote", "add", "origin", &remote_url]);

    // 未建立上游时，普通 push 返回明确的引导错误
    let err = git::push::push(&repo.path, false).await.unwrap_err();
    assert_eq!(err.code, "no_upstream");

    // 发布分支并推送
    let outcome = git::push::push(&repo.path, true).await.unwrap();
    assert_eq!(outcome.branch, "main");
    assert!(!outcome.summary.is_empty());

    let st = git::status(&repo.path).await.unwrap();
    assert_eq!(st.upstream.as_deref(), Some("origin/main"));
    assert_eq!(st.ahead, 0);

    // 再提交一次，ahead 应为 1；推送后归零
    repo.write("b.txt", "two\n");
    git_ok(&repo.path, &["add", "."]);
    git_ok(&repo.path, &["commit", "-m", "second"]);
    let st = git::status(&repo.path).await.unwrap();
    assert_eq!(st.ahead, 1);
    git::push::push(&repo.path, false).await.unwrap();
    let st = git::status(&repo.path).await.unwrap();
    assert_eq!(st.ahead, 0);
}

#[tokio::test]
async fn branch_checkout_requires_confirmation_when_dirty() {
    let repo = TestRepo::init();
    repo.write("a.txt", "one\n");
    git_ok(&repo.path, &["add", "."]);
    git_ok(&repo.path, &["commit", "-m", "init"]);
    git_ok(&repo.path, &["branch", "feature"]);

    repo.write("a.txt", "dirty\n");
    let err = git::branch::checkout(&repo.path, "feature", false)
        .await
        .unwrap_err();
    assert_eq!(err.code, "dirty_worktree");

    let name = git::branch::checkout(&repo.path, "feature", true)
        .await
        .unwrap();
    assert_eq!(name, "feature");

    let list = git::branch::list(&repo.path).await.unwrap();
    assert_eq!(list.current.as_deref(), Some("feature"));
    assert!(list.branches.contains(&"main".to_string()));

    let err = git::branch::checkout(&repo.path, "-evil", true)
        .await
        .unwrap_err();
    assert_eq!(err.code, "invalid_input");

    let err = git::branch::checkout(&repo.path, "no-such-branch", true)
        .await
        .unwrap_err();
    assert_eq!(err.code, "invalid_input");
}

#[tokio::test]
async fn init_repository_creates_usable_repo() {
    let dir = tempfile::tempdir().unwrap();
    let path = dunce::canonicalize(dir.path()).unwrap();

    let st = git::status(&path).await.unwrap();
    assert!(!st.is_repo);

    git::init_repository(&path).await.unwrap();
    let st = git::status(&path).await.unwrap();
    assert!(st.is_repo);
}
