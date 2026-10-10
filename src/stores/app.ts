import { reactive } from "vue";
import { api, normalizeError } from "../api";
import type { AiSettings, ProjectInfo, RecentProject, SettingsDto } from "../types";
import { askDialog } from "./dialog";
import { editorState, closeAllTabs, saveAllDirtyTabs } from "./editor";
import { refreshAll } from "./git";
import { toastError, toasts } from "./toast";

export const appState = reactive({
  project: null as ProjectInfo | null,
  theme: "dark" as "dark" | "light",
  recentProjects: [] as RecentProject[],
  ai: {
    baseUrl: "",
    model: "",
    timeoutSecs: 60,
    maxDiffChars: 60000,
  } as AiSettings,
  hasApiKey: false,
  keyringError: null as string | null,
  settingsLoaded: false,
  settingsOpen: false,
  opening: false,
});

export function applyTheme(theme: "dark" | "light"): void {
  appState.theme = theme;
  document.documentElement.dataset.theme = theme;
}

function applySettings(s: SettingsDto): void {
  appState.recentProjects = s.recentProjects;
  appState.ai = s.ai;
  appState.hasApiKey = s.hasApiKey;
  appState.keyringError = s.keyringError;
  applyTheme(s.theme);
  if (s.configWarning) {
    toasts.push("info", "设置提示", s.configWarning, 12000);
  }
}

export async function loadSettings(): Promise<void> {
  try {
    applySettings(await api.configGet());
  } catch (e) {
    toasts.push("error", "读取设置失败", normalizeError(e).detail, 9000);
  } finally {
    appState.settingsLoaded = true;
  }
}

export async function setTheme(theme: "dark" | "light"): Promise<void> {
  applyTheme(theme);
  try {
    applySettings(await api.configSave({ theme, ai: appState.ai }));
  } catch (e) {
    toastError(e);
  }
}

export async function saveAiSettings(input: AiSettings): Promise<boolean> {
  try {
    applySettings(await api.configSave({ theme: appState.theme, ai: input }));
    return true;
  } catch (e) {
    toastError(e);
    return false;
  }
}

export async function saveApiKey(key: string): Promise<boolean> {
  try {
    applySettings(await api.apiKeySet(key));
    toasts.push("success", "API Key 已保存到系统安全凭据存储");
    return true;
  } catch (e) {
    toastError(e);
    return false;
  }
}

export async function deleteApiKey(): Promise<boolean> {
  try {
    applySettings(await api.apiKeyDelete());
    toasts.push("info", "API Key 已从系统安全凭据存储中删除");
    return true;
  } catch (e) {
    toastError(e);
    return false;
  }
}

export async function removeRecentProject(path: string): Promise<void> {
  try {
    applySettings(await api.recentProjectRemove(path));
  } catch (e) {
    toastError(e);
  }
}

export async function clearRecentProjects(): Promise<void> {
  try {
    applySettings(await api.recentProjectClear());
  } catch (e) {
    toastError(e);
  }
}

/** 打开（或切换）项目：切换前保护未保存内容。 */
export async function requestOpenProject(path: string): Promise<boolean> {
  const dirtyTabs = editorState.tabs.filter((t) => t.kind === "file" && t.dirty);
  if (dirtyTabs.length > 0) {
    const choice = await askDialog({
      title: "存在未保存的文件",
      message: `当前有 ${dirtyTabs.length} 个文件未保存，切换项目前请选择处理方式。`,
      detail: dirtyTabs.map((t) => (t.kind === "file" ? t.path : "")).join("\n"),
      choices: [
        { value: "save", label: "保存全部并切换", kind: "primary" },
        { value: "discard", label: "放弃未保存的修改", kind: "danger" },
        { value: "cancel", label: "取消", kind: "cancel" },
      ],
    });
    if (choice === "save") {
      const ok = await saveAllDirtyTabs();
      if (!ok) return false;
    } else if (choice !== "discard") {
      return false;
    }
  }

  appState.opening = true;
  try {
    // 先清空后端项目根目录，再打开新项目
    await api.closeProject();
    closeAllTabs();
    const info = await api.openProject(path);
    appState.project = info;
    await refreshAll();
    return true;
  } catch (e) {
    toastError(e);
    return false;
  } finally {
    appState.opening = false;
  }
}
