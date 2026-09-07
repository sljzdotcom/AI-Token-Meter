# 2026-09-07：CLI 安装与账户引导、Windows 页签图标

关联 REQ-20260907-006/007；[规格](../design/specifications/2026-09-07-cli-onboarding-design.md)、[实施计划](../design/implementation-plans/2026-09-07-cli-onboarding.md)。

## 决策与来源

用户要求未安装可一键协助安装、待登录变色提醒、已登录可重新登录；Windows 设置页签加图标。依据用户“常规决策按推荐执行”的既有授权，选择官方独立安装器，不内嵌 CLI，也不额外安装 Node/Homebrew。已登录主操作选择“重新登录”，没有新增或执行退出登录。

2026-09-07 核对 [OpenAI Codex CLI](https://learn.chatgpt.com/docs/codex/cli) 与 [Claude Code 官方安装](https://code.claude.com/docs/en/setup)。macOS 使用各自官方 HTTPS shell 安装器，Windows 使用官方 PowerShell 安装器。用户必须点击后才打开可见安装流程；成功打开终端不算安装成功。

## 当前阶段

需求登记提交 `96a0bc4`；规格和计划 `95a4d3c`。在 `codex/cli-onboarding` 隔离工作区开发。CLI 安装/账户状态实现进行中，图标任务随后处理。0.4.0 公开安装包没有这些新功能。

## 验证边界

测试使用受控安装脚本 fixture，不在维护者机器上实际安装/重装 CLI 或更改真实账户。Windows 原生执行和字体/图标真实桌面外观必须区分本机测试、CI 与人工验收。
