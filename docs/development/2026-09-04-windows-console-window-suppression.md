# Windows 启动空白终端窗口修复记录

## 背景与目标

对应 `REQ-20260904-005`。用户在 Windows 11 启动 `0.3.0-preview.1` 后，桌面浮动条可以正常出现，但系统同时自动打开一个标题为 “AI Token Meter” 的巨大空白 Windows Terminal。该窗口从程序启动时即出现，并非点击 Provider 详情产生。

目标是让 Windows 主程序在进程创建阶段就不分配控制台，同时建立基于真实 `.exe` 的发布门禁，防止后续重构再次引入启动黑窗。

## 问题证据与根因

- 真机截图中的顶部标签、加号和下拉入口属于 Windows Terminal，不是 Edge 或 Tauri 详情 WebView。
- `windows/src-tauri/src/main.rs` 原先没有 Windows subsystem 声明；Rust 在 Windows 上因此生成 PE subsystem `3`（Windows Console）。
- Tauri 浮动条和托盘仍能运行，是因为控制台窗口只是主进程额外分配的宿主，并不代表网页详情渲染失败。
- Windows workflow `33830008532` 先完成前端测试、production build、rustfmt、严格 Clippy、完整 Rust 测试和 NSIS 构建；随后新增门禁解析真实 debug 主程序，读到 subsystem `3` 并按预期失败。这把根因与普通 UI/Provider 行为明确分离。

## 实现与关键决定

- 在 Windows Rust 主入口声明 `#![cfg_attr(windows, windows_subsystem = "windows")]`，让 debug 与 release 的 Windows 主程序都使用 PE subsystem `2`（Windows GUI）。
- 不在启动后查找、闪烁再隐藏 Terminal；问题在编译期消除，避免误关用户终端或留下可见闪烁。
- 不修改 Tauri meter/detail/settings 窗口配置、Provider 子进程、macOS AppKit 代码或 macOS 浮动条视觉。
- 新增 `scripts/check-windows-pe-subsystem.ps1`，直接验证 MZ、PE Signature、Optional Header 及 Subsystem 字段。Windows CI 与正式 Release 都在构建真实主程序后执行同一检查。

## 自动化验证

- TDD 红灯：workflow `33830008532` 的 PE 门禁报告 `actual subsystem is 3`；其前置测试和 NSIS 构建全部通过。
- 本机修复后：Rust 完整测试通过（含 Windows 平台策略和 DeepSeek 回环测试），`cargo fmt --check` 与 `cargo clippy --locked --all-targets -- -D warnings` 通过。
- 修复绿灯：Windows workflow `33831023542` 的完整构建通过，PE 门禁报告 `Windows PE subsystem check passed (GUI = 2)`；macOS workflow `33831023523` 同时通过。
- 正式发布 workflow 与公开资产复验将在 `0.3.0-preview.2` 发布后补入，不以未完成的发布冒充通过。

## 安全、隐私与兼容边界

本次不读取或修改 Claude Code、OpenAI Codex、DeepSeek 的凭证、账号或缓存，不改变网络端点和更新信任链。GUI subsystem 只改变 Windows 如何创建主进程窗口；应用需要的 CLI/ConPTY 子进程仍由既有受控执行器创建。macOS 只在同步 Preview 发布时变更版本号和 build。

## Git 与发布节点

- `f909c10`：设计规格与实现计划；
- `5eb72ba`：只加入 PE 失败门禁，尚未修改生产入口；
- `ac50e08`：在获得真实 subsystem `3` 红灯后加入最小 GUI subsystem 修复；
- Pull Request：[#5](https://github.com/sljzdotcom/AI-Token-Meter/pull/5)。

## 已知限制与后续验收

CI 可以证明新 `.exe` 的 PE 类型为 GUI，但无法替代交互式 Windows 桌面验收。发布后仍需用户安装 `0.3.0-preview.2`，确认启动时只显示浮动条与系统托盘、不再出现 Windows Terminal；在取得该证据前，需求保持 `待用户确认`。
