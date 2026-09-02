# AI Token Meter 服务账户显示与重新登录设计规格

**日期：** 2026-09-01  
**状态：** 已确认，待实施  
**范围：** Settings > Services 中的 Claude、Codex 与 DeepSeek 账户管理  
**基线：** `main` 提交 `228f781`

## 1. 背景与现状

Settings 的 Services 页面目前只说明 Claude、Codex 的认证由各自 CLI 管理，没有显示当前账户，也没有登录或重新登录入口。Claude 的现有 `Open one-time setup` 只负责批准 AI Token Meter 私有工作区，不是账户登录。Codex 已通过 app-server 读取额度，但没有读取 `account/read`。

DeepSeek 已允许在空白 SecureField 中输入新 API Key 并保存；保存会覆盖 Keychain 中的旧 Key，但界面仍显示通用的 `Save`，没有说明它会替换现有 Key，也会在验证新 Key 前丢失原来可用的 Key。

本机 CLI 能力已经核实：

- Claude 提供 `claude auth status --json` 与 `claude auth login`；
- Codex 提供 `codex login status`、`codex login`，app-server 的 `account/read` 可返回 ChatGPT 邮箱、套餐类型与账户类型；
- DeepSeek 的现有余额接口可验证候选 API Key，但不提供稳定的邮箱账户身份。

## 2. 目标

1. Services 页面始终显示 Claude、Codex、DeepSeek 的当前连接状态。
2. Claude、Codex 在已登录时显示当前账户，在失效时显示需要登录。
3. Claude、Codex 的登录按钮始终存在：已登录显示 `Sign in again`，未登录显示 `Sign in`。
4. 登录按钮只启动官方 CLI 登录流程；AI Token Meter 不接收、保存或代理密码、OAuth Token。
5. 官方登录完成后，应用自动检测账户变化并刷新额度。
6. DeepSeek 已配置时明确显示 `Replace API Key`，未配置时显示 `Save API Key`。
7. DeepSeek 新 Key 必须先验证，验证成功后才覆盖 Keychain 中的旧 Key。
8. 账户身份信息只在内存和 Settings 中使用，不进入缓存、Widget、通知或日志。

## 3. 非目标

- 不在 AI Token Meter 内嵌 Claude、ChatGPT 或 DeepSeek 的网页登录页。
- 不增加 Claude、Codex 的退出登录按钮。
- 不让 AI Token Meter 读取或保存 CLI 的访问 Token、Cookie 或刷新 Token。
- 不把账户邮箱加入 `UsageSnapshot`、本机活动记录或 Widget 快照。
- 不修改浮动条、详情页、额度算法、登录项、Widget 或刷新间隔设置。
- 不替换 Claude 私有工作区的一次性授权流程。

## 4. 方案选择

采用方案 A：统一账户状态卡片、始终可见的官方重新登录按钮，以及 DeepSeek 两阶段安全替换。

未采用的方案：

- **只在失效时显示登录按钮：** 页面更简洁，但无法主动切换账户，且按钮出现/消失会改变布局。
- **应用内嵌网页登录：** 视觉上集中，但会扩大 OAuth、Cookie、WebView 和账号安全边界，也容易跟随第三方页面变化而失效。

## 5. Services 页面设计

### 5.1 通用状态结构

三个服务继续使用独立 Form Section。每个 Section 的首行保持相同信息层级：

1. Provider 名称；
2. 连接状态图标与文字；
3. 当前账户描述；
4. 主操作按钮；
5. 必要时显示次要的 `Check Status` 操作和反馈信息。

状态文字使用系统语义色和图标，Settings 全部继续使用 macOS 默认字体，不受 Antonio、DIN Condensed 或内容字号设置影响。

### 5.2 Claude

可能显示：

- `Connected` + CLI 返回的邮箱；
- CLI 没有提供邮箱时，显示认证方式，例如 `Claude account · OAuth`；
- `Sign-in required`；
- `Claude CLI not installed`；
- `Status unavailable`，用于超时、不可解析输出或临时执行失败。

主按钮始终存在：

- 已连接：`Sign in again`；
- 未连接或状态不可用：`Sign in`；
- CLI 未安装：按钮禁用，并显示缺少 CLI 的说明。

账户登录与私有工作区授权分开显示。`Sign in` 运行 `claude auth login`；工作区授权继续运行现有安全模式命令。

### 5.3 Codex

应用通过现有 Codex app-server 会话调用 `account/read`。ChatGPT 账户显示：

- 邮箱；
- 套餐类型，例如 Plus、Pro、Team；
- 登录方式 `ChatGPT`。

如果 Codex 使用 API Key，app-server 不提供邮箱时显示 `API Key account`，不得展示 Key 或推测邮箱。没有账户时显示 `Sign-in required`。

主按钮始终存在：

- 已连接：`Sign in again`；
- 未连接或状态不可用：`Sign in`；
- CLI 未安装：按钮禁用。

点击后运行官方 `codex login`，不使用内部、不稳定的 Token 注入方式。

### 5.4 DeepSeek

DeepSeek API 不提供稳定的账户邮箱，因此不把 API Key 冒充为用户账户。已配置时显示：

```text
Connected · API Key ••••ABCD
```

只显示最后四个字符；Key 长度不足时只显示 `API Key configured`。未配置或官方接口明确拒绝认证时显示 `API Key required`。

SecureField 永远为空，不回填现有 Key。按钮规则：

- 未配置：`Save API Key`；
- 已配置：`Replace API Key`；
- 候选 Key 为空：禁用；
- 验证期间：禁用输入和按钮并显示 `Verifying…`。

`Remove API Key` 继续保留为独立破坏性操作。

## 6. 账户状态模型与边界

新增独立于 `UsageSnapshot` 的内存模型，例如：

```text
ServiceAccountStatus
  provider
  connectionState
  accountLabel
  accountDetail
  authenticationMethod
  checkedAt
```

`connectionState` 至少覆盖：

- connected；
- signInRequired；
- notInstalled；
- checking；
- unavailable。

账户模型不实现 `Codable`，不交给 SnapshotCache、WidgetSnapshotBuilder 或通知服务。Provider 专用解析器只返回已经过最小化的展示字段，不向 UI 暴露原始 JSON。

## 7. 状态读取

### 7.1 Claude 状态探针

运行已定位到的 Claude 可执行文件：

```text
claude auth status --json
```

解析 `loggedIn`、`authMethod` 以及 CLI 版本可能提供的可选邮箱/订阅字段。字段缺失时降级为认证方式，不把缺失邮箱判定为登录失败。非零退出码且 JSON 明确表示 `loggedIn = false` 时为 `signInRequired`；超时或无法解析则为 `unavailable`。

### 7.2 Codex 状态探针

通过 app-server 初始化后调用 `account/read`：

- ChatGPT：读取可选邮箱与 `planType`；
- API Key：只显示账户类型；
- account 为 `null` 且要求 OpenAI 认证：`signInRequired`；
- 协议错误或超时：`unavailable`。

账户读取和额度读取共享底层 app-server 请求实现，但保持不同返回模型。邮箱不得进入额度快照。

### 7.3 DeepSeek Key 描述

Keychain 读取必须在后台执行并受超时保护，不能破坏现有“应用初始化不被 Keychain 阻塞”的保证。完整 Key 只在验证请求和计算四字符后缀的局部作用域存在；内存状态只保存遮罩描述。

## 8. 登录启动与自动回查

新增通用 CLI 认证启动器，职责仅限：

1. 定位官方可执行文件；
2. 使用安全 shell quoting 生成仅包含固定登录命令的临时 `.command`；
3. 以 `0700` 权限写入应用专用目录；
4. 通过 macOS 默认 Terminal 打开；
5. 不把邮箱、Token、API Key 或用户输入写进脚本。

命令固定为：

```text
claude auth login
codex login
```

启动后，AppModel 对对应 Provider 进行最长两分钟的有限回查。回查每 2–3 秒执行一次；检测到 connected 后立即停止、更新账户状态并触发一次额度刷新。窗口关闭、应用退出或用户再次启动登录时取消旧回查任务。

两分钟内未完成不判定登录失败，显示“登录窗口仍可继续，完成后点 Check Status”。`Check Status` 始终是无副作用的手动回查。

## 9. DeepSeek 两阶段替换

替换过程必须保证旧 Key 在新 Key 验证前保持可用：

1. 标准化候选 Key，拒绝空白；
2. 使用候选 Key 调用现有官方余额接口，但不写 Keychain；
3. 成功得到有效余额响应后才保存候选 Key；
4. 保存成功后更新遮罩后缀、清空 SecureField，并刷新余额和 30 天数据；
5. 官方明确返回认证失败时显示 `API Key is invalid`，旧 Key 不变；
6. 网络、超时或服务端错误时显示 `Could not verify the new API Key`，旧 Key不变；
7. Keychain 写入失败时显示保存错误，旧 Key 仍保持原值。

如果 Keychain 实现不能原子覆盖，保存前在受限局部作用域读取旧 Key；新值写入失败时恢复旧值。测试必须证明任何失败路径都不会删除或覆盖原有有效 Key。

## 10. 错误处理与反馈

- CLI 未安装：显示明确状态，登录按钮禁用；不自动下载软件。
- CLI 状态超时：保留上一次成功识别的账户描述，并标记 `Status unavailable`，避免闪烁成未登录。
- 用户取消官方登录：有限回查自然停止，不修改现有凭据。
- 多次点击登录：取消旧回查，最多保留一个登录启动任务和一个状态回查任务。
- 账户切换成功但额度暂时失败：账户显示 connected，额度继续使用现有缓存和错误语义，两者不能相互覆盖。
- DeepSeek 候选 Key 验证失败：SecureField 内容保留，方便用户修正；旧 Key 与旧余额继续可用。
- 所有用户可见反馈路由到 Services Tab，并区分 Claude 登录、Claude 工作区、Codex 登录和 DeepSeek Credential。

## 11. 隐私与安全

- Claude/Codex 邮箱只保留在 AppModel 的内存状态，应用退出即消失。
- 邮箱、套餐、Key 后缀不得进入 SnapshotCache、Widget、通知、分析图表、命令脚本或日志。
- 不记录 CLI 原始状态 JSON；错误日志只记录归一化错误类型。
- DeepSeek 完整 Key 继续只存放于 Keychain；UI 最多显示后四位。
- 登录按钮启动官方 CLI 流程，所有密码、浏览器授权和 MFA 都由官方工具处理。
- 不自动执行 logout，不删除 Claude/Codex 的官方凭据。

## 12. 测试策略

### 账户解析与状态

- Claude 已登录、未登录、无邮箱、未知字段、非零退出码和超时；
- Codex ChatGPT 邮箱/套餐、API Key 账户、空账户、协议错误和超时；
- CLI 未安装映射到 notInstalled；
- 上次成功账户在暂时不可用时保留，但状态明确降级。

### 登录启动器

- 只生成批准的两个固定命令；
- 路径包含空格和引号时正确 quoting；
- 脚本权限为 `0700`；
- 脚本不包含账户、Token、Key 或环境秘密；
- 重复登录取消旧回查；成功、超时、取消和应用结束都正确收尾任务。

### DeepSeek 替换

- 首次保存成功；
- 已有 Key 时验证成功后替换；
- 认证失败、网络失败、超时、无效响应和 Keychain 写入失败时旧 Key 均不变；
- 成功后只保留遮罩后缀，SecureField 清空；
- Keychain 慢读不能阻塞 AppModel 初始化。

### UI 与隐私合同

- 三个 Section 始终显示账户状态和主操作；
- 已连接使用 `Sign in again` / `Replace API Key`；
- 未连接使用 `Sign in` / `Save API Key`；
- Settings 继续使用系统字体；
- 账户邮箱与 Key 后缀不会出现在 Codable 快照、Widget JSON、缓存、通知或测试日志中；
- 原有 227 项回归继续通过。

## 13. 实机验收

1. Claude 未登录时显示 Sign-in required；点击 Sign in 打开官方登录流程；完成后自动显示当前账户。
2. Claude 已登录时仍显示 Sign in again；重新登录后账户和额度刷新。
3. Codex 显示当前 ChatGPT 邮箱和套餐；Sign in again 可切换账户。
4. Codex 使用 API Key 时只显示 API Key account，不泄露 Key。
5. DeepSeek 显示当前遮罩 Key；正确新 Key 验证后替换并刷新余额。
6. 错误 DeepSeek Key、断网和超时都不会破坏旧 Key。
7. 关闭再打开 Settings 后状态仍可重新获取；没有把账户身份持久化到磁盘。
8. Claude 私有工作区授权按钮仍可用，且不会被误标为账户登录。

## 14. 文档与版本记录

实现后更新：

- `README.md`；
- `CHANGELOG.md`；
- `docs/user-guide/settings.md`；
- `docs/user-guide/troubleshooting.md`；
- `docs/security-and-privacy.md`；
- 对应开发日志与 `docs/development/commit-history.md`。

## 15. 完成标准

- 三个 Provider 在 Services 中始终显示准确、可降级的账户状态；
- Claude/Codex 可一键启动官方重新登录并自动回查；
- Codex ChatGPT 账户显示邮箱与套餐，Claude 在 CLI 提供邮箱时显示邮箱；
- DeepSeek 新 Key 只有验证成功才替换，所有失败路径保留旧 Key；
- 账户身份不进入持久化用量数据或 Widget；
- 自动化测试、Release 构建、签名、安装和真实 Settings 验收通过；
- 实现经过代码审查后再合入 `main`。
