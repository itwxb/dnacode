import { reactive } from "vue";

export interface DialogChoice {
  value: string;
  label: string;
  kind?: "primary" | "danger" | "cancel";
}

export interface DialogRequest {
  title: string;
  message: string;
  detail?: string | null;
  choices: DialogChoice[];
}

const state = reactive({
  request: null as DialogRequest | null,
});

let resolver: ((value: string | null) => void) | null = null;

export const dialogState = state;

/** 打开一个带结果的确认对话框；返回值是所选 choice 的 value，取消/关闭返回 null。 */
export function askDialog(req: DialogRequest): Promise<string | null> {
  if (resolver) {
    resolver(null);
    resolver = null;
  }
  return new Promise((resolve) => {
    resolver = resolve;
    state.request = req;
  });
}

export function resolveDialog(value: string | null): void {
  const r = resolver;
  resolver = null;
  state.request = null;
  r?.(value);
}
