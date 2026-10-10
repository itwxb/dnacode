<script setup lang="ts">
import { reactive } from "vue";
import type { CommitFile } from "../types";
import { getCommitFiles, gitState, loadMoreHistory } from "../stores/git";
import { openDiffTab } from "../stores/editor";
import { toastError } from "../stores/toast";
import { formatRelativeTime, statusTone } from "../utils/format";

const expanded = reactive<Record<string, boolean>>({});
const files = reactive<Record<string, CommitFile[]>>({});
const loadingSha = reactive<Record<string, boolean>>({});

async function toggle(sha: string): Promise<void> {
  if (expanded[sha]) {
    expanded[sha] = false;
    return;
  }
  expanded[sha] = true;
  if (!files[sha] && !loadingSha[sha]) {
    loadingSha[sha] = true;
    try {
      files[sha] = await getCommitFiles(sha);
    } catch (e) {
      toastError(e);
    } finally {
      loadingSha[sha] = false;
    }
  }
}

function openCommitFile(sha: string, file: CommitFile): void {
  openDiffTab({ type: "commit", sha, path: file.path });
}
</script>

<template>
  <div class="git-section">
    <div class="git-section-header">
      <span>提交历史</span>
      <span v-if="gitState.history.length" class="count">{{ gitState.history.length }}</span>
    </div>
    <div>
      <div v-if="gitState.history.length === 0 && !gitState.historyLoading" class="git-empty">
        还没有提交记录
      </div>
      <div
        v-for="c in gitState.history"
        :key="c.sha"
        class="history-item"
        @click="toggle(c.sha)"
        :title="c.sha"
      >
        <div class="summary">{{ c.summary }}</div>
        <div class="meta">
          <span class="sha">{{ c.shortSha }}</span>
          <span>{{ c.author }}</span>
          <span>{{ formatRelativeTime(c.timeMs) }}</span>
        </div>
        <div v-if="expanded[c.sha]" class="history-files" @click.stop>
          <div v-if="loadingSha[c.sha]" class="hint">正在读取文件列表…</div>
          <template v-else>
            <div
              v-for="f in files[c.sha] ?? []"
              :key="f.path"
              class="history-file"
              :title="f.origPath ? `${f.origPath} → ${f.path}` : f.path"
              @click="openCommitFile(c.sha, f)"
            >
              <span class="status-chip" :class="statusTone(f.status)">{{ f.status }}</span>
              <span class="path">{{ f.path }}</span>
              <span v-if="f.origPath" class="hint orig">← {{ f.origPath }}</span>
            </div>
            <div v-if="(files[c.sha] ?? []).length === 0" class="hint">没有文件变更</div>
          </template>
        </div>
      </div>
      <div v-if="gitState.historyHasMore" class="history-more">
        <button class="btn small" :disabled="gitState.historyLoading" @click="loadMoreHistory">
          {{ gitState.historyLoading ? "加载中…" : "加载更多" }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.history-more {
  display: flex;
  justify-content: center;
  padding: 8px;
}
.history-file .path {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--mono-font);
}
.history-file .orig {
  font-size: 11px;
  max-width: 45%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
