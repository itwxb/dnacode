import { reactive } from "vue";
import { normalizeError } from "../api";

export type ToastType = "success" | "error" | "info";

export interface ToastItem {
  id: number;
  type: ToastType;
  message: string;
  detail: string | null;
}

const state = reactive({
  items: [] as ToastItem[],
});

let seq = 1;

export const toasts = {
  items: state.items,
  push(type: ToastType, message: string, detail: string | null = null, timeout = 6000): void {
    const id = seq++;
    state.items.push({ id, type, message, detail });
    if (timeout > 0) {
      window.setTimeout(() => toasts.dismiss(id), timeout);
    }
  },
  dismiss(id: number): void {
    const idx = state.items.findIndex((t) => t.id === id);
    if (idx >= 0) state.items.splice(idx, 1);
  },
};

/** 显示一个错误 Toast（附带原始错误详情，便于排查）。 */
export function toastError(e: unknown): void {
  const err = normalizeError(e);
  toasts.push("error", err.message, err.detail, 9000);
}
