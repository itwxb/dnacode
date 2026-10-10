<script setup lang="ts">
import { ref, watch } from "vue";
import { api, normalizeError } from "../api";
import type { DirEntry } from "../types";
import { appState } from "../stores/app";
import TreeNode from "./TreeNode.vue";

const entries = ref<DirEntry[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

async function reload(): Promise<void> {
  const project = appState.project;
  if (!project) {
    entries.value = [];
    return;
  }
  loading.value = true;
  error.value = null;
  try {
    entries.value = await api.listDirectory(project.path);
  } catch (e) {
    error.value = normalizeError(e).message;
    entries.value = [];
  } finally {
    loading.value = false;
  }
}

watch(() => appState.project?.path, reload, { immediate: true });
</script>

<template>
  <div class="file-tree">
    <div class="panel-header">
      <span>项目文件</span>
      <div class="spacer"></div>
      <span v-if="loading" class="spinner"></span>
      <button class="btn icon small subtle" title="刷新目录" :disabled="loading" @click="reload">
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
    <div class="panel-body">
      <div v-if="error" class="tree-empty">{{ error }}</div>
      <template v-else>
        <TreeNode v-for="e in entries" :key="e.path" :entry="e" :depth="0" />
        <div v-if="!loading && entries.length === 0" class="tree-empty">当前文件夹是空的</div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.file-tree {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}
</style>
