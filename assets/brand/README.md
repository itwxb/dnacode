# 品牌资源（Brand Assets）

本目录是 DNACode 的全部品牌资产，随仓库公开（MIT 许可，见根目录 [LICENSE](../../LICENSE)）。

## 目录结构

- **根目录（品牌套件）**：图标 SVG 母版（`DNACode-icon-master.svg`）、16–1024px PNG 导出、16/24/32px 艺术指导小尺寸、32px 专用版、浅色（`-light`）/ 单色（`-monochrome`）变体、字标（`DNACode-wordmark.svg`）；完整设计规范见 [BRAND-GUIDE.md](BRAND-GUIDE.md)。
- **[review/](review/)**：历次设计评审文档（发给设计师的单文件版本，图片已内嵌，浏览器直接打开）：
  - `original-icon-review.html` —— 初版图标评审稿；
  - `refined-icon-review.html` / `.md` —— 精修版复审与补件说明。
- **[verification/](verification/)**：工程验证证据（16px 可读性逐像素对比、真实构建产物提取、打包帧字节级比对、NSIS 安装器图标验证等）；结论见 [VERIFICATION-REPORT.md](verification/VERIFICATION-REPORT.md)。

## 图标再生流程

```bash
pnpm icons
```

等价于 `tauri icon`（从 `DNACode-icon-1024.png` 生成全平台图标）+ `scripts/compose-icons.mjs`
（把艺术指导 16/24/32px 小尺寸装回 `.ico` / `.icns`，避免单一源转换抹平小尺寸优化）。
细节与注意事项见脚本头部注释；本地增量构建后若怀疑 exe 图标未更新，见 `src-tauri/build.rs` 中的说明。
