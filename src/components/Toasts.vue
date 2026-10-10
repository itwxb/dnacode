<script setup lang="ts">
import { toasts } from "../stores/toast";
</script>

<template>
  <div v-if="toasts.items.length" class="toasts">
    <div
      v-for="t in toasts.items"
      :key="t.id"
      class="toast"
      :class="t.type"
      @click="toasts.dismiss(t.id)"
      title="点击关闭"
    >
      <div class="toast-message">{{ t.message }}</div>
      <div v-if="t.detail" class="toast-detail">{{ t.detail }}</div>
    </div>
  </div>
</template>
