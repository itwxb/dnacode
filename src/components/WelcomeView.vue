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
        <svg width="46" height="46" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path
            d="M8 2c0 5 8 5 8 10s-8 5-8 10"
            stroke="var(--accent)"
            stroke-width="1.6"
            stroke-linecap="round"
          />
          <path
            d="M16 2c0 5-8 5-8 10s8 5 8 10"
            stroke="var(--accent)"
            stroke-width="1.6"
            stroke-linecap="round"
            opacity="0.5"
          />
          <path
            d="M9.4 7h5.2M9.4 17h5.2"
            stroke="var(--accent)"
            stroke-width="1.4"
            stroke-linecap="round"
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
