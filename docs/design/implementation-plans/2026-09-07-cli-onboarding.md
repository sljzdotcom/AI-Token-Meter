# CLI 安装与账户引导实现计划

> 面向 AI 工作者：按 subagent-driven-development 完成实现、任务审查与最终整分支审查。用户已授权推荐路径，除重大分歧不重复询问。

**目标：** 双平台 Settings 按真实安装/登录状态给出可恢复操作，Windows 页签增加清晰图标。
**架构：** 复用现有 locator/account reader/login launcher；新增固定官方安装脚本构建/启动边界和有界状态协调，不重做认证。
**技术栈：** Swift/SwiftUI、Rust/Tauri、React/TypeScript。

## 全局约束

用户点击才安装/登录；官方固定 HTTPS 地址，当前用户身份，不自动升级已发现 CLI。保留账号，已连接显示重新登录，不执行 logout。Windows 显式 WSL/自定义路径缺失不偷偷安装 Native。Settings 字体始终系统默认。安装启动不等于安装成功，不收集原始凭据/安装器输出。测试不得实际安装第三方 CLI。

### Task 1: 双平台 CLI 引导

文件：新增 Sources/AIMeterCore/Accounts/CLIInstallationScriptBuilder.swift、Sources/AIMeterApp/System/CLIInstallationLauncher.swift；修改 AppModel.swift、ServicesSettingsView.swift；Windows accounts 下新增安装模块，修改 accounts/windows_service.rs、lib.rs 命令入口、前端 Shell.tsx、settings/SettingsWindow.tsx 和 i18n 文案。对应 Swift Core/App 测试与 Windows Rust/React 测试由实现者按上述模块新增。

- [x] 先写红灯：未安装主操作可安装而非禁用/仅开教程；signInRequired 强调、connected 重新登录不强调、checking/操作中禁用、unavailable 不误报缺失；安装发现不自动登录。
  ```swift
  #expect(model.serviceAction(for: .claude).title == "Install CLI")
  #expect(model.serviceAction(for: .claude).needsAttention)
  ```
  API 可按现有结构命名，断言真实状态推导和用户可观察行为，不只 grep UI 源码。
- [x] 运行聚焦测试确认预期失败，再最小实现状态/launcher 边界。安装脚本只接受 Provider 枚举，固定来源为 Claude 的 https://claude.ai/install.sh / install.ps1 与 Codex 的 https://chatgpt.com/codex/install.sh / install.ps1；下载完整成功后由 bash/sh/PowerShell 执行，用户可见终端，不提权。
- [x] 通过可注入 launcher / account reader 测试重复点击、已安装重查、启动失败、轮询停止/超时与后续重试；用本地假下载器/脚本运行实际生成脚本验证下载失败不执行、成功退出结果，不访问真实安装端点。
- [x] 对 Windows 后端单独验证 Native/WSL/指定路径的执行边界与新命令注册；原 locator 覆盖官方安装位置，缺少位置时最小补齐并测重新发现。
- [x] 跑 Swift 聚焦→完整 scripts/test.sh；Windows npm test、test:density、build、cargo fmt/clippy/test。共享 Swift 缓存不可与其他 Swift 构建并行。保存 RED/GREEN 命令、输出与环境边界到任务报告，自审并提交仅本任务文件。

### Task 2: Windows 页签图标

文件：windows/src/settings/SettingsWindow.tsx，独立 SettingsTabIcon.tsx（如需要）、windows/src/styles.css 及 React 设置测试。

- [x] 红灯：渲染中英文四个页签，测试对应文字旁可见 SVG、aria-hidden；点击/键盘切换仍显示正确 panel。
  ```tsx
  expect(screen.getByRole('tab', { name: 'Appearance' }).querySelector('svg')).toHaveAttribute('aria-hidden', 'true')
  ```
  若现有结构不是 tab role，遵循正确语义而非更改角色来迎合断言。
- [x] 最小实现 16px currentColor 线性调色板/活动/连接/信息 SVG，保留文字和系统字体，不增加外部依赖或网络请求。
- [ ] 运行 npm test、test:density、build，检查两种语言不会因图标撑开/截断页签；记录证据并提交。勿覆盖 Task 1 设置改动。

### Task 3: 集成验证与记录

- [ ] 独立任务审查和整分支审查；修复重要发现。
- [ ] 更新 docs/user-guide/settings.md、README 未发布说明、CHANGELOG、docs/development/2026-09-07-cli-onboarding.md、docs/requirements-backlog.md 和索引；记录测试与提交证据。
- [ ] scripts/check-docs.sh、scripts/test.sh、Windows 完整门禁与公开安全扫描；PR 原生 CI 后合并 main。本轮不自动发布，旧 0.4.0 安装包保持不变。
