<script setup lang="ts">
import { computed, ref } from "vue";
import type { GitStatusEntry } from "../types";
import { appState } from "../stores/app";
import { askDialog } from "../stores/dialog";
import { openDiffTab } from "../stores/editor";
import {
  applyAiCommitMessage,
  cancelAi,
  checkoutBranch,
  commit,
  deleteUntrackedFiles,
  discardChanges,
  dismissAiResult,
  gitState,
  initRepository,
  push,
  refreshAll,
  stageAll,
  stagePaths,
  summarize,
  unstageAll,
  unstagePaths,
} from "../stores/git";
import { dirNameOf, fileNameOf, formatChars, statusLetter, statusTone } from "../utils/format";
import AiPreviewDialog from "./AiPreviewDialog.vue";
import HistoryList from "./HistoryList.vue";

const status = computed(() => gitState.status);

const stagedEntries = computed(
  () => status.value?.entries.filter((e) => !e.untracked && !e.conflicted && e.x !== " ") ?? [],
);
const unstagedEntries = computed(
  () => status.value?.entries.filter((e) => !e.untracked && !e.conflicted && e.y !== " ") ?? [],
);
const untrackedEntries = computed(() => status.value?.entries.filter((e) => e.untracked) ?? []);
const conflictedEntries = computed(() => status.value?.entries.filter((e) => e.conflicted) ?? []);
const hasAnyChange = computed(
  () =>
    stagedEntries.value.length +
      unstagedEntries.value.length +
      untrackedEntries.value.length +
      conflictedEntries.value.length >
    0,
);

const branchMenuOpen = ref(false);
const aiDialogOpen = ref(false);

const commitDisabled = computed(
  () => gitState.busy || !gitState.commitMessage.trim() || (status.value?.stagedCount ?? 0) === 0,
);

const identityMissing = computed(
  () => !!status.value?.isRepo && (!status.value.identityName || !status.value.identityEmail),
);

const canPush = computed(() => {
  const s = status.value;
  if (!s || !s.isRepo || s.unborn || s.detached) return false;
  return s.remotes.length > 0;
});

const pushLabel = computed(() => (status.value?.upstream ? "推送" : "发布分支并推送"));

const pushHint = computed(() => {
  const s = status.value;
  if (!s) return "";
  if (s.remotes.length === 0) return "没有远程仓库";
  if (s.detached) return "处于 detached HEAD，无法推送";
  if (s.unborn) return "尚无提交，无法推送";
  if (!s.upstream) return "本地分支没有上游分支";
  const ab = [s.ahead ? `领先 ${s.ahead}` : "", s.behind ? `落后 ${s.behind}` : ""]
    .filter(Boolean)
    .join("，");
  return ab ? `${s.upstream}（${ab}）` : s.upstream;
});

const aiScopeLabel = computed(() =>
  gitState.aiScope === "staged" ? "仅已暂存内容" : "全部工作区变更",
);

function entryKey(entry: GitStatusEntry, side: string): string {
  return `${side}:${entry.path}`;
}

function openEntryDiff(entry: GitStatusEntry, side: "staged" | "unstaged"): void {
  if (side === "staged") {
    openDiffTab({ type: "staged", path: entry.path, untracked: false });
  } else {
    openDiffTab({ type: "worktree", path: entry.path, untracked: entry.untracked });
  }
}

async function onCommit(): Promise<void> {
  await commit();
}

function startAi(): void {
  if (!appState.ai.baseUrl || !appState.ai.model || !appState.hasApiKey) {
    void guideAiSetup();
    return;
  }
  aiDialogOpen.value = true;
}

async function guideAiSetup(): Promise<void> {
  const missing: string[] = [];
  if (!appState.ai.baseUrl) missing.push("API Base URL");
  if (!appState.ai.model) missing.push("模型名称");
  if (!appState.hasApiKey) missing.push("API Key");
  const choice = await askDialog({
    title: "尚未配置 AI（可选功能）",
    message: `缺少：${missing.join("、")}。\n\n未配置 AI 不影响打开项目、编辑代码和全部 Git 操作。配置后可在提交前生成修改总结与建议的提交信息。`,
    choices: [
      { value: "settings", label: "打开设置", kind: "primary" },
      { value: "cancel", label: "取消", kind: "cancel" },
    ],
  });
  if (choice === "settings") {
    appState.settingsOpen = true;
  }
}

function confirmAi(scope: "staged" | "all"): void {
  aiDialogOpen.value = false;
  void summarize(scope);
}

async function onPickBranch(branch: string): Promise<void> {
  branchMenuOpen.value = false;
  await checkoutBranch(branch);
}
</script>

<template>
  <div class="git-panel">
    <div class="panel-header">
      <span>Git</span>
      <button
        v-if="status?.isRepo && (status.branch || status.detached)"
        class="branch-chip"
        :title="'切换分支'"
        @click="branchMenuOpen = !branchMenuOpen"
      >
        <span>{{ status.detached ? "detached HEAD" : status.branch }}</span>
      </button>
      <div class="spacer"></div>
      <span v-if="gitState.loading" class="spinner"></span>
      <button
        class="btn icon small subtle"
        title="刷新 Git 状态"
        :disabled="gitState.loading"
        @click="refreshAll()"
      >
        <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path
            d="M13.5 8a5.5 5.5 0 1 1-1.7-4M13.6 2.2v3.2h-3.2"
            stroke="currentColor"
            stroke-width="1.4"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
    </div>

    <div v-if="branchMenuOpen" class="branch-menu-backdrop" @click="branchMenuOpen = false"></div>
    <div v-if="branchMenuOpen" class="branch-menu">
      <div
        v-for="b in gitState.branches?.branches ?? []"
        :key="b"
        class="branch-menu-item"
        :class="{ current: b === gitState.branches?.current }"
        @click="onPickBranch(b)"
      >
        {{ b }}
        <span v-if="b === gitState.branches?.current" class="hint">当前</span>
      </div>
      <div v-if="!(gitState.branches?.branches ?? []).length" class="branch-menu-item muted">
        没有本地分支
      </div>
    </div>

    <div class="panel-body">
      <template v-if="!status">
        <div class="git-empty">
          <span class="spinner"></span>
          正在读取 Git 状态…
        </div>
      </template>

      <template v-else-if="!status.isRepo">
        <div class="git-notice">
          <div class="title">当前文件夹不是 Git 仓库</div>
          <div>代码查看与编辑不受影响。初始化 Git 仓库后即可使用变更审查、提交与推送。</div>
          <div style="margin-top: 10px">
            <button class="btn small" :disabled="gitState.busy" @click="initRepository">
              初始化 Git 仓库
            </button>
          </div>
        </div>
      </template>

      <template v-else>
        <div v-if="conflictedEntries.length" class="git-section">
          <div class="git-section-header">
            <span>冲突</span>
            <span class="count">{{ conflictedEntries.length }}</span>
            <div class="spacer"></div>
            <span class="hint">请在外部工具解决冲突后再暂存</span>
          </div>
          <div
            v-for="e in conflictedEntries"
            :key="entryKey(e, 'conflict')"
            class="git-entry"
            :title="e.path"
            @click="openEntryDiff(e, 'unstaged')"
          >
            <span class="status-chip" :class="statusTone('!')">!</span>
            <span class="entry-path">
              {{ fileNameOf(e.path) }}
              <span v-if="dirNameOf(e.path)" class="entry-dir">{{ dirNameOf(e.path) }}</span>
            </span>
            <span class="entry-actions" @click.stop>
              <button
                class="btn icon small subtle"
                title="冲突已解决，暂存该文件"
                :disabled="gitState.busy"
                @click="stagePaths([e.path])"
              >
                <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
                  <path
                    d="M8 3.5v9M3.5 8h9"
                    stroke="currentColor"
                    stroke-width="1.5"
                    stroke-linecap="round"
                  />
                </svg>
              </button>
            </span>
          </div>
        </div>

        <div v-if="stagedEntries.length" class="git-section">
          <div class="git-section-header">
            <span>已暂存</span>
            <span class="count">{{ stagedEntries.length }}</span>
            <div class="spacer"></div>
            <button class="link" :disabled="gitState.busy" @click="unstageAll">全部取消暂存</button>
          </div>
          <div
            v-for="e in stagedEntries"
            :key="entryKey(e, 'staged')"
            class="git-entry"
            :title="e.path"
            @click="openEntryDiff(e, 'staged')"
          >
            <span class="status-chip" :class="statusTone(statusLetter(e, 'staged'))">{{
              statusLetter(e, "staged")
            }}</span>
            <span class="entry-path">
              {{ fileNameOf(e.path) }}
              <span v-if="dirNameOf(e.path)" class="entry-dir">{{ dirNameOf(e.path) }}</span>
            </span>
            <span class="entry-actions" @click.stop>
              <button
                class="btn icon small subtle"
                title="取消暂存"
                :disabled="gitState.busy"
                @click="unstagePaths([e.path])"
              >
                <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
                  <path
                    d="M6.5 3.5H3V7M3.3 6.9a5.2 5.2 0 1 0 1.2-2.2"
                    stroke="currentColor"
                    stroke-width="1.4"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </button>
            </span>
          </div>
        </div>

        <div v-if="unstagedEntries.length" class="git-section">
          <div class="git-section-header">
            <span>未暂存</span>
            <span class="count">{{ unstagedEntries.length }}</span>
            <div class="spacer"></div>
            <button class="link" :disabled="gitState.busy" @click="stageAll">全部暂存</button>
          </div>
          <div
            v-for="e in unstagedEntries"
            :key="entryKey(e, 'unstaged')"
            class="git-entry"
            :title="e.path"
            @click="openEntryDiff(e, 'unstaged')"
          >
            <span class="status-chip" :class="statusTone(statusLetter(e, 'unstaged'))">{{
              statusLetter(e, "unstaged")
            }}</span>
            <span class="entry-path">
              {{ fileNameOf(e.path) }}
              <span v-if="dirNameOf(e.path)" class="entry-dir">{{ dirNameOf(e.path) }}</span>
            </span>
            <span class="entry-actions" @click.stop>
              <button
                class="btn icon small subtle"
                title="暂存该文件"
                :disabled="gitState.busy"
                @click="stagePaths([e.path])"
              >
                <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
                  <path
                    d="M8 3.5v9M3.5 8h9"
                    stroke="currentColor"
                    stroke-width="1.5"
                    stroke-linecap="round"
                  />
                </svg>
              </button>
              <button
                class="btn icon small subtle"
                title="丢弃该文件的未暂存修改"
                :disabled="gitState.busy"
                @click="discardChanges([e.path])"
              >
                <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
                  <path
                    d="M6.5 3.5H3V7M3.3 6.9a5.2 5.2 0 1 0 1.2-2.2"
                    stroke="var(--danger)"
                    stroke-width="1.4"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </button>
            </span>
          </div>
        </div>

        <div v-if="untrackedEntries.length" class="git-section">
          <div class="git-section-header">
            <span>未跟踪</span>
            <span class="count">{{ untrackedEntries.length }}</span>
          </div>
          <div
            v-for="e in untrackedEntries"
            :key="entryKey(e, 'untracked')"
            class="git-entry"
            :title="e.path"
            @click="openEntryDiff(e, 'unstaged')"
          >
            <span class="status-chip" :class="statusTone('?')">?</span>
            <span class="entry-path">
              {{ fileNameOf(e.path) }}
              <span v-if="dirNameOf(e.path)" class="entry-dir">{{ dirNameOf(e.path) }}</span>
            </span>
            <span class="entry-actions" @click.stop>
              <button
                class="btn icon small subtle"
                title="暂存该文件"
                :disabled="gitState.busy"
                @click="stagePaths([e.path])"
              >
                <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
                  <path
                    d="M8 3.5v9M3.5 8h9"
                    stroke="currentColor"
                    stroke-width="1.5"
                    stroke-linecap="round"
                  />
                </svg>
              </button>
              <button
                class="btn icon small subtle"
                title="删除该未跟踪文件"
                :disabled="gitState.busy"
                @click="deleteUntrackedFiles([e.path])"
              >
                <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
                  <path
                    d="M3 4.5h10M6.3 4.5V3h3.4v1.5M4.8 4.5l.5 8.2h5.4l.5-8.2"
                    stroke="var(--danger)"
                    stroke-width="1.3"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </button>
            </span>
          </div>
        </div>

        <div v-if="!hasAnyChange" class="git-empty">工作区没有未提交的变更</div>

        <HistoryList />
      </template>
    </div>

    <div v-if="status?.isRepo" class="git-bottom">
      <div v-if="gitState.aiBusy" class="ai-summary-box">
        <div class="ai-head">
          <span class="spinner"></span>
          <span>正在请求 AI 总结（{{ aiScopeLabel }}）…</span>
          <div class="spacer" style="flex: 1"></div>
          <button class="btn small" @click="cancelAi">取消</button>
        </div>
      </div>

      <div v-else-if="gitState.aiError" class="ai-summary-box">
        <div class="ai-head">
          <span class="ai-error">AI 总结失败</span>
          <div style="flex: 1"></div>
          <button class="btn subtle small icon" title="关闭" @click="dismissAiResult">✕</button>
        </div>
        <div class="ai-body">
          <div class="ai-error">{{ gitState.aiError.message }}</div>
          <div v-if="gitState.aiError.detail" class="detail-block" style="margin-top: 8px">
            {{ gitState.aiError.detail }}
          </div>
        </div>
      </div>

      <div v-else-if="gitState.aiSummary" class="ai-summary-box">
        <div class="ai-head">
          <span>AI 修改总结</span>
          <div style="flex: 1"></div>
          <span class="hint">
            {{ gitState.aiSummary.model }} · {{ gitState.aiSummary.filesIncluded }} 个文件 ·
            {{ formatChars(gitState.aiSummary.totalChars) }} ·
            {{ (gitState.aiSummary.elapsedMs / 1000).toFixed(1) }}s
          </span>
          <button class="btn subtle small icon" title="关闭" @click="dismissAiResult">✕</button>
        </div>
        <div class="ai-body">{{ gitState.aiSummary.summary }}</div>
        <div class="ai-foot">
          <button
            v-if="gitState.aiSummary.commitMessage"
            class="btn small"
            @click="applyAiCommitMessage"
          >
            填入建议提交信息
          </button>
          <div style="flex: 1"></div>
          <span v-if="gitState.aiSummary.redactions" class="hint">
            已脱敏 {{ gitState.aiSummary.redactions }} 处
          </span>
          <span v-if="gitState.aiSummary.truncated" class="hint warn">部分差异被截断</span>
        </div>
      </div>

      <div class="commit-box">
        <textarea
          v-model="gitState.commitMessage"
          class="textarea"
          rows="3"
          placeholder="提交信息（Ctrl+Enter 提交）"
          @keydown.ctrl.enter.prevent="onCommit"
          @keydown.meta.enter.prevent="onCommit"
        ></textarea>
        <div v-if="identityMissing" class="commit-hint hint warn">
          未配置 Git 提交身份（user.name / user.email）。可在系统终端执行：
          <code>git config --global user.name "名字"</code> 与
          <code>git config --global user.email "邮箱"</code>
        </div>
        <div v-else-if="(status?.stagedCount ?? 0) === 0" class="commit-hint">
          没有已暂存的修改（不会自动暂存未暂存的改动）
        </div>
        <div v-else class="commit-hint">
          将提交 {{ status?.stagedCount }} 个已暂存文件<span v-if="status?.unstagedCount"
            >，另有 {{ status.unstagedCount }} 个未暂存文件不会包含在内</span
          >
        </div>
        <div class="commit-actions">
          <button class="btn" :disabled="gitState.busy" @click="startAi">AI 总结本次修改</button>
          <button class="btn primary" :disabled="commitDisabled" @click="onCommit">
            <span v-if="gitState.busy" class="spinner"></span>
            {{ gitState.busy ? "处理中…" : "提交" }}
          </button>
        </div>
      </div>

      <div class="push-row">
        <span class="push-info" :title="pushHint">{{ pushHint }}</span>
        <button
          v-if="canPush"
          class="btn"
          :disabled="gitState.busy"
          @click="push(!status?.upstream)"
        >
          {{ pushLabel }}
        </button>
      </div>
    </div>

    <AiPreviewDialog
      v-if="aiDialogOpen"
      :initial-scope="(status?.stagedCount ?? 0) > 0 ? 'staged' : 'all'"
      @confirm="confirmAi"
      @close="aiDialogOpen = false"
    />
  </div>
</template>

<style scoped>
.git-panel {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}
.git-bottom {
  flex: 0 0 auto;
  border-top: 1px solid var(--border);
  max-height: 55%;
  overflow: auto;
}
.branch-menu-backdrop {
  position: fixed;
  inset: 0;
  z-index: 40;
}
.branch-menu {
  position: absolute;
  top: 34px;
  left: 8px;
  z-index: 41;
  min-width: 220px;
  max-height: 320px;
  overflow: auto;
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  box-shadow: var(--shadow-toast);
  padding: 4px;
}
.branch-menu-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 6px;
  cursor: pointer;
  font-family: var(--mono-font);
  font-size: 12px;
}
.branch-menu-item:hover {
  background: var(--bg-hover);
}
.branch-menu-item.current {
  color: var(--accent);
}
.branch-menu-item.muted {
  color: var(--text-3);
  font-family: var(--ui-font);
  cursor: default;
}
code {
  font-family: var(--mono-font);
  font-size: 11.5px;
  background: var(--bg-app);
  padding: 1px 4px;
  border-radius: 4px;
}
.commit-hint code:first-of-type {
  margin-left: 4px;
}
</style>
