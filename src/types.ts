/** 与 Rust 后端 DTO 对应的类型定义（serde camelCase）。 */

export interface ProjectInfo {
  path: string;
  name: string;
}

export interface DirEntry {
  name: string;
  path: string;
  isDir: boolean;
  isSymlink: boolean;
  hidden: boolean;
}

export interface FileContent {
  path: string;
  content: string;
  mtimeMs: number;
  size: number;
}

export interface FileMeta {
  exists: boolean;
  mtimeMs: number | null;
  size: number | null;
}

export interface WriteResult {
  mtimeMs: number;
  size: number;
}

export interface GitStatusEntry {
  path: string;
  origPath: string | null;
  x: string;
  y: string;
  untracked: boolean;
  conflicted: boolean;
}

export interface GitStatus {
  isRepo: boolean;
  branch: string | null;
  upstream: string | null;
  upstreamGone: boolean;
  ahead: number;
  behind: number;
  detached: boolean;
  unborn: boolean;
  entries: GitStatusEntry[];
  stagedCount: number;
  unstagedCount: number;
  untrackedCount: number;
  conflictedCount: number;
  remotes: string[];
  identityName: string | null;
  identityEmail: string | null;
}

export type DiffKind = "text" | "binary" | "too_large" | "empty";

export interface DiffPayload {
  kind: DiffKind;
  text: string | null;
  truncated: boolean;
  added: number;
  removed: number;
  message: string | null;
  path: string;
}

export interface CommitInfo {
  sha: string;
  shortSha: string;
  summary: string;
  author: string;
  email: string;
  timeMs: number;
}

export interface CommitFile {
  status: string;
  path: string;
  origPath: string | null;
}

export interface BranchList {
  current: string | null;
  detached: boolean;
  branches: string[];
}

export interface CommitOutcome {
  sha: string;
  summary: string;
}

export interface PushOutcome {
  branch: string;
  summary: string;
}

export interface RecentProject {
  path: string;
  name: string;
  lastOpenedMs: number;
}

export interface AiSettings {
  baseUrl: string;
  model: string;
  timeoutSecs: number;
  maxDiffChars: number;
}

export interface SettingsDto {
  theme: "dark" | "light";
  ai: AiSettings;
  recentProjects: RecentProject[];
  hasApiKey: boolean;
  keyringError: string | null;
  configWarning: string | null;
}

export interface PreviewFile {
  path: string;
  status: string;
  chars: number;
  redactions: number;
}

export interface SkippedFile {
  path: string;
  reason: string;
}

export interface AiPreview {
  scope: string;
  files: PreviewFile[];
  skipped: SkippedFile[];
  totalChars: number;
  truncated: boolean;
}

export interface AiSummary {
  summary: string;
  commitMessage: string | null;
  filesIncluded: number;
  filesSkipped: number;
  totalChars: number;
  truncated: boolean;
  model: string;
  redactions: number;
  elapsedMs: number;
}

export interface AiTestResult {
  ok: boolean;
  model: string;
  endpoint: string;
  latencyMs: number;
}
