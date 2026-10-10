<script setup lang="ts">
import { computed } from "vue";
import { iconKindForEntry, extensionOf, type FileIconKind } from "../utils/language";

const props = defineProps<{
  name: string;
  isDir: boolean;
  open?: boolean;
}>();

const kind = computed<FileIconKind>(() => iconKindForEntry(props.name, props.isDir));

const COLORS: Record<FileIconKind, string> = {
  folder: "#8b949e",
  code: "#4c8dff",
  vue: "#3fb950",
  web: "#e37933",
  style: "#a371f7",
  data: "#d29922",
  config: "#8b949e",
  markdown: "#58a6ff",
  python: "#4c8dff",
  rust: "#e07a5f",
  java: "#e07a5f",
  php: "#8a8fd8",
  image: "#3fb950",
  archive: "#8b949e",
  lock: "#d29922",
  text: "#8b949e",
  file: "#8b949e",
};

const badge = computed(() => {
  if (props.isDir) return "";
  const lower = props.name.toLowerCase();
  if (lower === "dockerfile") return "DOCK";
  if (lower === "makefile") return "MK";
  if (lower === ".gitignore") return "GIT";
  if (lower.startsWith(".env")) return "ENV";
  const ext = extensionOf(props.name);
  if (!ext || ext.length > 4) return "";
  return ext.toUpperCase().slice(0, 3);
});

const color = computed(() => COLORS[kind.value]);
</script>

<template>
  <span class="entry-icon">
    <svg
      v-if="isDir"
      width="14"
      height="14"
      viewBox="0 0 16 16"
      fill="none"
      :style="{ color }"
      aria-hidden="true"
    >
      <path
        d="M1.8 3.6c0-.5.4-.9.9-.9h3c.3 0 .5.1.7.3l.9 1.1h5.9c.5 0 .9.4.9.9v6.6c0 .5-.4.9-.9.9H2.7c-.5 0-.9-.4-.9-.9V3.6Z"
        stroke="currentColor"
        stroke-width="1.2"
      />
    </svg>
    <span
      v-else-if="badge"
      class="file-badge"
      :style="{
        color,
        backgroundColor: `color-mix(in srgb, ${color} 16%, transparent)`,
      }"
      >{{ badge }}</span
    >
    <svg
      v-else
      width="14"
      height="14"
      viewBox="0 0 16 16"
      fill="none"
      :style="{ color }"
      aria-hidden="true"
    >
      <path
        d="M3.5 1.8h6l3 3v9.4c0 .4-.3.8-.8.8H3.5c-.4 0-.8-.4-.8-.8V2.6c0-.4.4-.8.8-.8Z"
        stroke="currentColor"
        stroke-width="1.2"
      />
      <path d="M9.3 2v3h3" stroke="currentColor" stroke-width="1.2" />
    </svg>
  </span>
</template>

<style scoped>
.file-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 14px;
  border-radius: 3px;
  font-size: 8.5px;
  font-weight: 700;
  letter-spacing: 0.2px;
  font-family: var(--mono-font);
}
</style>
