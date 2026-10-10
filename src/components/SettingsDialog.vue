<script setup lang="ts">
import { onMounted, reactive, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import type { AiTestResult } from "../types";
import { api, normalizeError } from "../api";
import {
  appState,
  clearRecentProjects,
  deleteApiKey,
  removeRecentProject,
  saveAiSettings,
  saveApiKey,
  setTheme,
} from "../stores/app";
import { gitState } from "../stores/git";
import { toasts } from "../stores/toast";

const emit = defineEmits<{ (e: "close"): void }>();

const form = reactive({
  baseUrl: appState.ai.baseUrl,
  model: appState.ai.model,
  timeoutSecs: appState.ai.timeoutSecs,
  maxDiffChars: appState.ai.maxDiffChars,
});

const apiKeyInput = ref("");
const savingKey = ref(false);
const testing = ref(false);
const testResult = ref<string | null>(null);
const testOk = ref(false);
const savedHint = ref(false);
const version = ref("");

onMounted(async () => {
  try {
    version.value = await getVersion();
  } catch {
    version.value = "0.1.0";
  }
});

async function saveAll(): Promise<void> {
  const ok = await saveAiSettings({
    baseUrl: form.baseUrl.trim(),
    model: form.model.trim(),
    timeoutSecs: Number(form.timeoutSecs) || 60,
    maxDiffChars: Number(form.maxDiffChars) || 60000,
  });
  if (ok) {
    savedHint.value = true;
    window.setTimeout(() => (savedHint.value = false), 2500);
    toasts.push("success", "设置已保存");
  }
}

async function saveKey(): Promise<void> {
  const key = apiKeyInput.value.trim();
  if (!key) return;
  savingKey.value = true;
  try {
    if (await saveApiKey(key)) {
      apiKeyInput.value = "";
    }
  } finally {
    savingKey.value = false;
  }
}

async function removeKey(): Promise<void> {
  await deleteApiKey();
}

async function testConnection(): Promise<void> {
  testing.value = true;
  testResult.value = null;
  try {
    const r: AiTestResult = await api.aiTestConnection();
    testOk.value = true;
    testResult.value = `连接成功：模型 ${r.model}，耗时 ${r.latencyMs} ms`;
  } catch (e) {
    testOk.value = false;
    const err = normalizeError(e);
    testResult.value = err.detail ? `${err.message}\n${err.detail}` : err.message;
  } finally {
    testing.value = false;
  }
}

function identityText(): string {
  const s = gitState.status;
  if (!s || !appState.project) return "打开项目后显示";
  if (!s.isRepo) return "当前项目不是 Git 仓库";
  if (!s.identityName || !s.identityEmail) {
    return "未配置（提交前需要配置 user.name / user.email）";
  }
  return `${s.identityName} <${s.identityEmail}>`;
}
</script>

<template>
  <div class="modal-overlay" @click.self="emit('close')">
    <div class="modal">
      <div class="modal-header">
        <span>设置</span>
        <div class="spacer"></div>
        <button class="btn subtle small icon" title="关闭" @click="emit('close')">✕</button>
      </div>

      <div class="modal-body">
        <section class="settings-section">
          <h3>外观</h3>
          <div class="radio-row">
            <label>
              <input
                type="radio"
                value="dark"
                :checked="appState.theme === 'dark'"
                @change="setTheme('dark')"
              />
              深色（默认）
            </label>
            <label>
              <input
                type="radio"
                value="light"
                :checked="appState.theme === 'light'"
                @change="setTheme('light')"
              />
              浅色
            </label>
          </div>
        </section>

        <section class="settings-section">
          <h3>AI（可选，OpenAI 兼容 API）</h3>
          <div>
            <label class="field-label">API Base URL</label>
            <input
              v-model="form.baseUrl"
              class="input"
              placeholder="例如：https://api.openai.com/v1 或 https://api.deepseek.com"
            />
          </div>
          <div>
            <label class="field-label">模型名称</label>
            <input
              v-model="form.model"
              class="input"
              placeholder="例如：gpt-4o-mini / deepseek-chat"
            />
          </div>
          <div>
            <label class="field-label"> API Key（存储在系统安全凭据中，不会写入配置文件） </label>
            <div class="settings-row">
              <input
                v-model="apiKeyInput"
                class="input grow"
                type="password"
                autocomplete="off"
                :placeholder="appState.hasApiKey ? '已保存（输入新 Key 可覆盖）' : '输入 API Key'"
              />
              <button class="btn" :disabled="savingKey || !apiKeyInput.trim()" @click="saveKey">
                保存
              </button>
              <button v-if="appState.hasApiKey" class="btn danger" @click="removeKey">删除</button>
            </div>
            <div v-if="appState.keyringError" class="hint error">{{ appState.keyringError }}</div>
            <div v-else-if="appState.hasApiKey" class="hint ok">
              API Key 已保存在系统安全凭据存储中
            </div>
            <div v-else class="hint">尚未保存 API Key</div>
          </div>
          <div class="settings-row">
            <div class="grow">
              <label class="field-label">请求超时（秒）</label>
              <input
                v-model.number="form.timeoutSecs"
                class="input"
                type="number"
                min="5"
                max="600"
              />
            </div>
            <div class="grow">
              <label class="field-label">单次请求差异上限（字符）</label>
              <input
                v-model.number="form.maxDiffChars"
                class="input"
                type="number"
                min="2000"
                max="400000"
                step="1000"
              />
            </div>
          </div>
          <div class="settings-row">
            <button class="btn primary" @click="saveAll">保存设置</button>
            <button class="btn" :disabled="testing || !appState.hasApiKey" @click="testConnection">
              <span v-if="testing" class="spinner"></span>
              {{ testing ? "测试中…" : "测试连接" }}
            </button>
            <span v-if="savedHint" class="hint ok">已保存</span>
          </div>
          <div
            v-if="testResult"
            class="hint"
            :class="testOk ? 'ok' : 'error'"
            style="white-space: pre-wrap"
          >
            {{ testResult }}
          </div>
          <div class="hint">
            AI 仅在点击「AI 总结本次修改」时被调用；发送前会展示将上传的文件列表并自动脱敏疑似密钥；
            未配置 AI 不影响其他功能。
          </div>
        </section>

        <section class="settings-section">
          <h3>Git 提交身份</h3>
          <div class="hint">{{ identityText() }}</div>
          <div class="hint">
            提交身份沿用系统的 Git 配置。若未配置，可在终端执行：<br />
            <code>git config --global user.name "你的名字"</code><br />
            <code>git config --global user.email "你的邮箱"</code>
          </div>
        </section>

        <section class="settings-section">
          <h3>最近项目</h3>
          <div v-if="appState.recentProjects.length" class="recent-list">
            <div v-for="r in appState.recentProjects" :key="r.path" class="recent-item">
              <span class="name">{{ r.name }}</span>
              <span class="path" :title="r.path">{{ r.path }}</span>
              <button
                class="btn subtle small icon"
                title="移除"
                @click="removeRecentProject(r.path)"
              >
                ✕
              </button>
            </div>
          </div>
          <div v-else class="hint">暂无最近项目</div>
          <div v-if="appState.recentProjects.length">
            <button class="btn small" @click="clearRecentProjects">清空最近项目</button>
          </div>
        </section>

        <section class="settings-section">
          <h3>文件树忽略规则</h3>
          <div class="hint">
            默认忽略 .git、node_modules、dist、build、coverage、target、__pycache__、.venv 等
            依赖缓存与构建产物目录；隐藏文件（. 开头）会以较暗的样式显示，不会被隐藏。
          </div>
          <div class="hint">本应用不包含任何遥测；不会在未经操作的情况下上传任何仓库内容。</div>
        </section>

        <section class="settings-section">
          <h3>关于</h3>
          <div class="hint">
            DNACode v{{ version }} — Minimal Code Editor<br />
            Less IDE. More Flow.<br />
            Tauri 2 · Rust · Vue 3 · TypeScript · CodeMirror 6
          </div>
        </section>
      </div>

      <div class="modal-footer">
        <button class="btn" @click="emit('close')">关闭</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
code {
  font-family: var(--mono-font);
  font-size: 11.5px;
  background: var(--bg-app);
  padding: 1px 4px;
  border-radius: 4px;
}
</style>
