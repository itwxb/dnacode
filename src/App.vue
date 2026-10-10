<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import ToolbarBar from "./components/ToolbarBar.vue";
import WelcomeView from "./components/WelcomeView.vue";
import FileTree from "./components/FileTree.vue";
import EditorPane from "./components/EditorPane.vue";
import GitPanel from "./components/GitPanel.vue";
import SettingsDialog from "./components/SettingsDialog.vue";
import ConfirmDialog from "./components/ConfirmDialog.vue";
import Toasts from "./components/Toasts.vue";
import { appState, loadSettings } from "./stores/app";
import { editorState, saveActiveTab } from "./stores/editor";
import { refreshStatus } from "./stores/git";

const sidebarWidth = ref(220);
const gitWidth = ref(340);

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

let dragging: "left" | "right" | null = null;

function startDrag(side: "left" | "right", ev: PointerEvent): void {
  dragging = side;
  const startX = ev.clientX;
  const startSidebar = sidebarWidth.value;
  const startGit = gitWidth.value;

  const onMove = (e: PointerEvent): void => {
    if (dragging === "left") {
      sidebarWidth.value = clamp(startSidebar + (e.clientX - startX), 160, 520);
    } else if (dragging === "right") {
      gitWidth.value = clamp(startGit - (e.clientX - startX), 280, 640);
    }
  };
  const onUp = (): void => {
    dragging = null;
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
  };
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
}

function onKeydown(e: KeyboardEvent): void {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
    e.preventDefault();
    if (editorState.activeId) void saveActiveTab();
  }
}

function onWindowFocus(): void {
  void refreshStatus();
}

onMounted(async () => {
  window.addEventListener("keydown", onKeydown);
  window.addEventListener("focus", onWindowFocus);
  await loadSettings();
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
  window.removeEventListener("focus", onWindowFocus);
});
</script>

<template>
  <div class="app-shell">
    <ToolbarBar />

    <div v-if="appState.project" class="main-grid">
      <div class="sidebar" :style="{ width: sidebarWidth + 'px' }">
        <FileTree />
      </div>
      <div class="splitter" @pointerdown="startDrag('left', $event)"></div>

      <EditorPane class="editor-area" />

      <div class="splitter" @pointerdown="startDrag('right', $event)"></div>
      <div class="git-area" :style="{ width: gitWidth + 'px' }">
        <GitPanel />
      </div>
    </div>

    <WelcomeView v-else />

    <SettingsDialog v-if="appState.settingsOpen" @close="appState.settingsOpen = false" />
    <ConfirmDialog />
    <Toasts />
  </div>
</template>
