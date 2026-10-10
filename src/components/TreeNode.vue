<script setup lang="ts">
import { computed, ref } from "vue";
import { api, normalizeError } from "../api";
import type { DirEntry } from "../types";
import { activateTab, editorState, openFileTab } from "../stores/editor";
import FileIcon from "./FileIcon.vue";

const props = defineProps<{ entry: DirEntry; depth: number }>();

const expanded = ref(false);
const children = ref<DirEntry[] | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

const isActiveFile = computed(() => {
  const tab = editorState.tabs.find((t) => t.id === editorState.activeId);
  return tab?.kind === "file" && tab.path === props.entry.path;
});

async function load(): Promise<void> {
  loading.value = true;
  error.value = null;
  try {
    children.value = await api.listDirectory(props.entry.path);
  } catch (e) {
    error.value = normalizeError(e).message;
    children.value = [];
  } finally {
    loading.value = false;
  }
}

async function onClick(): Promise<void> {
  if (props.entry.isDir) {
    expanded.value = !expanded.value;
    if (expanded.value && children.value === null) {
      await load();
    }
  } else {
    await openFileTab(props.entry.path, props.entry.name);
    activateTab(`file:${props.entry.path}`);
  }
}
</script>

<template>
  <div>
    <div
      class="tree-row"
      :class="{ selected: isActiveFile, 'hidden-entry': entry.hidden }"
      :style="{ paddingLeft: 8 + depth * 14 + 'px' }"
      :title="entry.path"
      @click="onClick"
    >
      <span v-if="entry.isDir" class="chevron" :class="{ open: expanded }">
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <path
            d="M3.5 1.5 7 5 3.5 8.5"
            stroke="currentColor"
            stroke-width="1.4"
            fill="none"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </span>
      <span v-else class="chevron"></span>
      <FileIcon :name="entry.name" :is-dir="entry.isDir" />
      <span class="entry-name">{{ entry.name }}</span>
      <span v-if="entry.isSymlink" class="hint" title="符号链接">↗</span>
      <span v-if="loading" class="spinner"></span>
    </div>

    <template v-if="entry.isDir && expanded">
      <div v-if="error" class="tree-empty" :style="{ paddingLeft: 22 + (depth + 1) * 14 + 'px' }">
        {{ error }}
      </div>
      <template v-else-if="children !== null">
        <TreeNode v-for="c in children" :key="c.path" :entry="c" :depth="depth + 1" />
        <div
          v-if="children.length === 0 && !loading"
          class="tree-empty"
          :style="{ paddingLeft: 22 + (depth + 1) * 14 + 'px' }"
        >
          空文件夹
        </div>
      </template>
    </template>
  </div>
</template>
