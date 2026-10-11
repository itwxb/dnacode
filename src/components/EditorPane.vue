<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { Compartment, EditorState, type Extension } from "@codemirror/state";
import {
  EditorView,
  crosshairCursor,
  drawSelection,
  dropCursor,
  highlightActiveLine,
  highlightActiveLineGutter,
  highlightSpecialChars,
  keymap,
  lineNumbers,
  rectangularSelection,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import {
  HighlightStyle,
  bracketMatching,
  foldGutter,
  foldKeymap,
  indentOnInput,
  syntaxHighlighting,
} from "@codemirror/language";
import { closeBrackets, closeBracketsKeymap } from "@codemirror/autocomplete";
import { highlightSelectionMatches, searchKeymap } from "@codemirror/search";
import { tags as t } from "@lezer/highlight";
import {
  activateTab,
  closeTab,
  editorState,
  findTab,
  getSavedSnapshot,
  isDocDirtyText,
  saveActiveTab,
  setEditorBridge,
  type EditorTab,
  type FileTab,
} from "../stores/editor";
import { loadLanguageForFile } from "../utils/language";
import DiffView from "./DiffView.vue";

const host = ref<HTMLElement | null>(null);
let view: EditorView | null = null;
const mountedTabId = ref<string | null>(null);
const states = new Map<string, EditorState>();
const pendingContent = new Map<string, string>();
const languageCompartment = new Compartment();

const activeFileTab = computed<FileTab | null>(() => {
  const id = editorState.activeId;
  if (!id) return null;
  const tab = findTab(id);
  return tab && tab.kind === "file" ? (tab as FileTab) : null;
});

const activeDiffTab = computed<EditorTab | null>(() => {
  const id = editorState.activeId;
  if (!id) return null;
  const tab = findTab(id);
  return tab && tab.kind === "diff" ? tab : null;
});

// ---------------------------------------------------------------------------
// 主题与高亮：全部使用 CSS 变量，跟随深色/浅色主题切换
// ---------------------------------------------------------------------------

const editorTheme = EditorView.theme({
  "&": {
    height: "100%",
    color: "var(--text-1)",
    backgroundColor: "var(--bg-editor)",
  },
  ".cm-content": {
    caretColor: "var(--text-1)",
    fontFamily: "var(--mono-font)",
    fontSize: "13px",
    padding: "4px 0",
  },
  ".cm-line": { padding: "0 10px" },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--accent)" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
    backgroundColor: "var(--selection-bg)",
  },
  ".cm-gutters": {
    backgroundColor: "var(--bg-editor)",
    color: "var(--text-3)",
    border: "none",
  },
  ".cm-lineNumbers .cm-gutterElement": { padding: "0 10px 0 14px", minWidth: "36px" },
  ".cm-activeLineGutter": {
    backgroundColor: "var(--active-line-bg)",
    color: "var(--text-2)",
  },
  ".cm-activeLine": { backgroundColor: "var(--active-line-bg)" },
  ".cm-foldPlaceholder": {
    backgroundColor: "transparent",
    border: "1px solid var(--border)",
    color: "var(--text-2)",
  },
  ".cm-searchMatch": {
    backgroundColor: "var(--search-match-bg)",
    outline: "1px solid var(--border-strong)",
  },
  ".cm-searchMatch.cm-searchMatch-selected": {
    backgroundColor: "var(--search-match-selected-bg)",
  },
  ".cm-panels": { backgroundColor: "var(--bg-panel)", color: "var(--text-1)" },
  ".cm-panels.cm-panels-top": { borderBottom: "1px solid var(--border)" },
  ".cm-panels.cm-panels-bottom": { borderTop: "1px solid var(--border)" },
  ".cm-panel input, .cm-panel button, .cm-panel select": {
    fontFamily: "var(--ui-font)",
    fontSize: "12px",
    background: "var(--bg-app)",
    color: "var(--text-1)",
    border: "1px solid var(--border-strong)",
    borderRadius: "4px",
    padding: "2px 6px",
  },
  ".cm-panel label": { color: "var(--text-2)", fontSize: "12px" },
  ".cm-tooltip": {
    backgroundColor: "var(--bg-elevated)",
    border: "1px solid var(--border-strong)",
    color: "var(--text-1)",
  },
  ".cm-tooltip-autocomplete ul li[aria-selected]": { backgroundColor: "var(--accent-muted)" },
});

const highlightStyle = HighlightStyle.define([
  { tag: t.comment, color: "var(--syn-comment)", fontStyle: "italic" },
  { tag: t.keyword, color: "var(--syn-keyword)" },
  { tag: [t.string, t.special(t.string)], color: "var(--syn-string)" },
  { tag: [t.number, t.bool, t.null], color: "var(--syn-number)" },
  { tag: [t.function(t.variableName), t.function(t.propertyName)], color: "var(--syn-function)" },
  {
    tag: [t.typeName, t.className, t.namespace, t.definition(t.typeName)],
    color: "var(--syn-type)",
  },
  { tag: [t.propertyName, t.variableName], color: "var(--syn-property)" },
  { tag: [t.operator, t.punctuation, t.separator], color: "var(--syn-operator)" },
  { tag: [t.tagName, t.angleBracket], color: "var(--syn-tag)" },
  { tag: [t.attributeName], color: "var(--syn-attr)" },
  { tag: [t.heading, t.strong], color: "var(--text-1)", fontWeight: "600" },
  { tag: t.emphasis, fontStyle: "italic" },
  { tag: t.link, color: "var(--syn-link)", textDecoration: "underline" },
  { tag: t.invalid, color: "var(--danger)" },
]);

function buildExtensions(lang: Extension | null): Extension[] {
  return [
    lineNumbers(),
    highlightActiveLineGutter(),
    highlightSpecialChars(),
    history(),
    foldGutter(),
    drawSelection(),
    dropCursor(),
    EditorState.allowMultipleSelections.of(true),
    indentOnInput(),
    bracketMatching(),
    closeBrackets(),
    highlightActiveLine(),
    rectangularSelection(),
    crosshairCursor(),
    highlightSelectionMatches(),
    syntaxHighlighting(highlightStyle, { fallback: true }),
    keymap.of([
      {
        key: "Mod-s",
        preventDefault: true,
        run: () => {
          void saveActiveTab();
          return true;
        },
      },
      ...closeBracketsKeymap,
      ...defaultKeymap,
      ...searchKeymap,
      ...historyKeymap,
      ...foldKeymap,
      indentWithTab,
    ]),
    languageCompartment.of(lang ? [lang] : []),
    editorTheme,
    EditorView.updateListener.of((update) => {
      const id = mountedTabId.value;
      if (!id) return;
      if (update.docChanged || update.selectionSet) {
        states.set(id, update.state);
      }
      if (update.docChanged) {
        const tab = findTab(id);
        if (tab && tab.kind === "file") {
          tab.dirty = isDocDirtyText(id, update.state.doc);
        }
      }
    }),
  ];
}

// ---------------------------------------------------------------------------
// 标签切换与 CodeMirror 状态管理
// ---------------------------------------------------------------------------

async function mountFor(id: string): Promise<void> {
  const tab = findTab(id);
  if (!tab || tab.kind !== "file" || !view) return;

  const cached = states.get(id);
  if (cached) {
    mountedTabId.value = id;
    view.setState(cached);
    view.requestMeasure();
    return;
  }

  const content = pendingContent.get(id) ?? getSavedSnapshot(id) ?? "";
  pendingContent.delete(id);
  const lang = await loadLanguageForFile(tab.title);
  if (!view || editorState.activeId !== id) return;

  const state = EditorState.create({ doc: content, extensions: buildExtensions(lang) });
  states.set(id, state);
  mountedTabId.value = id;
  view.setState(state);
  view.requestMeasure();
}

const bridgeImpl = {
  attachContent(id: string, content: string): void {
    pendingContent.set(id, content);
    states.delete(id);
    if (editorState.activeId === id) void mountFor(id);
  },
  getDoc(id: string): string | null {
    if (view && mountedTabId.value === id) return view.state.doc.toString();
    const cached = states.get(id);
    if (cached) return cached.doc.toString();
    return getSavedSnapshot(id) ?? null;
  },
  markSaved(id: string, _content: string): void {
    if (view && mountedTabId.value === id) states.set(id, view.state);
  },
  disposeTab(id: string): void {
    states.delete(id);
    pendingContent.delete(id);
    if (mountedTabId.value === id) mountedTabId.value = null;
  },
};

watch(
  () => editorState.activeId,
  (id) => {
    if (!view) return;
    if (!id) {
      mountedTabId.value = null;
      return;
    }
    const tab = findTab(id);
    if (!tab || tab.kind !== "file") {
      mountedTabId.value = null;
      return;
    }
    void mountFor(id);
  },
);

onMounted(() => {
  if (!host.value) return;
  view = new EditorView({
    state: EditorState.create({ doc: "", extensions: buildExtensions(null) }),
    parent: host.value,
  });
  setEditorBridge(bridgeImpl);
  const id = editorState.activeId;
  if (id) void mountFor(id);
});

onBeforeUnmount(() => {
  setEditorBridge(null);
  view?.destroy();
  view = null;
});

function tabKindLabel(tab: EditorTab): string {
  if (tab.kind !== "diff") return "";
  if (tab.request.type === "staged") return "已暂存";
  if (tab.request.type === "worktree") return "未暂存";
  return "历史";
}

function tabTooltip(tab: EditorTab): string {
  if (tab.kind === "file") return tab.path;
  if (tab.request.type === "commit") {
    return tab.request.path
      ? `${tab.request.path} @ ${tab.request.sha}`
      : `提交 ${tab.request.sha}`;
  }
  return tab.request.path;
}
</script>

<template>
  <div class="editor-pane">
    <div v-if="editorState.tabs.length" class="tabs-bar">
      <div
        v-for="tab in editorState.tabs"
        :key="tab.id"
        class="tab"
        :class="{ active: tab.id === editorState.activeId }"
        @click="activateTab(tab.id)"
        @auxclick.middle.prevent="closeTab(tab.id)"
      >
        <span v-if="tab.kind === 'diff'" class="tab-kind">{{ tabKindLabel(tab) }}</span>
        <span class="tab-title" :title="tabTooltip(tab)">{{ tab.title }}</span>
        <span
          v-if="tab.kind === 'file' && tab.externalChanged"
          class="external-badge"
          title="文件已被外部程序修改，保存时会提示冲突"
          >⚠</span
        >
        <span v-if="tab.kind === 'file' && tab.dirty" class="dirty-dot" title="未保存的修改"></span>
        <button class="close-btn" title="关闭标签" @click.stop="closeTab(tab.id)">
          <svg width="9" height="9" viewBox="0 0 10 10" aria-hidden="true">
            <path
              d="M1.5 1.5l7 7M8.5 1.5l-7 7"
              stroke="currentColor"
              stroke-width="1.3"
              stroke-linecap="round"
            />
          </svg>
        </button>
      </div>
    </div>

    <div class="editor-body">
      <div v-show="activeFileTab" class="editor-host">
        <div ref="host" class="cm-host"></div>
        <div v-if="activeFileTab?.loading" class="editor-loading">
          <span class="spinner"></span>
        </div>
      </div>

      <DiffView v-if="activeDiffTab" />

      <div v-if="!activeFileTab && !activeDiffTab" class="editor-placeholder">
        <svg
          width="36"
          height="36"
          viewBox="0 0 1024 1024"
          fill="none"
          aria-hidden="true"
          opacity="0.5"
        >
          <path
            d="M704 188C704 404 320 620 320 836"
            stroke="var(--text-3)"
            stroke-width="68"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
          <path
            d="M370.5 344H653.5M370.5 680H653.5"
            stroke="var(--text-3)"
            stroke-width="42"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
          <path
            d="M320 188C320 404 704 620 704 836"
            stroke="var(--text-3)"
            stroke-width="68"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        <div>从左侧文件树选择文件开始查看和编辑</div>
        <div class="hint">Ctrl+S 保存 · 点击右侧 Git 面板中的文件可查看真实差异</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.editor-pane {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}
.editor-body {
  flex: 1;
  min-height: 0;
  position: relative;
  display: flex;
  flex-direction: column;
}
.cm-host {
  height: 100%;
}
</style>
