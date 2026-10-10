<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import type { AiPreview } from "../types";
import { api, normalizeError } from "../api";
import { appState } from "../stores/app";
import { formatChars } from "../utils/format";

const props = defineProps<{ initialScope: "staged" | "all" }>();
const emit = defineEmits<{
  (e: "confirm", scope: "staged" | "all"): void;
  (e: "close"): void;
}>();

const scope = ref<"staged" | "all">(props.initialScope);
const preview = ref<AiPreview | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

async function load(): Promise<void> {
  loading.value = true;
  error.value = null;
  preview.value = null;
  try {
    preview.value = await api.aiPreview(scope.value);
  } catch (e) {
    error.value = normalizeError(e).message;
  } finally {
    loading.value = false;
  }
}

watch(scope, load);
onMounted(load);

function totalRedactions(): number {
  return (preview.value?.files ?? []).reduce((sum, f) => sum + f.redactions, 0);
}
</script>

<template>
  <div class="modal-overlay" @click.self="emit('close')">
    <div class="modal">
      <div class="modal-header">
        <span>AI 总结本次修改</span>
        <div class="spacer"></div>
        <button class="btn subtle small icon" title="关闭" @click="emit('close')">✕</button>
      </div>
      <div class="modal-body">
        <div class="radio-row">
          <label>
            <input type="radio" value="staged" v-model="scope" />
            仅已暂存内容
          </label>
          <label>
            <input type="radio" value="all" v-model="scope" />
            全部工作区变更
          </label>
        </div>

        <div v-if="loading" class="hint">正在收集真实差异…</div>
        <div v-else-if="error" class="hint error">{{ error }}</div>

        <template v-else-if="preview">
          <div class="hint">
            将发送 <strong>{{ preview.files.length }}</strong> 个文件、约
            <strong>{{ formatChars(preview.totalChars) }}</strong> 的变更内容到：
            <span class="endpoint">{{ appState.ai.baseUrl }}</span>
          </div>
          <div v-if="totalRedactions() > 0" class="hint warn">
            已自动脱敏 {{ totalRedactions() }} 处疑似密钥/令牌内容。
          </div>
          <div v-if="preview.truncated" class="hint warn">
            部分差异因长度限制被截断，可在设置中调大请求限制。
          </div>
          <div v-if="preview.files.length === 0" class="hint warn">
            没有可发送的变更内容（可能全部为二进制、冲突或为空）。
          </div>

          <div v-if="preview.files.length" class="preview-file-list">
            <div v-for="f in preview.files" :key="f.path" class="preview-file">
              <span class="status-chip" :class="'modified'">{{ f.status }}</span>
              <span class="path" :title="f.path">{{ f.path }}</span>
              <span class="meta">{{ formatChars(f.chars) }}</span>
              <span v-if="f.redactions" class="meta warn">脱敏 {{ f.redactions }}</span>
            </div>
          </div>

          <div v-if="preview.skipped.length" class="hint">
            已跳过 {{ preview.skipped.length }} 个文件：
            {{ preview.skipped.map((s) => `${s.path}（${s.reason}）`).join("；") }}
          </div>

          <div class="hint">
            请求仅在你点击“开始总结”后发出，不会自动上传任何内容；DNACode 不包含遥测。
          </div>
        </template>
      </div>
      <div class="modal-footer">
        <button class="btn" @click="emit('close')">取消</button>
        <button
          class="btn primary"
          :disabled="loading || !preview || preview.files.length === 0"
          @click="emit('confirm', scope)"
        >
          开始总结
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.endpoint {
  font-family: var(--mono-font);
  color: var(--accent);
  word-break: break-all;
}
.preview-file .warn {
  color: var(--warning);
}
</style>
