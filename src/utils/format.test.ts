import { describe, expect, it } from "vitest";
import {
  fileNameOf,
  dirNameOf,
  formatBytes,
  formatChars,
  formatRelativeTime,
  statusLabel,
  statusLetter,
  statusTone,
} from "./format";
import type { GitStatusEntry } from "../types";

function entry(partial: Partial<GitStatusEntry>): GitStatusEntry {
  return {
    path: "a.txt",
    origPath: null,
    x: " ",
    y: " ",
    untracked: false,
    conflicted: false,
    ...partial,
  };
}

describe("路径工具", () => {
  it("从 Windows / 普通路径中取文件名", () => {
    expect(fileNameOf("C:\\项目\\src\\main.ts")).toBe("main.ts");
    expect(fileNameOf("src/components/App.vue")).toBe("App.vue");
    expect(fileNameOf("main.rs")).toBe("main.rs");
  });

  it("取目录部分", () => {
    expect(dirNameOf("src/components/App.vue")).toBe("src/components");
    expect(dirNameOf("App.vue")).toBe("");
  });
});

describe("格式化", () => {
  it("格式化字节", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(2048)).toBe("2.0 KB");
    expect(formatBytes(5 * 1024 * 1024)).toBe("5.00 MB");
  });

  it("格式化字符数", () => {
    expect(formatChars(999)).toBe("999 字符");
    expect(formatChars(2500)).toBe("2.5 千字符");
  });

  it("相对时间", () => {
    const now = Date.now();
    expect(formatRelativeTime(now - 10_000, now)).toBe("刚刚");
    expect(formatRelativeTime(now - 5 * 60_000, now)).toBe("5 分钟前");
    expect(formatRelativeTime(now - 3 * 3600_000, now)).toBe("3 小时前");
    expect(formatRelativeTime(now - 2 * 86400_000, now)).toBe("2 天前");
  });
});

describe("Git 状态显示", () => {
  it("按分组取状态字符", () => {
    expect(statusLetter(entry({ x: "M", y: "M" }), "staged")).toBe("M");
    expect(statusLetter(entry({ x: "M", y: "M" }), "unstaged")).toBe("M");
    expect(statusLetter(entry({ x: "A", y: " " }), "unstaged")).toBe("?");
    expect(statusLetter(entry({ untracked: true, x: "?", y: "?" }), "unstaged")).toBe("?");
    expect(statusLetter(entry({ conflicted: true, x: "U", y: "U" }), "staged")).toBe("!");
  });

  it("状态颜色与文案", () => {
    expect(statusTone("A")).toBe("added");
    expect(statusTone("D")).toBe("deleted");
    expect(statusTone("R")).toBe("renamed");
    expect(statusTone("?")).toBe("untracked");
    expect(statusTone("U")).toBe("conflicted");
    expect(statusLabel("M")).toBe("修改");
    expect(statusLabel("R")).toBe("重命名");
  });
});
