# Gemini CLI 额度可行性表征

关联需求：REQ-20260908-012。日期：2026-09-08。下文先记录 Task1；文末 Task4a 附节扩展了合成环境的完整启动验证。Task1 范围：固定官方源码的内置命令、最终额度渲染与默认 headless 循环；不包含实际账号、CLI 启动或自动采集验收。

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
PASS: real /model renders 25% and 60%; empty stats hides quota; headless stats falls through to model boundary; headless model rejects; modelBoundaryAttempts=3; no real generation implementation; credential/network = 0.
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

## Task1 历史阶段：当时尚未覆盖的启动链

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

Task1 提交为 `8bf76ec`。独立审查通过，无 P1/P2；两项 Minor 已补：stdout 明示 3 次模型边界尝试且无真实生成实现，结果输出实际依赖版本与 lockfile SHA-256。本次 Task1 lockfile 摘要为 `cbace9eaacd30cc93bd35191839e9d1c0dcc66a5bdcde647c901b1d50641caa4`。

## Task4a：完整官方 CLI 合成启动探测

本附节扩展 Task1 的验证边界：固定官方 npm CLI 0.58.0 在真实 macOS 子进程/PTY 中启动，恢复合成 OAuth 缓存，获取进程内 quota 夹具，输入 `/model` 显示 Pro 25%、Flash 60% 和重置描述，再用 Esc、`/quit` 干净退出。没有修改上游 CLI 源码，没有使用真实账户或连接 Google 后端。产品采集器尚未实现，此结果可作为下一阶段采集设计的证据。

### 七个场景与实际断言

| 场景 | 官方 CLI 行为 | 判定 |
| --- | --- | --- |
| authenticated（默认限制组合） | 真实 `/model` 有 25%/60%/Resets；Esc、`/quit` exit 0；三个哨兵均无启动尝试 | 合成完整启动链通过 |
| authenticated --without-mcp-allowlist | 只有文件内 admin 禁用配置时，用户 MCP 哨兵仍尝试启动；测试权限阻止执行 | 否决“admin.mcp.enabled=false 文件设置足以禁用 MCP” |
| authenticated --sentinel-control | 开启 hooks/MCP/extensions 后，三个独立哨兵全部产生真实启动尝试，分别经 bash hook、用户 MCP、扩展 MCP；权限阻止执行 | 证明哨兵夹具接到了真实副作用路径 |
| missing-auth | 显示 Google 授权 URL 与 `Enter the authorization code:`；无网络夹具请求，不输入 `/model`；Ctrl+C 后 exit 0 | 需要认证，不能把退出码 0 当成功 |
| invalid-auth | 合成 tokeninfo 返回 401，随后同样等待授权码；仅 tokeninfo 请求；Ctrl+C 后 exit 0 | 失效账户不会给出额度 |
| quota-failure | quota 夹具返回 403；可打开模型对话框但无 `Model usage`；Esc、`/quit` exit 0，模型设置未改变 | 缺额度不冒充 0% 或点击模型选择 |
| version --sentinel-control | `--version` 输出 0.58.0、exit 0；未请求认证/配额、未尝试哨兵，但读取了 settings 与 `.env` | 版本预检仍需要同样的环境隔离 |

所有成功用例明确断言：截止时间内观察到退出、没有未知/模型端点请求、没有浏览器/系统 keychain helper 或 keytar addon 尝试、哨兵文件不存在、模型配置未持久化改变。安全组合断言的是“哨兵启动尝试为零”，不是仅断言“被权限阻止后实际执行为零”。普通 git、编辑器发现、终端父进程探测仍会尝试子进程；本测试拒绝这些操作，不能声称官方配置使整个 CLI 完全不尝试其他子进程。

### 复现与隔离边界

脚本：[test-gemini-cli-startup.mjs](../../scripts/test-gemini-cli-startup.mjs)。测试限定 macOS、Node 26.7.0、系统 Python PTY；这不是给用户安装 Node 26 的要求。所有包只安装到 `/private/tmp/req012-gemini-startup`：

```sh
npm install --prefix /private/tmp/req012-gemini-startup --ignore-scripts --no-audit --no-fund @google/gemini-cli@0.58.0 nock@14.0.10
node scripts/test-gemini-cli-startup.mjs authenticated
node scripts/test-gemini-cli-startup.mjs authenticated --without-mcp-allowlist
node scripts/test-gemini-cli-startup.mjs authenticated --sentinel-control
node scripts/test-gemini-cli-startup.mjs missing-auth
node scripts/test-gemini-cli-startup.mjs invalid-auth
node scripts/test-gemini-cli-startup.mjs quota-failure
node scripts/test-gemini-cli-startup.mjs version --sentinel-control
```

官方包完整性为 `sha512-++LtUYMcLE8dVxMcuwv6kIp8+h6z+std/7iVE+vSunkrwNDaWMFkWw/psv2RSySWjr2A1SsEEIGCK0xULWY2sA==`；脚本检查 npm package-lock 记录并输出 Node/nock 版本及 lock 摘要。本次 lock SHA-256 为 `8257f064a811bc7ba6471ff1fd824de50b292f87aabe91c7f170a9dafd3ca5d9`。npm 完整性核验发生在下载时；运行时没有把 lock 中的字符串校验夸大为重算整个已安装包的签名。

执行测试时保留以下边界：

- CLI 使用复制到专用目录的 Node，启用真实 Node 权限系统，仅允许读取测试根目录及单一 `/.dockerenv` 标记路径，写入本场景目录。环境从白名单重建，HOME/USERPROFILE/TMPDIR 指向合成目录，不继承真实用户 env 或认证变量。
- 真实网络、CLI 子进程创建、native addons、FFI 等默认禁止；启动前负控实际尝试读取专用目录外的**自建无害文件**、启动 `/usr/bin/true`、连接 loopback discard 端口，均观察到权限拒绝。没有用真实私密文件检验权限。
- Nock 只在进程内回应 `tokeninfo`、`loadCodeAssist`、`listExperiments`、`retrieveUserQuota` 四类官方端点。其他网络默认拒绝并记录；没有生成端点夹具。WASM 的 `data:` URL 走原生 fetch，不属于网络请求。
- 只为规避 Node JS realpath 对祖先 `/private` 做 lstat 的权限问题，将**测试目录内** callback realpath 改用 Node 自带 native realpath，保留真实路径解析结果。这个运行器适配不属于产品配置，不能宣称完全没有任何测试适配。
- Python 驱动的是实际 PTY，持续读取屏幕同时分时输入每个字符，Enter 单独发送。最初驱动在发键时停止读屏导致背压，把输入合并成粘贴；当时的超时/强制关闭结果未冒充通过。修正后正常流程约 8 秒、exit 0，无强制取消。截止时仍会关闭 PTY、尝试终止并记录实际 waitpid；没有观察到退出的用例一定失败。
- 全部 stdout/stderr 原始终端帧、请求路径/进程尝试/文件路径日志及退出状态保存在对应场景目录的 `terminal.txt`、`events.jsonl`、`outcome.json`、`result.json`。不记录请求 Authorization/header 或真实凭据；CLI 自己产生的失败日志只包含本次合成值。

### 给实际采集器的约束（不依赖 Node 26 测试权限）

1. **固定已验证版本与协议。** 本证据只覆盖官方 npm 0.58.0、macOS、默认交互路径。`--version` 在此版本会先加载 settings/`.env`，因此预检也要使用私有 cwd、空 `.env` 和受控环境；不能先在用户项目中跑一次“不带限制”的版本检测。
2. **使用实际生效的官方限制项。** `-e none` 禁用扩展；`--allowed-mcp-server-names <本次生成的随机UUID名称>` 将用户 MCP 排除（测试使用固定不存在名称便于复现，产品不应照抄固定名称）。系统覆盖 `hooksConfig.enabled=false`；同时 `privacy.usageStatisticsEnabled=false` 和 `telemetry.enabled=false`，因为仅后者仍会触发 system_profiler 硬件信息采集。IDE/自动更新/内存重启也在夹具中禁用。不得只依赖 `admin.mcp.enabled=false`。
3. **了解设置优先级。** 普通单值按 schema defaults → system defaults → user → trusted workspace → system overrides 合并；但 `computeMergedSettings()` 明确忽略所有文件中的 admin 字段，改用 remote admin/defaults。这解释了 MCP 负控。`GEMINI_CLI_SYSTEM_SETTINGS_PATH` 指向的是替换的系统设置文件，不是自动叠加文件；尚未验证保留现有企业系统配置的生产方案，不能擅自声称原有系统限制都会保留。
4. **认证与后台采集分开。** `NO_BROWSER=true` 已验证在缺凭据/401 时抑制浏览器启动，改为手工授权码等待；采集器应识别授权 URL/输码/账号选择/主题/欢迎等待态，立即取消并返回需要认证或不支持状态，不能继续发送 `/model`、登录码或自然语言，也不能仅靠退出码判成功。测试没有为欢迎/主题选择自动按确认；未知 UI 必须停止。调用官方 CLI 自己的 OAuth 缓存恢复，不让应用读取令牌内容。
5. **系统凭据提示的边界。** 本次白名单环境没有 `GEMINI_FORCE_ENCRYPTED_FILE_STORAGE`、`GOOGLE_APPLICATION_CREDENTIALS`、云端 access token 或代理/扩展注入变量；真实源码在 encrypted storage 开关为真时会改走凭据存储。本次缺/失效 plain OAuth 缓存没有 keytar/helper 尝试，不能泛化到加密存储账号。实际采集器若不能保证同等环境或遇到不支持的凭据模式，应停止，不自动打开 keychain/密码提示。此行为约束由环境和状态机实现，不能假定每个用户有 Node 26 权限系统。
6. **交互严格限于读额度。** 等待可识别的主输入态后逐键 `/model`，读取完整 `Model usage`；成功或缺额度都用 Esc 关闭，再 `/quit`。实时排空 PTY、使用单独 Enter，不能一次塞多行；对话框内不发送 Enter/方向键/Tab 或模型名称。403 测试证明固定退出序列没有改写 model 设置。
7. **保持官方终端口径。** Pro/Flash 是 CLI 分组后的模型档位；25%/60% 是 `1 - remainingFraction` 得到的已用比例并四舍五入，不是剩余比例，也不是完整原始模型池。保留 CLI 显示的重置描述；没有时为空，不从“1h”等描述捏造精确 resetAt、总请求上限或原始模型池。
8. **仍需生产生命周期实现和验收。** Node/Nock/合成 HOME/native realpath 只是测试护栏与夹具，产品不带这些适配。真实用户已有 CLI、实际账号恢复、实际后端、企业配置、Windows/ConPTY、不同终端宽度/语言和超时取消必须由后续实现与平台测试分别验证。源代码支持不等于所有这些现场状态都已通过。

内存/上下文也只覆盖当前夹具：私有 cwd 内空 `.env` 截断父目录环境扫描，合成全局 GEMINI.md 存在但没有生成调用；`context.fileName=[]` 在上游 UI 留下 `1 undefined file` 标签，因此不将这个空数组的 UI 细节推荐为正式产品行为。后续若改用独有的不存在文件名或其他上下文隔离方式，要对该配置单独补证据。

### 最终运行证据

- `authenticated-allowlist`：exit 0，8.16 秒，超时 false，额度可见 true，哨兵尝试 0。
- `authenticated`：exit 0，8.15 秒，超时 false，额度可见 true，哨兵尝试 1。
- `authenticated-control`：exit 0，8.16 秒，超时 false，额度可见 true，哨兵尝试 3。
- `missing-auth-allowlist`：exit 0，8.02 秒，超时 false，额度可见 false，哨兵尝试 0。
- `invalid-auth-allowlist`：exit 0，8.02 秒，超时 false，额度可见 false，哨兵尝试 0。
- `quota-failure-allowlist`：exit 0，8.15 秒，超时 false，额度可见 false，哨兵尝试 0。
- `version-control`：exit 0，1.23 秒，超时 false，额度可见 false，哨兵尝试 0。

后续只调整了超时分支的有界 waitpid 回收：版本场景复查通过，独立合成等待子进程在 2 秒截止后关闭 PTY 并观察到退出，总耗时小于 5 秒；没有拿七个正常退出用例冒充超时分支覆盖。

实际完整对话框帧（去除 ANSI 颜色、CR 与行尾空格后的原始内容，截取最近一次完整 model 对话框）：

```text
╭──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
│                                                                                                                                          │
│ Select Model                                                                                                                             │
│                                                                                                                                          │
│   1. Auto                                                                                                                                │
│      Let Gemini CLI decide the best model for the task: gemini-2.5-pro, gemini-2.5-flash                                                 │
│ ● 2. Manual                                                                                                                              │
│      Manually select a model                                                                                                             │
│                                                                                                                                          │
│ Remember model for future sessions: false (Press Tab to toggle)                                                                          │
│ > To use a specific Gemini model on startup, use the --model flag.                                                                       │
│                                                                                                                                          │
│ ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────── │
│ Model usage                                                                                                                              │
│                                                                                                                                          │
│ Pro         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬  25%  Resets: 5:47 PM (1h)
│ Flash       ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬  60%  Resets: 5:47 PM (1h)
│                                                                                                                                          │
│ (Press Esc to close)                                                                                                                     │
│                                                                                                                                          │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
```

采集器应使用完整最新屏幕中的 `Model usage` 区块；不得从累积 ANSI 文本重复读取旧帧，也不能把启动 footer 的 43% 聚合显示当作某个模型档位。403 场景保留 `Select Model` 对话框但整个 `Model usage` 标题与行消失。

### 持久依赖锁与系统路径定点

Task4a 独立审查通过，无 P1/P2；唯一 Minor 为依赖锁持久化，已将两次实际运行的 package.json/package-lock.json 原样保存在 [专用 fixture 与 npm ci 指引](../../scripts/fixtures/gemini-cli-probe/README.md)。锁文件只含公开 npm 元信息，SHA-256 与上述运行记录一致；复现不再依赖临时目录留存或重新解析 semver。

以下为固定 v0.58.0 源码定点核对，不读取真实本机系统配置：`packages/cli/src/config/settings.ts:104` 中 macOS 默认系统文件是 `/Library/Application Support/GeminiCli/settings.json`，Windows 为 `C:\ProgramData\gemini-cli\settings.json`，Linux 为 `/etc/gemini-cli/settings.json`，非空 `GEMINI_CLI_SYSTEM_SETTINGS_PATH` 优先。`:117` 中 defaults 文件由非空 `GEMINI_CLI_SYSTEM_DEFAULTS_PATH` 指定，否则为最终系统 settings 同目录下的 `system-defaults.json`；custom settings 路径会同时改变默认 defaults 目录。

`packages/core/src/code_assist/oauth2.ts:112` 的加密开关精确比较环境变量 `GEMINI_FORCE_ENCRYPTED_FILE_STORAGE === 'true'`；这是 env，不是 JSON settings 字段。常量定义见 `packages/core/src/mcp/token-storage/index.ts:13`。`oauth2.ts:697` 加密分支调用 OAuthCredentialStorage.loadCredentials，后者使用 HybridTokenStorage/KeychainTokenStorage；未开启时才读取官方 plain OAuth 缓存路径，此外仍可能读取 `GOOGLE_APPLICATION_CREDENTIALS`。本测试环境移除了这些注入变量；没有验证或迁移真实加密账户。

### Task4a 第八场景：保留默认信任设置并明确不信任目录

七场景合成设置中的 `security.folderTrust.enabled=false` 不是用户默认。上游默认 true，无匹配信任规则的新目录会由 `useFolderTrust.ts:79` 打开信任对话，即使目录为空。根开发入口另行授权了一个独立有界场景：保持 `folderTrust.enabled=true`，环境设置官方支持的 `GEMINI_CLI_TRUST_WORKSPACE=false`，其他限制与 `/model`/Esc/`/quit` 协议完全不变。

```sh
node scripts/test-gemini-cli-startup.mjs authenticated --untrusted-workspace
```

真实官方 CLI 已通过：输出 untrusted，仍显示 Pro 25%、Flash 60% 和重置描述，正常 exit 0，无强制取消、信任菜单、自动批准或 trustedFolders.json 写入；用户 security 设置保持 true，模型选择不变，三类哨兵尝试 0，未知/模型网络尝试 0。`core/utils/trust.ts:50` 明确返回 false，交互 hook 只在 undefined 时要求选择，这解释了观测。生产可用此更严格环境保留用户信任策略，无须为只读额度先信任私有目录；不能改成 true 或把 enabled 写成 false。

原始完整证据：`/private/tmp/req012-gemini-startup/authenticated-untrusted-allowlist/terminal.txt` 及同目录 result.json/events.jsonl/outcome.json。此场景仍为合成账号/macOS，其他未验证边界不变。
