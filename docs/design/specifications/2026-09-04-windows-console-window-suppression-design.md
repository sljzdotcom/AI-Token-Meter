# Windows 启动空白终端窗口修复设计

**需求：** `REQ-20260904-005`

**日期：** 2026-09-04

**状态：** 已确认（非视觉缺陷按用户既定授权采用推荐方案）

## 问题与根因

Windows `0.3.0-preview.1` 真机启动后，浮动条正常显示，但同时出现一个标题为 “AI Token Meter” 的巨大黑色窗口。截图顶部的终端标签、加号和下拉入口证明它是 Windows Terminal，不是 Edge、WebView2 或 Provider 详情窗口。

发布入口 `windows/src-tauri/src/main.rs` 没有声明 Windows GUI subsystem。Rust 因而把主程序构建为 console subsystem；从资源管理器、开始菜单或登录启动项运行时，Windows 会为它创建控制台宿主。启动后的 Tauri 浮动条仍正常工作，所以形成“浮动条 + 空白终端”并存的现象。

## 方案比较

1. **编译时固定 GUI subsystem（采用）：** 在 Windows 主入口增加 `windows_subsystem = "windows"`，从 PE 层阻止系统分配控制台；用实际构建产物的 PE Optional Header 验证 subsystem 为 `2`。
2. **启动后查找并隐藏终端：** 会先闪现窗口，依赖终端宿主实现，也可能误伤用户终端，不采用。
3. **延迟或懒加载 Tauri WebView：** 可以减少 WebView2 初始化，但不能改变主 `.exe` 的 console subsystem，与截图根因不符，不采用。

## 设计

- Windows 主入口对所有 Windows 构建声明 GUI subsystem；macOS 和其他目标不应用该属性。
- 不调用 `FreeConsole`、不查找 Windows Terminal 窗口，也不改变 Claude Code、OpenAI Codex 或 DeepSeek 子进程边界。
- 创建 `scripts/check-windows-pe-subsystem.ps1`，直接读取实际 `.exe` 的 DOS Header、PE Signature 和 Optional Header，断言 `Subsystem == 2 (Windows GUI)`。无效 PE、截断文件或 console subsystem 均明确失败。
- Windows CI 在 Tauri debug installer 构建后检查主程序 PE Header；Release workflow 在 release installer 构建后再次检查。这样 PR/`main` 与公开发布两条路径都无法产出带空终端的安装包。
- 同步 macOS/Windows 版本为 `0.3.0-preview.2`（build 9）。macOS 只同步版本与发布文档，不改变窗口、浮动条或采集代码。

## 测试与验收

- 红灯：先加入 PE 检查脚本和 Windows CI 步骤；现有 `main.rs` 构建出的 `.exe` 为 console subsystem，Windows runner 必须失败并报告实际值 `3`。
- 绿灯：加入 GUI subsystem 声明后，同一实际产物检查通过，完整 Windows 前端、Rust、Clippy、NSIS 构建继续通过。
- macOS 运行完整 Swift/文档/公开安全门禁，证明仅版本同步。
- 发布后公开重下并验证 Windows minisign、SHA-256、Preview feed 和 macOS Sparkle；稳定 macOS appcast 不写入 Preview。
- 真机验收：启动 `0.3.0-preview.2` 后只出现浮动条和系统托盘，不出现 Windows Terminal；详情与 Settings 仍按需打开。

## 非目标

- 不修改浮动条轮廓、配色或位置策略。
- 不修改 Provider 数据、凭据、CLI 认证或 DeepSeek 官网窗口。
- 不隐藏用户自己打开的终端，也不依赖窗口标题进行运行时修补。
