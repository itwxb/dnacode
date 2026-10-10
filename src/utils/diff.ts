/** 统一 diff 文本解析（纯函数，便于单元测试与渲染复用）。 */

export interface DiffRow {
  type: "hunk" | "add" | "del" | "context" | "meta" | "nonewline";
  oldNo: number | null;
  newNo: number | null;
  text: string;
}

const HUNK_RE = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/;

/**
 * 解析 git 生成的标准统一 Diff（unified diff）。
 * 返回可逐行渲染的行数组：新增、删除、上下文、hunk 头、文件元信息。
 */
export function parseUnifiedDiff(text: string): DiffRow[] {
  const rows: DiffRow[] = [];
  const lines = text.split("\n");
  if (lines.length > 0 && lines[lines.length - 1] === "") {
    lines.pop();
  }

  let oldNo = 0;
  let newNo = 0;
  let inHunk = false;

  for (const line of lines) {
    const hunk = HUNK_RE.exec(line);
    if (hunk) {
      oldNo = Number.parseInt(hunk[1], 10);
      newNo = Number.parseInt(hunk[2], 10);
      inHunk = true;
      rows.push({ type: "hunk", oldNo: null, newNo: null, text: line });
      continue;
    }

    if (inHunk) {
      if (line.startsWith("\\")) {
        rows.push({ type: "nonewline", oldNo: null, newNo: null, text: line });
      } else if (line.startsWith("+")) {
        rows.push({ type: "add", oldNo: null, newNo, text: line.slice(1) });
        newNo += 1;
      } else if (line.startsWith("-")) {
        rows.push({ type: "del", oldNo, newNo: null, text: line.slice(1) });
        oldNo += 1;
      } else if (line.startsWith(" ")) {
        rows.push({ type: "context", oldNo, newNo, text: line.slice(1) });
        oldNo += 1;
        newNo += 1;
      } else if (line === "") {
        rows.push({ type: "context", oldNo, newNo, text: "" });
        oldNo += 1;
        newNo += 1;
      } else {
        rows.push({ type: "meta", oldNo: null, newNo: null, text: line });
      }
      continue;
    }

    rows.push({ type: "meta", oldNo: null, newNo: null, text: line });
  }

  return rows;
}
