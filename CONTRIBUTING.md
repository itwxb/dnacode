# 贡献指南

感谢参与 DNACode！提交贡献前，请先了解以下约定。

## 产品原则

DNACode 刻意保持克制，只服务一条核心流程：

> 打开项目 → 查看/编辑代码 → 审查 Git 变更 → （可选）AI 总结 → 提交 / 推送

新增功能应当让这条流程更顺，而不是让它变复杂。较大改动请先开 Issue 讨论。

## 开发环境

- Windows 10/11 x64（第一目标平台；macOS / Linux 构建由 CI 验证）
- Node.js ≥ 20 与 pnpm
- Rust stable（MSVC 工具链）；Windows 需要 Visual Studio 2022 Build Tools（C++ 桌面开发）
- 系统 Git ≥ 2.23
- WebView2 Runtime（Windows 11 自带；Windows 10 可能需安装）

## 本地开发

```bash
pnpm install
pnpm tauri dev
```

## 提交前必须通过的检查

```bash
pnpm format:check && pnpm lint && pnpm typecheck && pnpm test && pnpm build

cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## 代码与提交约定

- **不要引入真实密钥**（代码、日志、测试夹具均不允许；测试脱敏逻辑时请使用 `AKIAIOSFODNN7EXAMPLE` 这类官方示例占位）
- Rust 侧新增逻辑需要单元测试；涉及 Git 行为的改动请在 `src-tauri/tests/git_integration.rs` 中增加基于临时仓库的集成测试（绝不操作用户真实项目）
- 前端保持 TypeScript 严格模式；样式使用现有 CSS 变量体系，不引入 UI 组件库或全局状态框架
- 提交信息使用祈使句短句（如 `fix: 修复 …`、`feat: 增加 …`），一个提交只做一件事
- 不提交生成物（`dist/`、`target/`、`node_modules/`，已在 .gitignore 中）

## Pull Request 流程

1. Fork 并创建特性分支
2. 完成改动并通过上述全部检查
3. 提交 PR，描述改动目的与验证方式；界面改动请附截图或录屏
4. 等待 CI 全部通过后请求评审

## 问题反馈

- 缺陷与功能建议请使用仓库的 Issue 模板
- 安全漏洞请遵循 [SECURITY.md](SECURITY.md) 的私密报告流程
