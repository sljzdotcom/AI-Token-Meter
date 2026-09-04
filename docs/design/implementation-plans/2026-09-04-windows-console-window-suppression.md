# Windows 启动空白终端窗口修复实现计划

> **面向 AI 代理的工作者：** 使用 test-driven-development 逐任务实施。步骤使用复选框（`- [ ]`）跟踪进度。

**目标：** 让 Windows AI Token Meter 主程序使用 GUI subsystem，启动时不再创建空白 Windows Terminal，并用实际 PE 产物门禁防止复发。

**架构：** 编译时属性决定主 `.exe` 的 Windows subsystem；PowerShell 门禁直接解析 PE Optional Header。Windows CI 与正式 Release 均检查真实产物，不以源码文本检查代替行为验证。

**技术栈：** Rust、Tauri 2、PE/COFF、PowerShell、GitHub Actions、NSIS。

---

## 文件结构

- 修改 `windows/src-tauri/src/main.rs`：Windows 构建使用 GUI subsystem。
- 创建 `scripts/check-windows-pe-subsystem.ps1`：验证真实 PE 文件为 Windows GUI subsystem。
- 修改 `.github/workflows/windows-ci.yml`：构建 debug Tauri 应用后验证主 `.exe`。
- 修改 `.github/workflows/release.yml`：正式 Windows 构建后重复验证 release `.exe`。
- 更新版本、Release notes、需求台账和开发日志。

### 任务 1：建立实际 PE 失败门禁

- [x] 创建 `scripts/check-windows-pe-subsystem.ps1`，校验 MZ、PE Signature、Optional Header 长度和 `Subsystem == 2`，错误中报告实际 subsystem。
- [x] 在 Windows CI 的 Tauri build 后调用脚本，传入 `windows/src-tauri/target/debug/ai-token-meter-windows.exe`。
- [x] 在 Release workflow 的 Tauri build 后调用脚本，传入 `windows/src-tauri/target/release/ai-token-meter-windows.exe`。
- [ ] 只提交测试和 CI 门禁并推送修复分支，确认现有程序在 Windows runner 因 subsystem `3` 正确失败。

### 任务 2：在编译源头消除控制台窗口

- [ ] 在 `windows/src-tauri/src/main.rs` 顶部增加仅 Windows 生效的 GUI subsystem 声明。
- [ ] 运行格式、Rust/前端定向测试和构建；推送后确认同一 Windows PE 门禁变绿。
- [ ] 确认窗口配置、Provider 子进程和 macOS 源码没有行为变化。
- [ ] 提交修复检查点。

### 任务 3：完整回归与 Preview 发布

- [ ] 同步根、macOS、npm、Cargo、Tauri 版本到 `0.3.0-preview.2`（build 9），补 Changelog、Release notes 和开发日志。
- [ ] 运行 Windows 前端/Rust/rustfmt/Clippy、macOS 完整测试、跨平台合同、文档、Release feed、资产和公开安全检查。
- [ ] 推送 `main` 和 `v0.3.0-preview.2`，等待双平台 Release workflow 全绿。
- [ ] 公开重下两平台资产，核对 Sparkle、Windows minisign、SHA-256、Preview feed 和稳定 appcast 隔离。
- [ ] 在需求台账保留 Windows 真机启动验收为 `待用户确认`，不以 CI 代替真实窗口确认。
