# Gemini CLI 额度可行性表征

关联需求：REQ-20260908-012。日期：2026-09-08。范围：固定官方源码的内置命令、最终额度渲染与默认 headless 循环；不包含实际账号、CLI 启动或自动采集验收。

## 结论

| 路径 | 本次实际执行结果 | 接入含义 |
| --- | --- | --- |
| 交互 `/model` | 原始命令刷新合成 quota 后返回 model 对话框；原始 ModelDialog、ModelQuotaDisplay 和 ProgressBar 经 React/Ink 渲染，空会话可见 Pro 25%、Flash 60% 及重置时间 | 命令与额度组件级无模型调用可行，PTY 自动采集仍未证实 |
| 交互 `/stats` | 原始命令刷新并记录 quota，原始 StatsDisplay 输出会话摘要，没有额度 | 不能从刷新成功推断终端可见额度 |
| 交互 `/stats model` | 空会话最终显示 `No API calls have been made in this session.` | 不能用于新建空会话读取额度；不能为使其可用先发模型请求 |
| 默认 headless `/stats`、`/stats model` | 原始 loader、命令解析、命令动作和 runNonInteractive 执行后，继续把原文本送到 sendMessageStream 边界 | 不可作为无推理额度查询；本次边界哨兵抛错阻止实际调用 |
| 默认 headless `/model` | 命令返回 dialog，原始处理器抛 `Exiting due to command result that is not supported in non-interactive mode.` | 拒绝，不到达模型边界 |
| 默认 headless 未知 `/not-a-command` | 同样到达 sendMessageStream 边界 | `-p` 不能当只执行内置指令的安全开关 |

这不是“永久无法自动采集”的结论。已找到可见的官方额度来源；保留官方认证的完整启动与交互自动化仍需另行验证。产品目前不能依据本报告自动启动用户 CLI。

## 固定基线与真实执行范围

官方版本 [v0.58.0](https://github.com/google-gemini/gemini-cli/releases/tag/v0.58.0)，提交 `ac9431c9e2290d68af31a77614ff2fddb2391ca3`。上游 package.json 版本为 0.58.0。父任务取得的官方归档解压目录为 `/private/tmp/req012-gemini-source`；归档 SHA-256 为 `65be4e1ae73029d33b92bcc7a0ddcd3b05904dda3c658852a125d40f4a601d11`。

探测脚本 [test-gemini-cli-capability.mjs](../../scripts/test-gemini-cli-capability.mjs) 对 22 个真实上游文件逐个转译 TypeScript，执行其原始逻辑，不以源码搜索代替行为断言。合并文件摘要固定为 `af6262dc728d48b9a9cf273afd6014f1c58b7534abc0f2f9be88be2be4e9c395`；输入改动即拒绝继续，结果 JSON 保留各文件 SHA-256。

实际链路：

- statsCommand → StatsDisplay / ModelStatsDisplay；真实 computeStats、formatters、Table、QuotaStatsInfo。
- modelCommand → ModelDialog → ModelQuotaDisplay → ProgressBar；真实 React 19.2.0 与 Ink 6.8.0 最终文本帧。
- BuiltinCommandLoader → CommandService → SlashCommandResolver → parseSlashCommand → handleSlashCommand → 默认 runNonInteractive；sendMessageStream 是终止边界。

这些模块在显式 VM 模块允许列表内执行。仅必要的外围副作用替换为合成配置、账户身份、quota 刷新、上下文、遥测、外部命令加载、控制台和模型边界。无真实凭据实现被加载；VM 内文件系统读操作与 fetch 被拒绝。非额度相关的模型选择单选列表被简化为标题文本，按键订阅不连接 TTY；额度条件、分组、百分比、reset 格式和实际 Ink 布局保持真实。被测试的默认 headless 调度、解析、动作与 fallback 分支没有替换。

账户适配器仅返回 `fixture@example.invalid`，完整 quota 字段取本次所需的上游 bucket 结构。模型层没有合成模型输出，只在尝试调用时抛出 `MODEL_BOUNDARY`。`modelGenerationCalls=0` 表示本探测没有实际生成实现；必须与非零 `modelBoundaryAttempts` 一起阅读，不能声称 headless 没有尝试推理。`secretReads=0` 仅描述隔离探测，不证明真实 OAuth 初始化不读凭据。

## 可复现入口

前置条件：Node 26.7.0（本次版本），仅在专用临时目录放源码和依赖。不得安装或启动用户实际 Gemini CLI，不使用其账号。依赖包安装时禁用 lifecycle scripts；不安装上游 CLI 或整个 monorepo 的依赖。

```sh
mkdir -p /private/tmp/req012-gemini-source /private/tmp/req012-gemini-probe
curl -fL https://api.github.com/repos/google-gemini/gemini-cli/tarball/ac9431c9e2290d68af31a77614ff2fddb2391ca3 -o /private/tmp/req012-gemini-source.tar.gz
tar -xzf /private/tmp/req012-gemini-source.tar.gz -C /private/tmp/req012-gemini-source --strip-components=1
npm install --prefix /private/tmp/req012-gemini-probe --ignore-scripts --no-audit --no-fund esbuild@0.25.12 react@19.2.0 ink@6.8.0 ink-testing-library@4.0.0 strip-ansi@7.1.2
node --experimental-vm-modules scripts/test-gemini-cli-capability.mjs
```

下载是独立的公开依赖准备步骤；探测执行期间不访问网络。`GEMINI_PROBE_SOURCE`、`GEMINI_PROBE_DEPS`、`GEMINI_PROBE_OUTPUT` 可更换专用路径。Node 会打印 VM Modules experimental warning，这是测试运行器提示。

反例验证入口（预期 exit 1）：

```sh
node --experimental-vm-modules scripts/test-gemini-cli-capability.mjs --expect-empty-stats-quota
```

本次先看到错误能力假设被真实渲染否决：`AssertionError: Rejected assumption: empty-session model stats must expose quota`，实际值只有空会话提示，缺少 `25%`。正常入口随后通过。此任务是已有上游行为的窄表征，没有产品实现修改，不冒充产品 TDD 红绿过程。

正常输出：

```text
PASS: real /model renders 25% and 60%; empty stats hides quota; headless stats falls through to model boundary; headless model rejects; generation/credential/network = 0.
Evidence: /private/tmp/req012-gemini-probe/result.json
```

## 本次原始渲染与循环证据

```text
╭──────────────────────────────────────────────────────────────────────────────────────────────────╮
│                                                                                                  │
│ Select Model                                                                                     │
│                                                                                                  │
│ auto | Manual                                                                                    │
│                                                                                                  │
│ Remember model for future sessions: false (Press Tab to toggle)                                  │
│ > To use a specific Gemini model on startup, use the --model flag.                               │
│                                                                                                  │
│ ──────────────────────────────────────────────────────────────────────────────────────────────── │
│ Model usage                                                                                      │
│                                                                                                  │
│ Pro         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬  25%  Resets: 5:27 PM (1h)       │
│                                                                                                  │
│ Flash       ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬  60%  Resets: 6:27 PM (2h)       │
│                                                                                                  │
│                                                                                                  │
│ (Press Esc to close)                                                                             │
│                                                                                                  │
╰──────────────────────────────────────────────────────────────────────────────────────────────────╯
```

```json
{
  "counters": {
    "modelGenerationCalls": 0,
    "modelBoundaryAttempts": 3,
    "secretReads": 0,
    "networkAttempts": 0,
    "quotaRefreshes": 4
  },
  "headless": [
    {
      "input": "/stats",
      "modelBoundaryAttempts": 1,
      "error": "MODEL_BOUNDARY:[{\"text\":\"/stats\"}]",
      "output": ""
    },
    {
      "input": "/stats model",
      "modelBoundaryAttempts": 1,
      "error": "MODEL_BOUNDARY:[{\"text\":\"/stats model\"}]",
      "output": ""
    },
    {
      "input": "/model",
      "modelBoundaryAttempts": 0,
      "error": "Exiting due to command result that is not supported in non-interactive mode.",
      "output": ""
    },
    {
      "input": "/not-a-command",
      "modelBoundaryAttempts": 1,
      "error": "MODEL_BOUNDARY:[{\"text\":\"/not-a-command\"}]",
      "output": ""
    }
  ]
}
```

## 启动链尚未覆盖的部分

没有执行官方 CLI main、认证恢复、真实 refreshUserQuota 网络响应、真实终端键盘/PTY/ConPTY、TTY 退出、Windows、ADK 非交互分支，也没有验证不同账号/tier/语言/版本的终端可解析性。没有证明 CLI 启动到 `/model` 之间无模型调用或其他用户配置副作用。

源码证据表明仅换工作目录不充分：[gemini.tsx](https://github.com/google-gemini/gemini-cli/blob/ac9431c9e2290d68af31a77614ff2fddb2391ca3/packages/cli/src/gemini.tsx) 调用 config.initialize 后触发 SessionStart hook；[config.ts](https://github.com/google-gemini/gemini-cli/blob/ac9431c9e2290d68af31a77614ff2fddb2391ca3/packages/cli/src/config/config.ts) 在创建 Config 前加载扩展；[settings.ts](https://github.com/google-gemini/gemini-cli/blob/ac9431c9e2290d68af31a77614ff2fddb2391ca3/packages/cli/src/config/settings.ts) 合并系统、用户和工作区设置。

找到的官方隔离候选项（只完成源码核对，尚未组合行为验证）：

- `hooksConfig.enabled=false`：schema 明确禁止执行 hooks，传入 Config.enableHooks。
- `admin.mcp.enabled=false`、`admin.extensions.enabled=false`：控制 MCP/扩展能力。
- `-e none`：extensionEnablement 明确支持禁用全部扩展。
- `GEMINI_CLI_SYSTEM_SETTINGS_PATH`：指定系统配置文件，可能提供不改用户配置的覆盖；系统 defaults 有独立路径变量。
- `context.memoryBoundaryMarkers=[]`：禁止父目录遍历，不等于禁用全部全局记忆加载。

用户 settings 和 OAuth 缓存仍共用 Storage.getGlobalGeminiDir 派生路径；不能用换 HOME 后无账号的表现证明真实认证兼容，也不能复制用户凭据到夹具。系统覆盖的优先级、受信任工作区、扩展早期加载、global GEMINI.md、.env、自定义命令/MCP/IDE/遥测等启动路径仍需纳入后续验证。

下一步可行的最小测试起点：在专用临时 HOME/cwd 创建带哨兵的 settings、global hook/MCP/extension/memory 文件，以临时依赖执行上游真实 loadSettings → loadCliConfig → Config.initialize，使用系统覆盖禁用上述项；在进程层拒绝网络/子进程，断言没有执行哨兵，合成 OAuth 边界仅提供已认证状态。再扩展至 main/PTY 输入 `/model` 和退出。该方法可以补齐配置/启动副作用证据，但合成认证仍不能变成真实账号验收；无需先请求用户改变产品数据范围。

进一步可在专用临时目录安装固定官方 npm CLI，通过真实 PTY 输入 `/model`，以子进程内拦截器提供合成 OAuth、Code Assist 账户初始化及 quota 回复，拒绝全部其他网络，并记录生成端点和 hooks/MCP/extensions 的进程启动尝试。这是可行的后续技术验证，但需要完整 CLI/PTY 依赖及多步协议夹具，范围大于本次窄探测；应先观察拒绝日志，再逐一加入必要的合成响应。单独拦截 retrieveUserQuota 无法证明整个启动链。即使通过，仍不包含真实账号、真实服务或 Windows 验收。

可复用的上游测试位置：`packages/cli/src/config/settings.test.ts` 的 system/admin 合并用例、`config/config.test.ts` 的 MCP/admin/hooks 参数用例、`config/extension-manager.test.ts`、`config/extensions/extensionEnablement.test.ts`、`core/initializer.test.ts`、`gemini.test.tsx` 与 `packages/core/src/config/config.test.ts`。这些位置已定位，未在本任务执行；不能把它们的存在记成验证通过。

## 验证与交付

本阶段正常探测通过；错误空会话额度假设按预期失败；缺少 quota 时真实对话框不显示使用率，没有把未知当零。运行文档检查，结果见任务回传。没有运行产品全套测试，因为本阶段仅新增隔离上游表征脚本和记录。没有安装、启动用户 CLI，没有登录、模型请求、发布或产品数据采集。独立审查由主开发入口安排，不能用本次自查代替。

Git 提交以 `test: characterize Gemini CLI quota command boundaries` 为索引；最终 SHA 在任务回传与后续整合记录中列出。
