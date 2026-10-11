#!/usr/bin/env node
/**
 * DNACode 品牌图标装配脚本
 *
 * 背景：`pnpm tauri icon` 从单一 1024px 源图生成全部平台图标，会把设计师为
 * 16/24/32px 专门制作的艺术指导小尺寸"抹平"（实测：单源转换后 16px 横档结构消失）。
 * 本脚本在 `tauri icon` 之后运行，把 assets/brand/ 中的艺术指导/品牌导出文件
 * 装配进 Windows .ico 与 macOS .icns，并覆盖 Linux/通用 PNG。
 *
 * 用法（已封装为 `pnpm icons`）：
 *   pnpm tauri icon "assets/brand/DNACode-icon-1024.png" -o src-tauri/icons
 *   node scripts/compose-icons.mjs
 *
 * 说明：
 * - .ico 手工装配为 6 帧 PNG 帧（16/24/32 艺术指导 + 48/64/256 母版导出），
 *   与 tauri icon 原帧集合一致（zjw 验证过字节级一致性与 Win32 加载）。
 * - .icns 替换 8 个现代 PNG 块（ic07-ic14），保留 tauri icon 生成的传统块
 *   （is32/il32/s8mk/l8mk，macOS 10.7+ 不再使用）。
 * - 脚本结尾打印帧/块清单供人工核对。
 */
import { readFileSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const brand = (name) => resolve(root, "assets", "brand", name);
const icons = (name) => resolve(root, "src-tauri", "icons", name);

const sha1 = (buf) => createHash("sha1").update(buf).digest("hex");
const read = (name) => {
  const buf = readFileSync(brand(name));
  if (buf[0] !== 0x89 || buf[1] !== 0x50 || buf[2] !== 0x4e || buf[3] !== 0x47) {
    throw new Error(`assets/brand/${name} 不是 PNG 文件`);
  }
  return buf;
};

// ---------- 1. Windows .ico（与 tauri icon 相同的帧集合，但小尺寸换为艺术指导版） ----------
const icoFrames = [
  ["DNACode-icon-16.png", 16], // 艺术指导
  ["DNACode-icon-24.png", 24], // 艺术指导
  ["DNACode-icon-32.png", 32], // 艺术指导
  ["DNACode-icon-48.png", 48], // 母版导出
  ["DNACode-icon-64.png", 64], // 母版导出
  ["DNACode-icon-256.png", 256], // 母版导出
].map(([file, size]) => ({ size, png: read(file) }));

{
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(icoFrames.length, 4);

  const dir = Buffer.alloc(16 * icoFrames.length);
  let offset = 6 + 16 * icoFrames.length;
  icoFrames.forEach((frame, i) => {
    const o = i * 16;
    const dim = frame.size >= 256 ? 0 : frame.size;
    dir.writeUInt8(dim, o); // 宽（256 记 0）
    dir.writeUInt8(dim, o + 1); // 高
    dir.writeUInt8(0, o + 2); // 调色板色数
    dir.writeUInt8(0, o + 3);
    dir.writeUInt16LE(1, o + 4); // 色彩平面
    dir.writeUInt16LE(32, o + 6); // 位深
    dir.writeUInt32LE(frame.png.length, o + 8);
    dir.writeUInt32LE(offset, o + 12);
    offset += frame.png.length;
  });

  writeFileSync(icons("icon.ico"), Buffer.concat([header, dir, ...icoFrames.map((f) => f.png)]));
}

// ---------- 2. macOS .icns（替换现代 PNG 块） ----------
const icnsTypeMap = {
  ic11: "DNACode-icon-32.png", // 16pt @2x —— 艺术指导
  ic12: "DNACode-icon-64.png",
  ic07: "DNACode-icon-128.png",
  ic08: "DNACode-icon-256.png",
  ic13: "DNACode-icon-256.png",
  ic09: "DNACode-icon-512.png",
  ic14: "DNACode-icon-512.png",
  ic10: "DNACode-icon-1024.png",
};

{
  const src = readFileSync(icons("icon.icns"));
  if (src.toString("ascii", 0, 4) !== "icns") {
    throw new Error("src-tauri/icons/icon.icns 不是有效 icns（请先运行 pnpm tauri icon）");
  }

  const chunks = [];
  let pos = 8;
  while (pos + 8 <= src.length) {
    const type = src.toString("ascii", pos, pos + 4);
    const len = src.readUInt32BE(pos + 4);
    if (len < 8 || pos + len > src.length) {
      throw new Error(`icns 块结构异常：${type} len=${len}`);
    }
    chunks.push({ type, data: src.subarray(pos + 8, pos + len) });
    pos += len;
  }

  let replaced = 0;
  for (const chunk of chunks) {
    const file = icnsTypeMap[chunk.type];
    if (!file) continue;
    chunk.data = read(file);
    replaced++;
  }
  const expected = Object.keys(icnsTypeMap).length;
  if (replaced !== expected) {
    throw new Error(
      `期望替换 ${expected} 个 PNG 块，实际替换 ${replaced} 个（icns 结构可能已变化）`,
    );
  }

  const parts = [];
  for (const chunk of chunks) {
    const head = Buffer.alloc(8);
    head.write(chunk.type, 0, 4, "ascii");
    head.writeUInt32BE(8 + chunk.data.length, 4);
    parts.push(head, chunk.data);
  }
  const body = Buffer.concat(parts);
  const out = Buffer.alloc(8 + body.length);
  out.write("icns", 0, 4, "ascii");
  out.writeUInt32BE(8 + body.length, 4);
  body.copy(out, 8);
  writeFileSync(icons("icon.icns"), out);
}

// ---------- 3. Linux / 通用 PNG ----------
const pngMap = {
  "32x32.png": "DNACode-icon-32.png", // 艺术指导（Linux 桌面小尺寸）
  "64x64.png": "DNACode-icon-64.png",
  "128x128.png": "DNACode-icon-128.png",
  "128x128@2x.png": "DNACode-icon-256.png",
  "icon.png": "DNACode-icon-512.png",
};
for (const [dst, srcName] of Object.entries(pngMap)) {
  writeFileSync(icons(dst), read(srcName));
}

// ---------- 4. 输出核对清单 ----------
console.log("== icon.ico 帧 ==");
{
  const buf = readFileSync(icons("icon.ico"));
  const count = buf.readUInt16LE(4);
  for (let i = 0; i < count; i++) {
    const o = 6 + 16 * i;
    const size = buf[o] === 0 ? 256 : buf[o];
    const len = buf.readUInt32LE(o + 8);
    const off = buf.readUInt32LE(o + 12);
    const isPng = buf[off] === 0x89 && buf[off + 1] === 0x50;
    const same = sha1(buf.subarray(off, off + len)) === sha1(icoFrames[i].png);
    console.log(
      `  ${String(size).padStart(3)}px  ${String(len).padStart(6)} 字节  ${isPng ? "PNG" : "BMP"}  与源一致=${same}`,
    );
  }
}
console.log("== icon.icns 块 ==");
{
  const buf = readFileSync(icons("icon.icns"));
  let pos = 8;
  while (pos + 8 <= buf.length) {
    const type = buf.toString("ascii", pos, pos + 4);
    const len = buf.readUInt32BE(pos + 4);
    console.log(`  ${type}  ${len} 字节${icnsTypeMap[type] ? "  ← " + icnsTypeMap[type] : ""}`);
    pos += len;
  }
}
console.log("装配完成 ✔");
