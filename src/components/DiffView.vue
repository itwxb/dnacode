<script setup lang="ts">
import { computed } from "vue";
import { editorState, findTab, loadFullDiff, type DiffTab } from "../stores/editor";
import { parseUnifiedDiff } from "../utils/diff";

const tab = computed<DiffTab | null>(() => {
  const id = editorState.activeId;
  if (!id) return null;
  const t = findTab(id);
  return t && t.kind === "diff" ? t : null;
});

const rows = computed(() => {
  const payload = tab.value?.payload;
  if (!payload || payload.kind !== "text" || !payload.text) return [];
  return parseUnifiedDiff(payload.text);
});

const badge = computed(() => {
  const req = tab.value?.request;
  if (!req) return "";
  if (req.type === "staged") return "已暂存";
  if (req.type === "worktree") return "未暂存";
  return "提交历史";
});

const displayPath = computed(() => {
  const req = tab.value?.request;
  if (!req) return "";
  if (req.type === "commit") return req.path ?? req.sha;
  return req.path;
});
</script>

<template>
  <div v-if="tab" class="diff-view">
    <div class="diff-toolbar">
      <span class="tab-kind">{{ badge }}</span>
      <span class="diff-path" :title="displayPath">{{ displayPath }}</span>
      <span style="flex: 1"></span>
      <span v-if="tab.payload && tab.payload.kind === 'text'" class="diff-stats">
        <span class="add-count">+{{ tab.payload.added }}</span>
        <span class="del-count">-{{ tab.payload.removed }}</span>
      </span>
      <button
        v-if="tab.payload?.truncated && !tab.showingFull"
        class="btn small"
        @click="loadFullDiff(tab.id)"
      >
        加载完整差异
      </button>
    </div>

    <div v-if="tab.loading" class="diff-notice">
      <span class="spinner"></span>
      <span>正在读取差异…</span>
    </div>

    <div v-else-if="tab.payload" class="diff-body">
      <template v-if="tab.payload.kind === 'text'">
        <div v-for="(row, i) in rows" :key="i" class="diff-line" :class="row.type">
          <template v-if="row.type === 'add' || row.type === 'del' || row.type === 'context'">
            <span class="ln">{{ row.oldNo ?? "" }}</span>
            <span class="ln new-ln">{{ row.newNo ?? "" }}</span>
            <span class="content">{{ row.text }}</span>
          </template>
          <template v-else>
            <span class="content">{{ row.text }}</span>
          </template>
        </div>
      </template>
      <div v-else class="diff-notice">
        <div>{{ tab.payload.message ?? "没有可显示的差异" }}</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.diff-stats {
  display: inline-flex;
  gap: 8px;
  font-family: var(--mono-font);
}
.diff-line.hunk .content,
.diff-line.meta .content,
.diff-line.nonewline .content {
  padding-left: 10px;
}
</style>
