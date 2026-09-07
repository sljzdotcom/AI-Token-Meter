# 2026-09-07：CLI 安装与账户引导、Windows 页签图标

关联 REQ-20260907-006/007；[规格](../design/specifications/2026-09-07-cli-onboarding-design.md)、[实施计划](../design/implementation-plans/2026-09-07-cli-onboarding.md)。

## 决策与来源

用户要求未安装可一键协助安装、待登录变色提醒、已登录可重新登录；Windows 设置页签加图标。依据用户“常规决策按推荐执行”的既有授权，选择官方独立安装器，不内嵌 CLI，也不额外安装 Node/Homebrew。已登录主操作选择“重新登录”，没有新增或执行退出登录。

2026-09-07 核对 [OpenAI Codex CLI](https://learn.chatgpt.com/docs/codex/cli) 与 [Claude Code 官方安装](https://code.claude.com/docs/en/setup)。macOS 使用各自官方 HTTPS shell 安装器，Windows 使用官方 PowerShell 安装器。用户必须点击后才打开可见安装流程；成功打开终端不算安装成功。

## 当前阶段

需求登记提交 `96a0bc4`；规格和计划 `95a4d3c`。在 `codex/cli-onboarding` 隔离工作区开发。CLI 安装/账户状态实现进行中，图标任务随后处理。0.4.0 公开安装包没有这些新功能。

## 验证边界

### 第一轮实现验证

- Swift 重复登录点击先出现 3 项预期失败，再改为防重复后通过；安装脚本 fixture 覆盖两服务，失败下载不执行、退出码保留与临时文件清理。
- 前端实现第一轮 63 项测试、production build、21 项密度进程生命周期与 632 项 Chrome 计算样式通过；不是最终提交的验收结论。
- 全量 Swift 第一轮 414 项/80 组中，既有 `CLICollectorTests.codexTimeoutIsBounded()` 在第 226 行得到 `transportFailure`（约 1.916s）。该测试和 collector 无差异，单独 `CLICollectorTests` 15/15 通过，未改原断言或截止时间。并入既有 REQ-20260906-003 跟踪；本机完整原始输出位于 `/private/tmp/cli-swift-full.log`，临时日志不进入公开仓库。后续需记录整套复验，不能用隔离通过推断根因已消除。

测试使用受控安装脚本 fixture，不在维护者机器上实际安装/重装 CLI 或更改真实账户。Windows 原生执行和字体/图标真实桌面外观必须区分本机测试、CI 与人工验收。
