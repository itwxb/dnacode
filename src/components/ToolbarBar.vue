<script setup lang="ts">
import { computed } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { appState, requestOpenProject, setTheme } from "../stores/app";
import { gitState, refreshAll } from "../stores/git";

const branch = computed(() => {
  const s = gitState.status;
  if (!s || !s.isRepo) return "";
  if (s.detached) return "detached HEAD";
  return s.branch ?? "";
});

const aheadBehind = computed(() => {
  const s = gitState.status;
  if (!s) return "";
  return [s.ahead ? `↑${s.ahead}` : "", s.behind ? `↓${s.behind}` : ""].filter(Boolean).join(" ");
});

async function pickProject(): Promise<void> {
  const selected = await open({ directory: true, multiple: false, title: "选择项目文件夹" });
  if (typeof selected === "string") {
    await requestOpenProject(selected);
  }
}
</script>

<template>
  <div class="toolbar">
    <div class="brand">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <path
          d="M8 2c0 5 8 5 8 10s-8 5-8 10"
          stroke="var(--accent)"
          stroke-width="1.8"
          stroke-linecap="round"
        />
        <path
          d="M16 2c0 5-8 5-8 10s8 5 8 10"
          stroke="var(--accent)"
          stroke-width="1.8"
          stroke-linecap="round"
          opacity="0.5"
        />
        <path
          d="M9.4 7h5.2M9.4 17h5.2"
          stroke="var(--accent)"
          stroke-width="1.5"
          stroke-linecap="round"
        />
      </svg>
      <span>DNACode</span>
    </div>

    <button class="btn small" :disabled="appState.opening" @click="pickProject">打开项目</button>

    <div v-if="appState.project" class="project-label">
      <span class="project-name" :title="appState.project.path">{{ appState.project.name }}</span>
      <span
        v-if="branch"
        class="branch-chip"
        :title="
          gitState.status?.detached
            ? '当前处于 detached HEAD'
            : gitState.status?.upstream
              ? `上游：${gitState.status.upstream}`
              : '本地分支没有上游分支'
        "
      >
        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path d="M5 3.5v5A2.5 2.5 0 0 0 7.5 11H9.5" stroke="currentColor" stroke-width="1.3" />
          <circle cx="5" cy="3.5" r="1.7" stroke="currentColor" stroke-width="1.3" />
          <circle cx="11.5" cy="11" r="1.7" stroke="currentColor" stroke-width="1.3" />
        </svg>
        <span>{{ branch }}</span>
        <span v-if="aheadBehind" class="ahead-behind">{{ aheadBehind }}</span>
      </span>
    </div>

    <div class="spacer"></div>

    <button
      class="btn icon small subtle"
      title="刷新 Git 状态"
      :disabled="!appState.project || gitState.loading"
      @click="refreshAll()"
    >
      <svg
        width="14"
        height="14"
        viewBox="0 0 16 16"
        fill="none"
        :class="{ spin: gitState.loading }"
        aria-hidden="true"
      >
        <path
          d="M13.5 8a5.5 5.5 0 1 1-1.7-4M13.6 2.2v3.2h-3.2"
          stroke="currentColor"
          stroke-width="1.4"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>

    <button
      class="btn icon small subtle"
      :title="appState.theme === 'dark' ? '切换到浅色主题' : '切换到深色主题'"
      @click="setTheme(appState.theme === 'dark' ? 'light' : 'dark')"
    >
      <svg
        v-if="appState.theme === 'dark'"
        width="14"
        height="14"
        viewBox="0 0 16 16"
        fill="none"
        aria-hidden="true"
      >
        <path
          d="M13.4 9.4A5.6 5.6 0 0 1 6.6 2.6a5.6 5.6 0 1 0 6.8 6.8Z"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linejoin="round"
        />
      </svg>
      <svg v-else width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <circle cx="8" cy="8" r="3" stroke="currentColor" stroke-width="1.3" />
        <path
          d="M8 1.6v1.7M8 12.7v1.7M1.6 8h1.7M12.7 8h1.7M3.5 3.5l1.2 1.2M11.3 11.3l1.2 1.2M12.5 3.5l-1.2 1.2M4.7 11.3l-1.2 1.2"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
        />
      </svg>
    </button>

    <button class="btn icon small subtle" title="设置" @click="appState.settingsOpen = true">
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <circle cx="8" cy="8" r="2.2" stroke="currentColor" stroke-width="1.3" />
        <path
          d="M8 1.6v2M8 12.4v2M1.6 8h2M12.4 8h2M3.5 3.5l1.4 1.4M11.1 11.1l1.4 1.4M12.5 3.5l-1.4 1.4M4.9 11.1l-1.4 1.4"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
        />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.spin {
  animation: spin 0.9s linear infinite;
}
</style>
