#!/usr/bin/env node
/**
 * 从 CHANGELOG.md 提取指定版本的发布说明。
 * 供 Release 工作流使用：让 GitHub Release 页面直接展示变更内容，
 * 而不是只写一句“详见 CHANGELOG.md”。
 *
 * 用法：node scripts/changelog-section.mjs v0.1.1
 */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const tag = String(process.argv[2] ?? "").trim();
const version = tag.replace(/^v/, "");
const changelogPath = fileURLToPath(new URL("../CHANGELOG.md", import.meta.url));
const lines = readFileSync(changelogPath, "utf8").split(/\r?\n/);

const startIndex = lines.findIndex((line) => line.startsWith(`## [${version}]`));
if (startIndex === -1) {
  console.log(`DNACode ${tag || version}`);
  console.log("");
  console.log("详细变更见仓库中的 CHANGELOG.md。");
  process.exit(0);
}

let endIndex = lines.findIndex((line, i) => i > startIndex && line.startsWith("## "));
if (endIndex === -1) endIndex = lines.length;

// 去掉版本标题行本身（Release 页面已显示版本号），保留其内容
const body = lines
  .slice(startIndex + 1, endIndex)
  .join("\n")
  .trim();
console.log(body);
