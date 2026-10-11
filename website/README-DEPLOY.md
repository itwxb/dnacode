# DNACode 官网（中英双语静态站点）

本目录是 dnacode.cn 的纯静态站点，位于主仓库的 `website/` 下，与桌面应用代码互不影响（Tauri 打包只读取 `dist/`）。

站点没有任何前端框架、第三方脚本、外部字体、运行时 API 或图片 CDN 请求：首页仅 1 个网络请求（HTML 本身），其余全部内联或为本地文件。

## 文件

- `index.html`：中文默认首页，含完整页面内容、内联 CSS/JS、结构化数据和中英切换。
- `en/index.html`：独立英文页面，英文内容直接存在于 HTML 源码，便于搜索引擎索引。
- `favicon.svg` / `favicon.ico` / `apple-touch-icon.png`：站点图标，取自 `assets/brand/` 精修版品牌资产；`favicon.ico` 内含 16/24/32/48 全套艺术指导尺寸。
- `og-image.svg`：社交分享图源文件；`og-image.png`（1200×630）为实际使用版本，由 Chrome 无头模式导出：
  `chrome --headless=new --hide-scrollbars --force-device-scale-factor=1 --window-size=1200,630 --screenshot=og-image.png og-image.svg`
- 工作流区块的应用界面为纯 HTML/CSS 绘制（与首屏示意图同一套做法），不依赖任何图片文件，界面如调整可直接修改 `index.html` / `en/index.html` 中对应标记与样式。
- 页面「实测数据」条（安装包 2.4 MB / 安装后 ≈6 MB / 冷启动 0.1-0.2 s / 空闲内存 16.7 MB，含每项说明行）来自主仓库 README 的实测段落；数据或说明更新时请同步修改两页的 stats 区块与三处翻译映射（stat_1..4、stat_1_desc..4_desc、stat_note）。
- `404.html`：Vercel / EdgeOne 等平台自动将其用于未匹配的路径。
- `robots.txt` / `sitemap.xml`：搜索引擎抓取配置。
- `vercel.json`：Vercel 路径规则与安全响应头（含 CSP、HSTS）。
- `edgeone.json`：EdgeOne Pages 的安全响应头与静态资源缓存配置（与 `vercel.json` 等效，EdgeOne 会读取根目录下的此文件）。

## 本地预览

直接双击 `index.html` 打开即可（全站资源均为相对路径、零外部依赖，`file://` 下也能完整显示，包括图标等全部资源）；也可以在本目录执行 `npx serve .` 起本地服务。

## 部署

### Vercel

1. 推送仓库到 GitHub 后，在 Vercel 新建项目并导入该仓库。
2. Framework Preset 选择 **Other**，Build Command 留空，**Root Directory 选择 `website`**，Output Directory 保持 `website`（或留空）。
3. 部署成功后，在 Project → Settings → Domains 添加 `dnacode.cn`，按平台提示配置 DNS 与 HTTPS。

也可在本目录执行 `npx vercel --prod` 直接部署。

### EdgeOne Pages（当前部署方式）

1. 连接 Git 仓库后按以下填写：框架预设 **Other**，根目录 **`./website`**，输出目录填 **`.`**（站点本身就是产物，没有构建步骤）；**构建命令、安装命令全部留空**。
2. `vercel.json` 在 EdgeOne 上不生效；本目录的 `edgeone.json` 已配置等效的安全响应头与静态资源缓存。
3. 加速区域：未备案域名请选「全球可用区（不含中国大陆）」；`dnacode.cn` 若已完成 ICP 备案，选「全球可用区（含中国大陆）」可获得大陆直连加速。
4. 部署后在站点域名设置里添加 `dnacode.cn`，按控制台提示配置 DNS 并启用 HTTPS。

## 上线后检查

- `https://dnacode.cn/robots.txt`、`/sitemap.xml`、`/en/`、`/favicon.ico`、`/og-image.png` 均可正常访问。
- 在 Google Search Console、Bing Webmaster Tools 提交 sitemap；面向国内搜索引擎时按各站长平台要求提交。
- 将首页链接分享到任意支持 Open Graph 的平台，确认预览图显示为新的 1200×630 卡片。
- 页面主 CTA 均指向 `github.com/itwxb/dnacode/releases/latest`，发布新版本后无需修改页面。

## SEO / 内容注意事项

页面已设置标题、描述、canonical、hreflang、Open Graph（PNG 分享图 + 尺寸声明）、Twitter Cards、SoftwareApplication JSON-LD、robots 与 sitemap。技术 SEO 不保证排名：后续需持续维护真实版本更新、安装指南、截图、FAQ 与双语文案，并在各站长工具中确认收录情况。
