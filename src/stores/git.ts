import { reactive } from "vue";
import { api, normalizeError } from "../api";
import type { AiPreview, AiSummary, BranchList, CommitFile, CommitInfo, GitStatus } from "../types";
import { askDialog } from "./dialog";
import { refreshFileDiffTabs, setGitStatusRefresher } from "./editor";
import { toastError, toasts } from "./toast";

/** 历史每页条数（分页加载，避免一次性读取庞大历史）。 */
export const HISTORY_PAGE = 30;

export const gitState = reactive({
  status: null as GitStatus | null,
  loading: false,
  busy: false,
  branches: null as BranchList | null,
  history: [] as CommitInfo[],
  historyLoading: false,
  historyHasMore: true,
  historyOffset: 0,
  commitMessage: "",
  aiSummary: null as AiSummary | null,
  aiBusy: false,
  aiError: null as { message: string; detail: string | null } | null,
  aiScope: "staged" as "staged" | "all",
});

const commitFilesCache = new Map<string, CommitFile[]>();

async function runAction(fn: () => Promise<void>, successMessage?: string): Promise<boolean> {
  gitState.busy = true;
  try {
    await fn();
    if (successMessage) toasts.push("success", successMessage);
    return true;
  } catch (e) {
    const err = normalizeError(e);
    if (err.code !== "no_project") toastError(err);
    return false;
  } finally {
    gitState.busy = false;
  }
}

async function afterMutation(): Promise<void> {
  await refreshStatus();
  refreshFileDiffTabs();
}

// ---------------------------------------------------------------------------
// 状态刷新
// ---------------------------------------------------------------------------

export async function refreshStatus(): Promise<void> {
  gitState.loading = true;
  try {
    gitState.status = await api.gitStatus();
  } catch (e) {
    const err = normalizeError(e);
    if (err.code !== "no_project") toastError(err);
    gitState.status = null;
  } finally {
    gitState.loading = false;
  }
}

export async function refreshBranches(): Promise<void> {
  try {
    gitState.branches = await api.gitBranches();
  } catch {
    gitState.branches = null;
  }
}

export async function refreshAll(): Promise<void> {
  await Promise.all([refreshStatus(), refreshBranches()]);
  await reloadHistory();
}

// 编辑器保存文件后需要静默刷新状态（避免循环依赖，由这里注入）
setGitStatusRefresher(async () => {
  await refreshStatus();
  refreshFileDiffTabs();
});

// ---------------------------------------------------------------------------
// 暂存 / 取消暂存 / 丢弃
// ---------------------------------------------------------------------------

export async function stagePaths(paths: string[]): Promise<void> {
  const ok = await runAction(() => api.gitStage(paths));
  if (ok) await afterMutation();
}

export async function unstagePaths(paths: string[]): Promise<void> {
  const ok = await runAction(() => api.gitUnstage(paths));
  if (ok) await afterMutation();
}

export async function stageAll(): Promise<void> {
  const ok = await runAction(() => api.gitStageAll());
  if (ok) await afterMutation();
}

export async function unstageAll(): Promise<void> {
  const ok = await runAction(() => api.gitUnstageAll());
  if (ok) await afterMutation();
}

export async function discardChanges(paths: string[]): Promise<void> {
  const choice = await askDialog({
    title: "丢弃未暂存修改",
    message: `即将丢弃 ${paths.length} 个文件的未暂存修改，此操作无法通过 Git 恢复。确认继续？`,
    detail: paths.join("\n"),
    choices: [
      { value: "discard", label: "丢弃修改", kind: "danger" },
      { value: "cancel", label: "取消", kind: "cancel" },
    ],
  });
  if (choice !== "discard") return;
  const ok = await runAction(() => api.gitDiscardUnstaged(paths));
  if (ok) {
    toasts.push("success", `已丢弃 ${paths.length} 个文件的未暂存修改`);
    await afterMutation();
  }
}

export async function deleteUntrackedFiles(paths: string[]): Promise<void> {
  const choice = await askDialog({
    title: "删除未跟踪文件",
    message: `即将从磁盘删除 ${paths.length} 个未跟踪文件，文件将被永久删除（不进入回收站）。确认继续？`,
    detail: paths.join("\n"),
    choices: [
      { value: "delete", label: "删除文件", kind: "danger" },
      { value: "cancel", label: "取消", kind: "cancel" },
    ],
  });
  if (choice !== "delete") return;
  const ok = await runAction(() => api.gitDeleteUntracked(paths).then(() => undefined));
  if (ok) {
    toasts.push("success", `已删除 ${paths.length} 个未跟踪文件`);
    await afterMutation();
  }
}

// ---------------------------------------------------------------------------
// 提交与推送
// ---------------------------------------------------------------------------

export async function commit(): Promise<void> {
  const message = gitState.commitMessage;
  if (!message.trim()) return;
  gitState.busy = true;
  try {
    const outcome = await api.gitCommit(message);
    gitState.commitMessage = "";
    toasts.push("success", `已提交 ${outcome.sha}：${outcome.summary}`);
    await afterMutation();
    await reloadHistory();
  } catch (e) {
    // 失败时保留提交信息，便于用户修改后重试
    toastError(e);
  } finally {
    gitState.busy = false;
  }
}

export async function push(setUpstream: boolean): Promise<void> {
  gitState.busy = true;
  try {
    const outcome = await api.gitPush(setUpstream);
    toasts.push("success", `已推送分支 ${outcome.branch}`);
    await refreshStatus();
  } catch (e) {
    toastError(e);
  } finally {
    gitState.busy = false;
  }
}

// ---------------------------------------------------------------------------
// 分支
// ---------------------------------------------------------------------------

export async function checkoutBranch(branch: string): Promise<void> {
  if (branch === gitState.branches?.current) return;
  gitState.busy = true;
  try {
    try {
      await api.gitCheckout(branch, false);
    } catch (e) {
      const err = normalizeError(e);
      if (err.code !== "dirty_worktree") throw err;
      gitState.busy = false;
      const choice = await askDialog({
        title: "工作区有未提交修改",
        message: err.message,
        choices: [
          { value: "switch", label: "仍然尝试切换", kind: "primary" },
          { value: "cancel", label: "取消", kind: "cancel" },
        ],
      });
      if (choice !== "switch") return;
      gitState.busy = true;
      await api.gitCheckout(branch, true);
    }
    toasts.push("success", `已切换到分支 ${branch}`);
    await refreshAll();
  } catch (e) {
    toastError(e);
  } finally {
    gitState.busy = false;
  }
}

// ---------------------------------------------------------------------------
// 历史
// ---------------------------------------------------------------------------

export async function reloadHistory(): Promise<void> {
  gitState.history = [];
  gitState.historyOffset = 0;
  gitState.historyHasMore = true;
  commitFilesCache.clear();
  await loadMoreHistory();
}

export async function loadMoreHistory(): Promise<void> {
  if (gitState.historyLoading || !gitState.historyHasMore) return;
  gitState.historyLoading = true;
  try {
    const page = await api.gitLog(gitState.historyOffset, HISTORY_PAGE);
    if (page.length === 0) {
      gitState.historyHasMore = false;
    } else {
      gitState.history.push(...page);
      gitState.historyOffset += page.length;
      gitState.historyHasMore = page.length === HISTORY_PAGE;
    }
  } catch (e) {
    const err = normalizeError(e);
    if (err.code !== "no_project") toastError(err);
    gitState.historyHasMore = false;
  } finally {
    gitState.historyLoading = false;
  }
}

export async function getCommitFiles(sha: string): Promise<CommitFile[]> {
  const cached = commitFilesCache.get(sha);
  if (cached) return cached;
  const files = await api.gitCommitFiles(sha);
  commitFilesCache.set(sha, files);
  return files;
}

// ---------------------------------------------------------------------------
// 仓库初始化
// ---------------------------------------------------------------------------

export async function initRepository(): Promise<void> {
  const ok = await runAction(() => api.gitInitRepository());
  if (ok) {
    toasts.push("success", "已初始化 Git 仓库");
    await refreshAll();
  }
}

// ---------------------------------------------------------------------------
// AI 总结
// ---------------------------------------------------------------------------

export async function previewAi(scope: "staged" | "all"): Promise<AiPreview> {
  return api.aiPreview(scope);
}

export async function summarize(scope: "staged" | "all"): Promise<void> {
  gitState.aiBusy = true;
  gitState.aiError = null;
  gitState.aiScope = scope;
  try {
    const summary = await api.aiSummarize(scope);
    gitState.aiSummary = summary;
    if (summary.commitMessage && !gitState.commitMessage.trim()) {
      gitState.commitMessage = summary.commitMessage;
    }
  } catch (e) {
    const err = normalizeError(e);
    if (err.code === "ai_cancelled") {
      toasts.push("info", "AI 请求已取消");
    } else {
      gitState.aiError = { message: err.message, detail: err.detail };
    }
  } finally {
    gitState.aiBusy = false;
  }
}

export async function cancelAi(): Promise<void> {
  try {
    await api.aiCancel();
  } catch {
    // 取消失败无需打扰用户
  }
}

export function dismissAiResult(): void {
  gitState.aiSummary = null;
  gitState.aiError = null;
}

export function applyAiCommitMessage(): void {
  const m = gitState.aiSummary?.commitMessage;
  if (m) gitState.commitMessage = m;
}
