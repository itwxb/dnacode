<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import { appState, removeRecentProject, requestOpenProject } from "../stores/app";

async function pickProject(): Promise<void> {
  const selected = await open({ directory: true, multiple: false, title: "选择项目文件夹" });
  if (typeof selected === "string") {
    await requestOpenProject(selected);
  }
}
</script>

<template>
  <div class="welcome">
    <div class="welcome-inner">
      <div class="welcome-brand">
        <svg width="46" height="46" viewBox="0 0 1024 1024" fill="none" aria-hidden="true">
          <path
            d="M704 188C704 404 320 620 320 836"
            stroke="var(--brand-rear)"
            stroke-width="68"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
          <path
            d="M370.5 344H653.5M370.5 680H653.5"
            stroke="var(--accent)"
            stroke-width="42"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
          <path
            d="M320 188C320 404 704 620 704 836"
            stroke="var(--accent)"
            stroke-width="68"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        <div>
          <div class="welcome-title">DNACode</div>
          <div class="welcome-subtitle">Minimal Code Editor</div>
        </div>
      </div>

      <div class="welcome-slogan">Less IDE. More Flow.</div>

      <button
        class="btn primary"
        style="height: 34px; font-size: 14px"
        :disabled="appState.opening"
        @click="pickProject"
      >
        {{ appState.opening ? "正在打开…" : "打开项目文件夹" }}
      </button>

      <div v-if="appState.recentProjects.length" class="recent-list">
        <div
          v-for="r in appState.recentProjects"
          :key="r.path"
          class="recent-item"
          @click="requestOpenProject(r.path)"
        >
          <span class="name">{{ r.name }}</span>
          <span class="path" :title="r.path">{{ r.path }}</span>
          <button
            class="btn subtle small icon"
            title="从列表移除"
            @click.stop="removeRecentProject(r.path)"
          >
            ✕
          </button>
        </div>
      </div>
      <div v-else class="hint">还没有最近项目。选择一个本地文件夹即可开始查看代码与提交变更。</div>
    </div>
  </div>
</template>
