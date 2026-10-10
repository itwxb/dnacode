# DNACode

**Minimal Code Editor** · Less IDE. More Flow.

DNACode 是一个专注于「查看代码 → 编辑 → 审查 Git 变更 → AI 总结 → 提交推送」的极简桌面工具。
它不是 VS Code 的替代品，而是刻意删减功能的轻量编辑器：打开项目后，左侧文件树、中间编辑器、右侧 Git 面板同时可见，整个提交流程始终在同一个窗口内完成。

- 桌面框架：Tauri 2（Rust 后端 + 系统 WebView）
- 前端：Vue 3 + TypeScript + Vite
- 编辑器：CodeMirror 6
- Git：直接调用你系统里安装的 Git CLI（沿用你的凭据、SSH 密钥与配置）
- AI（可选）：OpenAI 兼容 Chat Completions API，仅在你主动点击时调用

> 当前版本：**0.1.0**。功能与限制以本文档与实际代码为准。

---

## 界面截图

> 截图占位说明：本仓库暂未附截图（不提供伪造图片）。界面布局如下，可自行运行 `pnpm tauri dev` 查看：
>
> - 顶部工具栏（约 44px）：DNACode 图标与名称、打开项目、当前项目名、当前分支（含 ↑领先/↓落后）、刷新 Git 状态、主题切换、设置。
> - 左侧文件树（默认约 220px，可拖拽调宽）：文件夹展开/折叠、文件类型图标、隐藏文件暗色显示。
> - 中间编辑区：多文件标签页、未保存圆点标记、行号、语法高亮、查找替换；Git 变更文件以标签页形式查看 Diff。
> - 右侧 Git 面板（默认约 340px，可拖拽调宽）：已暂存 / 未暂存 / 未跟踪 / 冲突分区、提交信息输入框与 AI 总结按钮、Commit、Push、提交历史。

---

## 核心功能

### 文件树与项目

- 选择任意本地文件夹作为项目；中文、空格及常见特殊字符路径均可用。
- 按需加载子目录，打开大型项目时不递归扫描全部文件。
- 默认忽略 `.git`、`node_modules`、`dist`、`build`、`coverage`、`target`、`__pycache__`、`.venv`、`.next` 等依赖缓存与构建目录（集中配置于 `src-tauri/src/fs_ops.rs` 的 `IGNORED_DIRS`）。
- 隐藏文件（`.` 开头）以较暗样式显示，而非直接隐藏。
- 最近项目列表由设置文件持久化。

### 代码编辑（CodeMirror 6）

- 行号、语法高亮、当前行高亮、括号匹配、自动缩进、Tab 缩进、查找替换、撤销/重做。
- 多文件标签页、未保存标记（圆点）、`Ctrl+S` 保存、关闭未保存文件时确认、切换项目时保护未保存内容。
- 语言按扩展名**动态加载**：JavaScript/TypeScript/JSX/TSX、Vue 单文件组件、HTML、CSS/SCSS/Less、JSON、Markdown、Python、Rust、Java、PHP、YAML；未知类型回退为纯文本模式。
- 保存安全：写入前比对文件修改时间，外部已修改时给出「覆盖 / 重新加载 / 取消」选择，绝不静默覆盖；文件被外部删除时确认后才重建。
- 超过 5 MB 的文本、二进制文件、非 UTF-8 文件会给出明确提示并拒绝打开。

### Git 可视化（基于真实仓库）

- 状态解析使用 `git status --porcelain=v1 -z`（NUL 分隔、机器可解析），严格区分**已暂存 / 未暂存 / 未跟踪 / 冲突**，识别新增、修改、删除、重命名（含原路径）与复制。
- 单文件 Diff：已暂存差异、未暂存差异区分展示；未跟踪文件基于真实内容生成“相对空文件”的新增差异；二进制文件与超大差异有明确提示，超长 Diff 可手动加载完整内容。
- 暂存/取消暂存：单个或全部；**Commit 严格遵守 Git 暂存区语义** —— 无已暂存内容时禁用提交、绝不自动暂存、失败时保留提交信息。
- 丢弃未暂存修改、删除未跟踪文件属于危险操作：先展示影响范围，确认后执行，且后端只接受处于对应状态的文件。
- Push 是独立按钮，永远不会在 Commit 后自动触发；支持「发布分支并推送」建立上游。认证失败、网络异常、远程拒绝、无远程仓库、缺少提交身份、未安装 Git、分支无上游、detached HEAD 等都有中文说明，并保留 Git 原始输出供排查。
- 分支列表与切换（脏工作区先确认）；提交历史分页加载，可查看每次提交的文件列表与单个文件的真实 Diff。
- 非 Git 仓库中提供一键「初始化 Git 仓库」（仅用户显式触发）。

### AI 修改总结（可选）

- 设置中配置：API Base URL、模型名称、API Key、请求超时、单次请求差异上限；提供「测试连接」。
- API Key 保存在操作系统安全凭据存储（Windows Credential Manager / macOS Keychain / Linux keyutils）。凭据存储不可用时明确报错并**拒绝保存**，不会把 Key 明文写入配置文件或日志。
- 点击「AI 总结本次修改」后：收集真实 Git 差异 → 跳过二进制/过大/冲突文件 → 脱敏疑似密钥、Token、私钥 → **展示将发送的文件清单与规模** → 你确认后才会发起请求（可选择“仅已暂存内容”或“全部工作区变更”）。
- 结果包含修改目的、主要改动、关键文件、风险提示与建议的 Commit Message；建议信息可一键填入输入框，由你最终确认。
- 请求有超时、支持取消；未配置或请求失败不影响任何编辑与 Git 功能。DNACode 不包含遥测，也不存在任何后台自动上传。

---

## 技术栈

| 层 | 技术 |
| --- | --- |
| 桌面框架 | Tauri 2 |
| 后端 | Rust（文件系统、Git CLI、AI 请求、配置与凭据） |
| 前端 | Vue 3 + TypeScript（严格模式）+ Vite |
| 编辑器 | CodeMirror 6（按需动态加载语言包） |
| 样式 | 原生 CSS 与 CSS Variables（无 UI 组件库） |
| Git | 系统 Git CLI（`--porcelain=v1 -z` 等机器可解析输出） |
| AI | OpenAI 兼容 Chat Completions API + 系统安全凭据存储 |
| 测试 | Rust 单测 + 临时仓库集成测试、Vitest、vue-tsc、ESLint、Prettier |
| 发布 | GitHub Actions（CI 三平台） + NSIS 安装包 |

---

## 开发环境要求

- 操作系统：Windows 10/11 x64（第一目标平台；CI 同时验证 macOS 与 Linux 构建）
- Node.js ≥ 20（推荐 22 LTS）与 pnpm ≥ 10
- Rust stable（MSVC 工具链）
- 系统已安装 Git（建议 ≥ 2.23）
- Windows：Visual Studio 2022 Build Tools（“使用 C++ 的桌面开发”或 C++ 生成工具，含 Windows SDK）
- WebView2 Runtime（Windows 11 自带；Windows 10 可能需要安装）

### 依赖安装（Windows 示例）

```powershell
# Rust 工具链（含 rustup）
winget install Rustlang.Rustup

# MSVC 构建工具（需在管理员 PowerShell 中执行）
winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.NativeDesktop --includeRecommended"

# Node.js 与 pnpm
winget install OpenJS.NodeJS.LTS
corepack enable

# 仓库依赖
pnpm install
```

---

## 常用命令

```bash
pnpm install          # 安装前端依赖
pnpm tauri dev        # 本地开发（启动 Tauri 窗口 + Vite）
pnpm tauri build      # 生产构建（含 Windows NSIS 安装包）

pnpm format:check     # Prettier 格式检查
pnpm lint             # ESLint
pnpm typecheck        # vue-tsc 严格类型检查
pnpm test             # 前端单元测试（Vitest）
pnpm build            # 类型检查 + 前端生产构建

cd src-tauri
cargo fmt --all -- --check   # Rust 格式检查
cargo clippy --all-targets   # Rust 静态检查
cargo test                   # Rust 单元测试 + 真实临时仓库集成测试
```

---

## Windows 安装说明

1. 使用 `pnpm tauri build` 构建，产物位于：

   ```
   src-tauri/target/release/bundle/nsis/DNACode_0.1.0_x64-setup.exe
   ```

2. 安装包为 NSIS 格式，默认按当前用户安装（无需管理员权限）。
3. 运行依赖 **WebView2 Runtime**：Windows 11 与大部分 Windows 10 设备已预装；若缺失，安装程序会尝试联网下载引导程序。完全离线环境请先手动安装 WebView2 Runtime。
4. 卸载：通过「设置 → 应用」或安装目录中的卸载程序完成。

---

## AI API 配置

在「设置 → AI」中填写：

| 配置项 | 说明 |
| --- | --- |
| API Base URL | OpenAI 兼容地址，例如 `https://api.openai.com/v1`、`https://api.deepseek.com`、`http://localhost:11434/v1`。DNACode 会自动尝试 `/v1/chat/completions` 与 `/chat/completions` 两种路径。 |
| 模型名称 | 例如 `gpt-4o-mini`、`deepseek-chat` 等。 |
| API Key | 保存到系统安全凭据存储，不会写入 `settings.json`。 |
| 请求超时 | 5–600 秒。 |
| 单次请求差异上限 | 2000–400000 字符，超出部分会截断并在结果中标注。 |

保存后可用「测试连接」验证。请求只在你点击「AI 总结本次修改」并确认发送清单后发出。

---

## Git 常见问题

| 现象 | 处理方式 |
| --- | --- |
| 提示未检测到 Git | 安装 Git 并确保 `git` 在 PATH 中；重新打开应用。 |
| 当前文件夹不是 Git 仓库 | 在 Git 面板点击「初始化 Git 仓库」，或在终端执行 `git init`。 |
| 提交前提示未配置身份 | 终端执行 `git config --global user.name "名字"` 与 `git config --global user.email "邮箱"`。 |
| 没有远程仓库 | 终端执行 `git remote add origin <仓库地址>` 后刷新。 |
| 本地分支没有上游分支 | 点击「发布分支并推送」（等效 `git push -u origin <分支>`）。 |
| 推送认证失败 | 检查系统的凭据助手（Git Credential Manager）或 SSH 密钥；DNACode 不接管凭据，也不会保存你的密码。 |
| 推送被远程拒绝（远程有新提交） | DNACode 首版不含 fetch/pull：请在终端执行 `git pull --rebase`（或 fetch 后合并）再回到应用推送。 |
| Commit 按钮不可用 | 需要：已暂存文件 + 非空提交信息。DNACode 不会自动暂存。 |

Git 的原始错误输出会保留在错误提示的“详情”中，便于排查。

---

## 隐私与安全

- **无账号、无遥测、无行为数据收集**；应用不会连接任何供应商之外的服务器。
- AI 上传内容遵循「用户主动触发 + 发送前预览 + 自动脱敏」；未配置 AI 时全部功能可用。
- API Key 仅存于系统安全凭据存储；日志、配置文件中不出现明文；错误信息不回显 Key。
- Rust 后端不经过 shell 执行任何命令；不暴露任意命令执行能力；文件访问限制在当前项目根目录内并禁止进入 `.git` 内部目录。
- Tauri capabilities 保持最小权限（核心能力 + 打开文件夹对话框）。
- 生产构建启用 CSP（`default-src 'self'`，不加载任何外部脚本/样式）。

---

## 性能设计

- 主窗口先渲染，不等待 Git 扫描、文件遍历或 AI 请求。
- 文件树按需加载；Git 与文件 IO 在后台线程执行，不阻塞界面。
- 提交历史分页加载；不轮询仓库状态（仅在打开项目、手动刷新、窗口聚焦、Git 操作与保存后刷新）。
- 大文件（>5 MB）与大 Diff 均有上限与截断提示；编辑器按需加载语言包。

### 实测数据（单机真实测量，非基准测试）

测试环境：Windows 10 x64（10.0.19045）、Intel Core i5-3470 @ 3.2GHz（4 核）、16 GB 内存，
使用 Release 构建（`pnpm tauri build` 产物 `dnacode.exe`，5.99 MB）。

| 指标 | 实测值 | 说明 |
| --- | --- | --- |
| 启动到主窗口可见 | 193 ms / 109 ms（两次冷启动样本） | 显示欢迎页；窗口不等 Git 扫描、文件遍历或网络 |
| 空闲工作集内存 | 约 16.7 MB | 欢迎页、未打开项目 |
| 打开小型/中型项目、Git 刷新耗时、滚动流畅度 | 待补充 | 需要真实项目上的交互测试；测量完成前不提供估算数字 |

> 以上为单台机器上的样本，仅作参考，不代表所有环境与项目规模。打开大型项目、Git 刷新与滚动
> 体验会在真实项目上补充记录（包括项目规模与硬件说明）。

---

## 当前已知限制

- 不包含 LSP、智能补全、调试器、终端、插件系统。
- 不包含 fetch/pull、冲突可视化编辑器、交互式 rebase、Git 网络图。
- 文件树忽略目录为内置常量，暂不支持在设置中自定义。
- 超过 5 MB 的文本文件、二进制文件、非 UTF-8 文件不能在编辑器内打开（Diff 与 Git 操作不受影响）。
- Vue 单文件组件使用 CodeMirror 官方 `@codemirror/lang-vue` 解析，语法高亮为通用实现，并非完整的 Vue 语言服务。

---

## 目录结构

```
├── src/                    # Vue 前端
│   ├── components/         # 界面组件（文件树 / 编辑器 / Git 面板 / 设置 / 对话框…）
│   ├── stores/             # 轻量响应式状态（无状态管理框架）
│   ├── utils/              # 纯函数工具（Diff 解析、格式化、语言映射）与单测
│   └── api.ts              # Tauri 命令封装与统一错误处理
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── fs_ops.rs       # 文件树与文件读写（路径校验、冲突保护）
│   │   ├── git/            # Git 命令、状态解析、Diff、暂存、提交、推送、分支、历史
│   │   ├── ai.rs           # AI 总结（脱敏、限额、超时、取消）
│   │   ├── config.rs       # 非敏感设置
│   │   ├── credentials.rs  # 系统安全凭据存储
│   │   └── error.rs        # 统一错误模型
│   ├── tests/              # 真实临时 Git 仓库集成测试
│   └── tauri.conf.json     # 窗口、CSP、打包配置
└── .github/workflows/      # CI 与 Release
```

---

## 贡献方式

1. Fork 本仓库并创建分支。
2. 提交前请运行：`pnpm format:check && pnpm lint && pnpm typecheck && pnpm test && pnpm build`，以及 `cd src-tauri && cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test`。
3. 保持功能克制：DNACode 的目标是“少即是多”，新增功能前请先确认它符合产品定位。
4. 通过 Pull Request 提交，并描述改动目的与验证方式。Issue 中请附上操作系统版本与复现步骤。

---

## 许可证

[MIT](LICENSE) © 2026 DNACode contributors

---

## English Overview

DNACode is a minimal desktop code editor built with Tauri 2, Rust, Vue 3, TypeScript and CodeMirror 6. It focuses on a fast open-project → edit → review Git changes → AI summary → commit → push workflow in a single window with a permanent three-pane layout (file tree, editor, Git panel).

Highlights:

- Lazily-loaded project file tree with sensible ignore rules; Chinese/space/special-character paths supported.
- CodeMirror 6 editor: line numbers, syntax highlighting (JS/TS/Vue/HTML/CSS/JSON/Markdown/Python/Rust/Java/PHP/YAML, loaded on demand), multi-tab editing, unsaved markers, Ctrl+S, external-modification conflict protection, size/binary/encoding safeguards.
- Real Git operations via the system Git CLI: machine-parsed porcelain v1 status, staged/unstaged/untracked/conflict sections, real diffs (including synthesized diffs for untracked files), stage/unstage, commit that strictly respects the index, explicit push (with "publish branch" support), branch switching with dirty-worktree confirmation, paginated history, per-commit file diffs.
- Optional AI change summaries via any OpenAI-compatible Chat Completions endpoint. Keys live in the OS credential store; uploads happen only after an explicit preview + confirm; secrets are redacted; timeouts and cancellation supported.
- No telemetry, no accounts, minimal Tauri permissions, CSP enabled in production builds.
- Tests: Rust unit tests, integration tests against real temporary Git repositories, Vitest unit tests, ESLint, Prettier and strict `vue-tsc` type checking; GitHub Actions CI on Windows/macOS/Linux and a tag-driven release workflow producing a Windows NSIS installer.

See the Chinese sections above for full documentation. License: MIT.
