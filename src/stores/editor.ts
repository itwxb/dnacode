import { reactive } from "vue";
import { api, normalizeError } from "../api";
import type { DiffPayload } from "../types";
import { fileNameOf } from "../utils/format";
import { askDialog } from "./dialog";
import { toastError, toasts } from "./toast";

// ---------------------------------------------------------------------------
// 选项卡模型
// ---------------------------------------------------------------------------

export interface FileTab {
  id: string;
  kind: "file";
  path: string;
  title: string;
  mtimeMs: number | null;
  dirty: boolean;
  /** 文件被外部程序修改或删除（在编辑器内有未保存修改时） */
  externalChanged: boolean;
  loading: boolean;
}

export type DiffRequest =
  | { type: "staged"; path: string; untracked: boolean }
  | { type: "worktree"; path: string; untracked: boolean }
  | { type: "commit"; sha: string; path: string | null };

export interface DiffTab {
  id: string;
  kind: "diff";
  title: string;
  request: DiffRequest;
  payload: DiffPayload | null;
  loading: boolean;
  showingFull: boolean;
}

export type EditorTab = FileTab | DiffTab;

export const editorState = reactive({
  tabs: [] as EditorTab[],
  activeId: null as string | null,
});

/** 每个文件标签页最近一次保存/加载时的内容（用于脏标记判断）。 */
const savedSnapshots = new Map<string, string>();

// ---------------------------------------------------------------------------
// 编辑器桥（由 EditorPane 注入，负责 CodeMirror 文档读写）
// ---------------------------------------------------------------------------

export interface EditorBridge {
  /** 设置/替换某个标签页的编辑内容 */
  attachContent(tabId: string, content: string): void;
  /** 读取当前编辑器中的文档内容 */
  getDoc(tabId: string): string | null;
  /** 保存成功后更新基线 */
  markSaved(tabId: string, content: string): void;
  disposeTab(tabId: string): void;
}

let bridge: EditorBridge | null = null;

export function setEditorBridge(b: EditorBridge | null): void {
  bridge = b;
}

export function getSavedSnapshot(tabId: string): string | undefined {
  return savedSnapshots.get(tabId);
}

export function isDocDirty(tabId: string, doc: string): boolean {
  const saved = savedSnapshots.get(tabId);
  if (saved === undefined) return false;
  if (saved === doc) return false;
  if (saved.length !== doc.length) return true;
  return saved !== doc;
}

/** 与 isDocDirty 相同，但接受 CodeMirror Text（先比长度，避免大文件每击键生成字符串）。 */
export function isDocDirtyText(
  tabId: string,
  doc: { length: number; toString(): string },
): boolean {
  const saved = savedSnapshots.get(tabId);
  if (saved === undefined) return false;
  if (saved.length !== doc.length) return true;
  return saved !== doc.toString();
}

// ---------------------------------------------------------------------------
// 查询
// ---------------------------------------------------------------------------

export function findTab(id: string): EditorTab | undefined {
  return editorState.tabs.find((t) => t.id === id);
}

export function currentTab(): EditorTab | null {
  return editorState.activeId ? (findTab(editorState.activeId) ?? null) : null;
}

export function diffTabId(req: DiffRequest): string {
  if (req.type === "commit") return `diff:commit:${req.sha}:${req.path ?? ""}`;
  return `diff:${req.type}:${req.path}`;
}

export function diffTabTitle(req: DiffRequest): string {
  if (req.type === "commit") {
    return req.path
      ? `${fileNameOf(req.path)} @ ${req.sha.slice(0, 8)}`
      : `提交 ${req.sha.slice(0, 7)}`;
  }
  return fileNameOf(req.path);
}

// ---------------------------------------------------------------------------
// 文件标签页
// ---------------------------------------------------------------------------

export async function openFileTab(path: string, title?: string): Promise<void> {
  const id = `file:${path}`;
  const existing = findTab(id);
  if (existing) {
    editorState.activeId = id;
    if (existing.kind === "file") void verifyExternalChange(existing);
    return;
  }

  const tab: FileTab = {
    id,
    kind: "file",
    path,
    title: title ?? fileNameOf(path),
    mtimeMs: null,
    dirty: false,
    externalChanged: false,
    loading: true,
  };
  editorState.tabs.push(tab);
  editorState.activeId = id;

  try {
    const fc = await api.readTextFile(path);
    tab.mtimeMs = fc.mtimeMs;
    savedSnapshots.set(id, fc.content);
    tab.loading = false;
    bridge?.attachContent(id, fc.content);
  } catch (e) {
    tab.loading = false;
    const err = normalizeError(e);
    closeTabSilently(id);
    toasts.push("error", `无法打开 ${fileNameOf(path)}：${err.message}`, err.detail, 10000);
  }
}

export function activateTab(id: string): void {
  editorState.activeId = id;
  const tab = findTab(id);
  if (!tab) return;
  if (tab.kind === "file") {
    void verifyExternalChange(tab);
  } else {
    void ensureDiffLoaded(tab);
  }
}

/** 探测外部修改：未修改的标签自动刷新；有本地修改的标签只打标记。 */
export async function verifyExternalChange(tab: FileTab): Promise<void> {
  try {
    const meta = await api.fileMeta(tab.path);
    if (!meta.exists) {
      tab.externalChanged = true;
      return;
    }
    if (meta.mtimeMs !== null && tab.mtimeMs !== null && meta.mtimeMs !== tab.mtimeMs) {
      if (tab.dirty) {
        tab.externalChanged = true;
      } else {
        await reloadTab(tab.id, true);
      }
    }
  } catch {
    // 探测失败不打扰用户
  }
}

export async function reloadTab(tabId: string, notify = false): Promise<boolean> {
  const tab = findTab(tabId);
  if (!tab || tab.kind !== "file") return false;
  try {
    const fc = await api.readTextFile(tab.path);
    tab.mtimeMs = fc.mtimeMs;
    tab.dirty = false;
    tab.externalChanged = false;
    savedSnapshots.set(tabId, fc.content);
    bridge?.attachContent(tabId, fc.content);
    if (notify) toasts.push("info", `${tab.title} 已从磁盘重新加载`);
    return true;
  } catch (e) {
    toastError(e);
    return false;
  }
}

/** 保存文件；force 用于用户在冲突提示后确认覆盖/重建。 */
export async function saveTab(tabId: string, force = false): Promise<boolean> {
  const tab = findTab(tabId);
  if (!tab || tab.kind !== "file") return false;

  const content = bridge?.getDoc(tabId) ?? null;
  if (content === null) return false;

  try {
    const res = await api.writeTextFile(tab.path, content, force ? null : tab.mtimeMs, force);
    tab.mtimeMs = res.mtimeMs;
    tab.dirty = false;
    tab.externalChanged = false;
    savedSnapshots.set(tabId, content);
    bridge?.markSaved(tabId, content);
    // 保存成功后刷新 Git 状态（文件修改会影响工作区状态）
    void refreshGitStatusQuietly();
    return true;
  } catch (e) {
    const err = normalizeError(e);
    if (err.code === "stale_file") {
      const choice = await askDialog({
        title: "文件已被外部修改",
        message: err.message,
        detail: err.detail,
        choices: [
          { value: "overwrite", label: "用编辑器内容覆盖磁盘", kind: "danger" },
          { value: "reload", label: "放弃我的修改并重新加载" },
          { value: "cancel", label: "取消", kind: "cancel" },
        ],
      });
      if (choice === "overwrite") return saveTab(tabId, true);
      if (choice === "reload") {
        await reloadTab(tabId);
        return false;
      }
      return false;
    }
    if (err.code === "file_missing") {
      const choice = await askDialog({
        title: "文件已被外部删除",
        message: err.message,
        choices: [
          { value: "recreate", label: "重新创建该文件", kind: "danger" },
          { value: "cancel", label: "取消", kind: "cancel" },
        ],
      });
      if (choice === "recreate") return saveTab(tabId, true);
      return false;
    }
    toastError(err);
    return false;
  }
}

// 避免 store 循环依赖：由 git store 在初始化时注入状态刷新函数
let refreshGitStatusQuietly: () => Promise<void> = async () => {};
export function setGitStatusRefresher(fn: () => Promise<void>): void {
  refreshGitStatusQuietly = fn;
}

export async function saveActiveTab(): Promise<boolean> {
  if (!editorState.activeId) return false;
  const tab = findTab(editorState.activeId);
  if (!tab || tab.kind !== "file") return false;
  return saveTab(tab.id);
}

/** 保存所有有修改的文件；任意一个失败或取消则返回 false。 */
export async function saveAllDirtyTabs(): Promise<boolean> {
  const dirty = editorState.tabs.filter((t): t is FileTab => t.kind === "file" && t.dirty);
  for (const tab of dirty) {
    const ok = await saveTab(tab.id);
    if (!ok) return false;
  }
  return true;
}

export async function closeTab(tabId: string): Promise<void> {
  const tab = findTab(tabId);
  if (!tab) return;
  if (tab.kind === "file" && tab.dirty) {
    const choice = await askDialog({
      title: "文件尚未保存",
      message: `“${tab.title}” 有未保存的修改，关闭前请选择处理方式。`,
      choices: [
        { value: "save", label: "保存并关闭", kind: "primary" },
        { value: "discard", label: "不保存", kind: "danger" },
        { value: "cancel", label: "取消", kind: "cancel" },
      ],
    });
    if (choice === "save") {
      const ok = await saveTab(tabId);
      if (!ok) return;
    } else if (choice !== "discard") {
      return;
    }
  }
  closeTabSilently(tabId);
}

export function closeTabSilently(tabId: string): void {
  const idx = editorState.tabs.findIndex((t) => t.id === tabId);
  if (idx < 0) return;
  editorState.tabs.splice(idx, 1);
  savedSnapshots.delete(tabId);
  bridge?.disposeTab(tabId);
  if (editorState.activeId === tabId) {
    const next = editorState.tabs[Math.min(idx, editorState.tabs.length - 1)];
    editorState.activeId = next ? next.id : null;
  }
}

export function closeAllTabs(): void {
  for (const t of [...editorState.tabs]) {
    closeTabSilently(t.id);
  }
}

// ---------------------------------------------------------------------------
// Diff 标签页
// ---------------------------------------------------------------------------

export function openDiffTab(request: DiffRequest): void {
  const id = diffTabId(request);
  const existing = findTab(id);
  if (existing && existing.kind === "diff") {
    editorState.activeId = id;
    void ensureDiffLoaded(existing, true);
    return;
  }
  const tab: DiffTab = {
    id,
    kind: "diff",
    title: diffTabTitle(request),
    request,
    payload: null,
    loading: false,
    showingFull: false,
  };
  editorState.tabs.push(tab);
  editorState.activeId = id;
  void ensureDiffLoaded(tab);
}

async function fetchDiffPayload(request: DiffRequest, full: boolean): Promise<DiffPayload> {
  if (request.type === "commit") {
    return api.gitCommitDiff(request.sha, request.path, full);
  }
  return api.gitFileDiff(request.path, request.type === "staged", request.untracked, full);
}

export async function ensureDiffLoaded(tab: DiffTab, refresh = false): Promise<void> {
  if (tab.loading) return;
  if (tab.payload && !refresh) return;
  tab.loading = true;
  try {
    tab.payload = await fetchDiffPayload(tab.request, tab.showingFull);
  } catch (e) {
    const err = normalizeError(e);
    tab.payload = {
      kind: "empty",
      text: null,
      truncated: false,
      added: 0,
      removed: 0,
      message: err.message,
      path: tab.request.type === "commit" ? (tab.request.path ?? "") : tab.request.path,
    };
  } finally {
    tab.loading = false;
  }
}

export async function loadFullDiff(tabId: string): Promise<void> {
  const tab = findTab(tabId);
  if (!tab || tab.kind !== "diff") return;
  tab.showingFull = true;
  await ensureDiffLoaded(tab, true);
}

/** Git 状态变化后刷新所有文件型 Diff 标签页。 */
export function refreshFileDiffTabs(): void {
  for (const tab of editorState.tabs) {
    if (tab.kind === "diff" && tab.request.type !== "commit") {
      void ensureDiffLoaded(tab, true);
    }
  }
}
