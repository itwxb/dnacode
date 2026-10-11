#!/usr/bin/env node
/**
 * DNACode 字标转曲生成脚本（可复现）
 *
 * 生成（写入 assets/brand/）：
 *   - DNACode-wordmark-outlined.svg            深色字标（文字转曲，用于深色背景）
 *   - DNACode-wordmark-light.svg               浅色字标（浅色图标 + 深色文字，用于浅色背景）
 *   - DNACode-icon-monochrome-transparent.svg  透明底单色图形（白标，可改色）
 *
 * 用法（在一个能联网安装依赖的目录里）：
 *   npm i opentype.js @expo-google-fonts/inter
 *   node <本脚本路径> [可选字体路径]
 *
 * 默认字体：./node_modules/@expo-google-fonts/inter/600SemiBold/Inter_600SemiBold.ttf
 * 许可说明：Inter 为 SIL OFL 开源字体，转曲结果可随 MIT 项目自由分发。
 */
import opentype from "opentype.js";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve, dirname } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const outDir = resolve(here, "..", "assets", "brand");
const fontPath =
  process.argv[2] ?? "./node_modules/@expo-google-fonts/inter/600SemiBold/Inter_600SemiBold.ttf";

const fontBuf = readFileSync(fontPath);
const font = opentype.parse(fontBuf.buffer.slice(fontBuf.byteOffset, fontBuf.byteOffset + fontBuf.byteLength));

// ---- 1. “DNACode” 文字转曲（size 72，字距 -2，与原字标一致）----
const text = "DNACode";
const size = 72;
const letterSpacing = -2;
const scale = size / font.unitsPerEm;
let x = 0;
let prev = null;
const parts = [];
for (const ch of text) {
  const glyph = font.charToGlyph(ch);
  if (prev) x += font.getKerningValue(prev, glyph) * scale;
  parts.push(glyph.getPath(x, 0, size).toPathData(2));
  x += glyph.advanceWidth * scale + letterSpacing;
  prev = glyph;
}
const textPathData = parts.join(" ");
const textWidth = x - letterSpacing;
console.log(`text outline: glyphs=${text.length} width=${textWidth.toFixed(1)}`);
console.log(`path data 长度: ${textPathData.length} 字符`);

// ---- 2. 图标组（母版几何，深/浅两套颜色）----
const ICON_SCALE = 0.17578125;
const iconGroup = (tileFill, tileStroke, rear, front) => `  <g transform="translate(0,0) scale(${ICON_SCALE})">
    <rect x="1" y="1" width="1022" height="1022" rx="180" fill="${tileFill}" stroke="${tileStroke}" stroke-width="2"/>
    <path d="M704 188 C704 404 320 620 320 836" fill="none" stroke="${rear}" stroke-width="68" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="M370.5 344 H653.5 M370.5 680 H653.5" fill="none" stroke="${front}" stroke-width="42" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="M320 188 C320 404 704 620 704 836" fill="none" stroke="${front}" stroke-width="68" stroke-linecap="round" stroke-linejoin="round"/>
  </g>`;

function wordmarkSvg(title, desc, iconGroupSvg, textFill) {
  return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="760" height="180" viewBox="0 0 760 180" role="img" aria-labelledby="title desc">
  <title id="title">${title}</title>
  <desc id="desc">${desc}</desc>
  <defs>
    <linearGradient id="background" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#1B2230"/>
      <stop offset="1" stop-color="#10151E"/>
    </linearGradient>
  </defs>
${iconGroupSvg}
  <!-- 文字已转曲（Inter SemiBold，SIL OFL） -->
  <g transform="translate(218,111)">
    <path d="${textPathData}" fill="${textFill}"/>
  </g>
</svg>
`;
}

writeFileSync(
  resolve(outDir, "DNACode-wordmark-outlined.svg"),
  wordmarkSvg(
    "DNACode wordmark (outlined)",
    "DNACode wordmark with outlined text and the dark app icon, for dark surfaces.",
    iconGroup(`url(#background)`, "#2B3547", "#315FA8", "#4C8DFF"),
    "#E6E9F0",
  ),
);

writeFileSync(
  resolve(outDir, "DNACode-wordmark-light.svg"),
  wordmarkSvg(
    "DNACode wordmark (light, outlined)",
    "DNACode wordmark with outlined text and the light app icon, for light surfaces.",
    iconGroup("#F4F7FC", "#D8E1EF", "#9AB9EB", "#2563EB"),
    "#1F2328",
  ),
);

// ---- 3. 透明底单色图形（去掉底板，保留图形）----
writeFileSync(
  resolve(outDir, "DNACode-icon-monochrome-transparent.svg"),
  `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024" role="img" aria-labelledby="title desc">
  <title id="title">DNACode monochrome mark (transparent background)</title>
  <desc id="desc">Single-colour DNACode DNA mark on a transparent background, for monochrome or single-colour usage.</desc>
  <path d="M704 188 C704 404 320 620 320 836" fill="none" stroke="#FFFFFF" stroke-width="68" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M370.5 344 H653.5 M370.5 680 H653.5" fill="none" stroke="#FFFFFF" stroke-width="42" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M320 188 C320 404 704 620 704 836" fill="none" stroke="#FFFFFF" stroke-width="68" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
`,
);

console.log("已生成 3 个 SVG：");
console.log("  DNACode-wordmark-outlined.svg");
console.log("  DNACode-wordmark-light.svg");
console.log("  DNACode-icon-monochrome-transparent.svg");
