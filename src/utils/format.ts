/** 通用格式化与 Git 状态显示工具（纯函数，便于单元测试）。 */

import type { GitStatusEntry } from "../types";

export function fileNameOf(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

export function dirNameOf(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx > 0 ? path.slice(0, idx) : "";
}

function pad(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

export function formatDateTime(ms: number): string {
  if (!ms) return "";
  const d = new Date(ms);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function formatRelativeTime(ms: number, now: number = Date.now()): string {
  if (!ms) return "";
  const diff = now - ms;
  if (diff < 0) return formatDateTime(ms);
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;
  if (diff < minute) return "刚刚";
  if (diff < hour) return `${Math.floor(diff / minute)} 分钟前`;
  if (diff < day) return `${Math.floor(diff / hour)} 小时前`;
  if (diff < 30 * day) return `${Math.floor(diff / day)} 天前`;
  return formatDateTime(ms);
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MB`;
}

export function formatChars(n: number): string {
  if (n < 1000) return `${n} 字符`;
  return `${(n / 1000).toFixed(1)} 千字符`;
}

/** 条目在指定分组中展示的状态字符。 */
export function statusLetter(entry: GitStatusEntry, side: "staged" | "unstaged"): string {
  if (entry.untracked) return "?";
  if (entry.conflicted) return "!";
  const ch = side === "staged" ? entry.x : entry.y;
  return ch === " " || !ch ? "?" : ch;
}

export type StatusTone =
  "added" | "modified" | "deleted" | "renamed" | "copied" | "untracked" | "conflicted";

export function statusTone(letter: string): StatusTone {
  switch (letter.toUpperCase()) {
    case "A":
      return "added";
    case "D":
      return "deleted";
    case "R":
      return "renamed";
    case "C":
      return "copied";
    case "?":
      return "untracked";
    case "!":
    case "U":
      return "conflicted";
    default:
      return "modified";
  }
}

export function statusLabel(letter: string): string {
  switch (letter.toUpperCase()) {
    case "A":
      return "新增";
    case "M":
      return "修改";
    case "D":
      return "删除";
    case "R":
      return "重命名";
    case "C":
      return "复制";
    case "T":
      return "类型变更";
    case "?":
      return "未跟踪";
    case "!":
    case "U":
      return "冲突";
    default:
      return "变更";
  }
}
