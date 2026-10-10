<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from "vue";
import { dialogState, resolveDialog } from "../stores/dialog";

const req = computed(() => dialogState.request);

function choose(value: string | null): void {
  resolveDialog(value);
}

function onKey(e: KeyboardEvent): void {
  if (!dialogState.request) return;
  if (e.key === "Escape") {
    e.preventDefault();
    choose(null);
  }
}

onMounted(() => window.addEventListener("keydown", onKey));
onBeforeUnmount(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <div v-if="req" class="modal-overlay" @click.self="choose(null)">
    <div class="modal narrow">
      <div class="modal-header">{{ req.title }}</div>
      <div class="modal-body">
        <div style="white-space: pre-wrap">{{ req.message }}</div>
        <div v-if="req.detail" class="detail-block">{{ req.detail }}</div>
      </div>
      <div class="modal-footer">
        <button
          v-for="(c, i) in req.choices"
          :key="c.value"
          class="btn"
          :class="{
            primary: c.kind === 'primary' || (!c.kind && i === 0),
            danger: c.kind === 'danger',
          }"
          @click="choose(c.value)"
        >
          {{ c.label }}
        </button>
      </div>
    </div>
  </div>
</template>
