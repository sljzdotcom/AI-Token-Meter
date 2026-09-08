# Task 3 报告：Claude 隔离工作区与初始化入口

日期：2026-09-08。关联：REQ-20260908-001。

## RED

- `cargo test --test claude_workspace`：失败，缺少 `platform::windows::claude_workspace`，证明原生/WSL 共用隔离工作区契约尚未实现。
- `npm test -- --run src/settings/cliOnboarding.test.ts src/settings/SettingsOnboarding.test.tsx`：4 项预期失败；自动/显式检查没有区分额度重试，初始化控制器、按钮和说明缺失。

## GREEN

- 原生 Claude 认证探测、额度采集与显式初始化统一使用 `%LOCALAPPDATA%/AI Token Meter/ClaudeUsageWorkspace`；环境缺失时只回退应用命名的临时目录，不使用用户主目录。
- WSL 使用发行版结构化参数和固定 `/bin/sh` 脚本，在该发行版当前用户的 `$HOME/.local/share/ai-token-meter/ClaudeUsageWorkspace` 创建并进入目录；发行版和 CLI 参数不插入脚本文本。
- 初始化只在用户点击后打开 Claude Code，不发送确认、信任或登录输入；启动成功只提示完成后手动检查，不伪造额度成功。
- Settings 初始加载、窗口 focus 和 onboarding 轮询传 `retryUsage: false`；只有显式 Check Status 传 `true`，解除认证/设置或暂时性 CLI 阻塞并触发单 Provider 刷新。有效 `rateLimited` 等待始终保留。
- 中英文按钮、说明、完成提示和失败重试提示已接入；Claude 忙碌时初始化、状态检查、登录和运行时控件统一禁用。

## 验证

- 聚焦 Rust：Claude workspace/collector/account/backoff 共 14 项通过；严格 Clippy 通过。
- 完整 Rust：本机非沙箱完整套件通过；第一次沙箱运行的 6 项 DeepSeek 本地服务器用例仅因 loopback bind `Operation not permitted` 失败，获准以相同命令运行后通过。
- 完整前端：85 项通过；production TypeScript/Vite build 通过。
- `scripts/check-docs.sh`：180 份 Markdown 通过。
- `cargo fmt --check`、`git diff --check`：通过。

## 限制

- 本机不是 Windows，未声称验证 CREATE_NEW_CONSOLE、WSL 或真实 Claude 交互；原生 Windows 编译与行为由父任务的 Windows CI 验证。
- 未修改凭据、CLI 信任配置、登录状态、安装程序、macOS 生产代码或更新通道。
- 需求整体仍为进行中，后续 Task 4、整体验证、Windows 真机复验和发布状态由父任务继续管理。
