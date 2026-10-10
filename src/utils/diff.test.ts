import { describe, expect, it } from "vitest";
import { parseUnifiedDiff } from "./diff";

describe("parseUnifiedDiff", () => {
  it("解析新增、删除、上下文与行号", () => {
    const text = [
      "diff --git a/a.txt b/a.txt",
      "index 7898192..6178079 100644",
      "--- a/a.txt",
      "+++ b/a.txt",
      "@@ -1,3 +1,3 @@",
      " one",
      "-two",
      "+TWO",
      " three",
      "",
    ].join("\n");

    const rows = parseUnifiedDiff(text);
    const kinds = rows.map((r) => r.type);
    expect(kinds).toEqual([
      "meta",
      "meta",
      "meta",
      "meta",
      "hunk",
      "context",
      "del",
      "add",
      "context",
    ]);

    const del = rows.find((r) => r.type === "del")!;
    expect(del.oldNo).toBe(2);
    expect(del.newNo).toBeNull();
    expect(del.text).toBe("two");

    const add = rows.find((r) => r.type === "add")!;
    expect(add.oldNo).toBeNull();
    expect(add.newNo).toBe(2);
    expect(add.text).toBe("TWO");

    const contexts = rows.filter((r) => r.type === "context");
    expect(contexts.map((r) => [r.oldNo, r.newNo])).toEqual([
      [1, 1],
      [3, 3],
    ]);
  });

  it("新增文件的 diff 始终从第 1 行开始", () => {
    const text = [
      "diff --git a/new.txt b/new.txt",
      "new file mode 100644",
      "--- /dev/null",
      "+++ b/new.txt",
      "@@ -0,0 +1,2 @@",
      "+line1",
      "+line2",
    ].join("\n");

    const rows = parseUnifiedDiff(text);
    const adds = rows.filter((r) => r.type === "add");
    expect(adds.map((r) => r.newNo)).toEqual([1, 2]);
  });

  it("处理 hunk 内以 -- 或 ++ 开头的内容行（不误判为文件头）", () => {
    const text = [
      "--- a/a.txt",
      "+++ b/a.txt",
      "@@ -1,2 +1,2 @@",
      "---not a header",
      "+++also content",
    ].join("\n");

    const rows = parseUnifiedDiff(text);
    const del = rows.find((r) => r.type === "del")!;
    expect(del.text).toBe("--not a header");
    const add = rows.find((r) => r.type === "add")!;
    expect(add.text).toBe("++also content");
  });

  it("识别 no newline 标记", () => {
    const text = [
      "--- a/a.txt",
      "+++ b/a.txt",
      "@@ -1 +1 @@",
      "-old",
      "\\ No newline at end of file",
      "+new",
    ].join("\n");
    const rows = parseUnifiedDiff(text);
    expect(rows.some((r) => r.type === "nonewline")).toBe(true);
  });

  it("空 diff 返回空数组", () => {
    expect(parseUnifiedDiff("")).toEqual([]);
  });
});
