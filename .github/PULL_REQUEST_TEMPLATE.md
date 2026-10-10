## 改动说明

<!-- 这次改动解决什么问题？为什么这样做？ -->

关联 Issue：#

## 改动类型

- [ ] 缺陷修复
- [ ] 新功能
- [ ] 重构 / 性能
- [ ] 文档
- [ ] 构建 / CI

## 验证方式

<!-- 你如何验证这次改动？涉及 Git 行为的请说明使用的测试场景 -->

## 提交前检查（必须实际执行）

- [ ] `pnpm format:check && pnpm lint && pnpm typecheck && pnpm test`
- [ ] `cd src-tauri && cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test`
- [ ] 涉及界面改动的，已在 `pnpm tauri dev` 下实际运行验证
- [ ] 未在代码、日志或测试夹具中引入真实密钥
