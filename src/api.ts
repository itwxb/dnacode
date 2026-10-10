import { invoke } from "@tauri-apps/api/core";
import type {
  AiPreview,
  AiSummary,
  AiSettings,
  AiTestResult,
  BranchList,
  CommitFile,
  CommitInfo,
  CommitOutcome,
  DiffPayload,
  DirEntry,
  FileContent,
  FileMeta,
  GitStatus,
  ProjectInfo,
  PushOutcome,
  SettingsDto,
  WriteResult,
} from "./types";

/** 后端统一错误结构：{ code, message, detail } */
export interface AppErrorShape {
  code: string;
  message: string;
  detail: string | null;
}

export class AppError extends Error {
  readonly code: string;
  readonly detail: string | null;

  constructor(code: string, message: string, detail: string | null = null) {
    super(message);
    this.name = "AppError";
    this.code = code;
    this.detail = detail;
  }
}

export function normalizeError(e: unknown): AppError {
  if (e instanceof AppError) return e;
  if (typeof e === "string") return new AppError("unknown", e, null);
  if (typeof e === "object" && e !== null) {
    const obj = e as Record<string, unknown>;
    if (typeof obj.code === "string" && typeof obj.message === "string") {
      return new AppError(
        obj.code,
        obj.message,
        typeof obj.detail === "string" ? obj.detail : null,
      );
    }
    if (typeof obj.message === "string") {
      return new AppError("unknown", obj.message, null);
    }
  }
  return new AppError("unknown", "发生了未知错误", null);
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw normalizeError(e);
  }
}

export const api = {
  // 项目与文件
  openProject: (path: string) => call<ProjectInfo>("open_project", { path }),
  closeProject: () => call<void>("close_project"),
  listDirectory: (path: string) => call<DirEntry[]>("list_directory", { path }),
  readTextFile: (path: string) => call<FileContent>("read_text_file", { path }),
  writeTextFile: (path: string, content: string, expectedMtimeMs: number | null, force: boolean) =>
    call<WriteResult>("write_text_file", { path, content, expectedMtimeMs, force }),
  fileMeta: (path: string) => call<FileMeta>("file_meta", { path }),

  // Git
  gitStatus: () => call<GitStatus>("git_status"),
  gitFileDiff: (path: string, staged: boolean, untracked: boolean, full = false) =>
    call<DiffPayload>("git_file_diff", { path, staged, untracked, full }),
  gitCommitDiff: (sha: string, path: string | null, full = false) =>
    call<DiffPayload>("git_commit_diff", { sha, path, full }),
  gitStage: (paths: string[]) => call<void>("git_stage", { paths }),
  gitUnstage: (paths: string[]) => call<void>("git_unstage", { paths }),
  gitStageAll: () => call<void>("git_stage_all"),
  gitUnstageAll: () => call<void>("git_unstage_all"),
  gitDiscardUnstaged: (paths: string[]) => call<void>("git_discard_unstaged", { paths }),
  gitDeleteUntracked: (paths: string[]) => call<number>("git_delete_untracked", { paths }),
  gitCommit: (message: string) => call<CommitOutcome>("git_commit", { message }),
  gitPush: (setUpstream: boolean) => call<PushOutcome>("git_push", { setUpstream }),
  gitBranches: () => call<BranchList>("git_branches"),
  gitCheckout: (branch: string, allowDirty: boolean) =>
    call<string>("git_checkout", { branch, allowDirty }),
  gitLog: (skip: number, limit: number) => call<CommitInfo[]>("git_log", { skip, limit }),
  gitCommitFiles: (sha: string) => call<CommitFile[]>("git_commit_files", { sha }),
  gitInitRepository: () => call<void>("git_init_repository"),

  // 设置与凭据
  configGet: () => call<SettingsDto>("config_get"),
  configSave: (input: { theme: string; ai: AiSettings }) =>
    call<SettingsDto>("config_save", { input }),
  apiKeySet: (key: string) => call<SettingsDto>("api_key_set", { key }),
  apiKeyDelete: () => call<SettingsDto>("api_key_delete"),
  recentProjectRemove: (path: string) => call<SettingsDto>("recent_project_remove", { path }),
  recentProjectClear: () => call<SettingsDto>("recent_project_clear"),

  // AI
  aiPreview: (scope: "staged" | "all") => call<AiPreview>("ai_preview", { scope }),
  aiSummarize: (scope: "staged" | "all") => call<AiSummary>("ai_summarize", { scope }),
  aiCancel: () => call<boolean>("ai_cancel"),
  aiTestConnection: () => call<AiTestResult>("ai_test_connection"),
};
