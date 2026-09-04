# 服务与指标说明

## 总览

AI Token Meter 不把三家服务强行换算成同一个“额度”。每个圆环严格遵循对应服务能够可靠获得的数据：

| 服务 | 圆环含义 | 详情补充 |
| --- | --- | --- |
| Claude Code | 官方当前额度窗口的已用比例 | 次级额度、重置时间，以及本机近 30 天活动 |
| OpenAI Codex | 官方通用速率限制的已用比例 | 次级窗口、重置额度，以及本机近 30 天三项活动聚合 |
| DeepSeek | 相对余额基准已经消耗的比例 | 当前余额、近 30 天成本/请求/Token 与每日成本图 |

## Claude Code

### 数据来源

AI Token Meter 先检查 Claude Code CLI 的认证状态，再在专用空工作区内通过伪终端启动 Claude Code，并执行 `/usage`。输出经 ANSI 清理后解析为统一快照。

Windows 可选择 Native 或 WSL CLI，使用 ConPTY、固定终端输入、超时和 Job Object；macOS 使用 PTY。Services 展示实际来源和版本，但两平台都只把解析后的额度字段交给 UI。

### 为什么使用隔离工作区

- 避免打开用户代码仓库时加载项目指令和工具；
- 避免 MCP 启动或项目扫描拖慢简单的额度查询；
- 避免当前终端目录没有被 Claude Code 信任；
- 让自动查询行为保持稳定、可复现。

首次运行可能仍需由用户批准该空工作区。AI Token Meter 会提供一次性设置按钮，不会代替用户接受授权提示。

### 数据口径

- 主指标通常是当前会话额度；
- 次指标通常是周额度或所有模型额度；
- 促销说明中的“增加 50%”等数字不会被误认为当前用量；
- 官方客户端与 AI Token Meter 刷新时间不同会产生短暂差异，手动刷新后应以最新官方值为准。

### 本机近 30 天活动

Claude Code 详情页上方始终是官方额度，下方 **Last 30 days · This Mac** 单独展示当前 Mac 可读取的本机活动：

- Sessions：主会话的去重数量，`subagents` 记录不会虚增会话数；
- Active days：30 个本机自然日中实际产生 Token 的天数；
- Tokens：Input、Output、Cache creation 与 Cache read 的合计；
- Daily token activity：固定 30 日柱形趋势，无活动的日期保留为零。

为保持详情简洁，页面不再单独展示 Input/Output/Cache 构成或模型排行。历史缓存和采集结果仍保留这些聚合字段以保证兼容，但不会把它们显示在详情页。

详情页也不再重复显示单独的隐私说明。应用仍只读取白名单聚合字段，完整边界见[隐私与安全](../security-and-privacy.md)。

该区域不是 Anthropic 账户的跨 Web、Desktop、移动端和多设备官方月报。个人 Pro/Max 目前没有稳定公开的跨设备 30 天明细接口，因此 AI Token Meter 不把本机活动伪装成官方总量。本机目录不可读时，区域显示不可用，官方额度仍正常刷新。

读取范围严格限制为 `~/.claude/projects` 中 JSONL 的时间、会话 ID、模型 ID 和 usage 数字；提示词、回复、项目路径、分支、标题和文件内容不会进入聚合模型、缓存或日志。

## OpenAI Codex

### 数据来源

AI Token Meter 启动已安装的 OpenAI Codex CLI `app-server`，通过 JSON-RPC 调用 `account/rateLimits/read`，读取账户返回的结构化速率限制。

定位器依次检查当前 PATH、用户级目录、常见 Node 管理器、Homebrew/系统目录，并以后备方式检查用户或系统 Applications 中 ChatGPT/Codex App 的内置 `codex`。nvm/npm 脚本启动时会把自身 `bin` 目录置于子进程 PATH 首位，使脚本使用相邻 Node 运行时；不会写死某个 Node 版本，也不会修改用户 Shell 配置。

Windows 定位器检查环境与注册表 PATH、常见 Node 安装和 WSL 发行版；选定后通过同一 `app-server` 协议读取 account/rateLimits。Native 与 WSL 状态不会混合，找不到 CLI 时不会用伪造 0% 代替。

### 数据口径

- 优先展示顶层通用额度，不用模型专属窗口覆盖通用额度；
- 圆环与百分比都使用同一个 `usedFraction`，不会出现“文字 0%、圆环却前进”的分离计算；
- 可用重置额度在详情中以独立卡片只读展示：数量使用胶囊标签，每张券显示名称、完整到期时间和按本机日历计算的剩余天数；多张券按到期时间排列，缺失明细会明确提示；
- AI Token Meter 不提供“立即使用”按钮，也不会自动消耗重置额度。

### 本机近 30 天活动

OpenAI Codex 详情把官方额度放在上方，下面单独标记 **Last 30 days · This Mac**，显示：

- Token：近 30 天内有更新的本机 OpenAI Codex 线程所记录的 Token 总数；
- Current streak：今天或昨天结束的连续本机活动日；
- Longest session：这些线程中创建到最后更新时间最长的一次。

三项都是本机估算，不是 OpenAI 账户跨设备统计。AI Token Meter 只查询 OpenAI Codex 本地状态库的 `tokens_used`、`created_at`、`updated_at` 聚合列，不读取对话标题、预览、提示词或回复。本地状态库缺失时，官方额度和充值券仍正常显示。

## DeepSeek

### 余额

余额来自 DeepSeek 官方 `/user/balance` API，API Key 在 macOS 从 Keychain、在 Windows 从 Credential Manager 读取。圆环的计算方式为：

```text
已消耗比例 = clamp((余额基准 - 当前余额) / 余额基准, 0, 1)
```

默认余额基准为 ¥100。充值后余额高于基准时圆环回到 0%；余额接近 0 时圆环接近完整一圈。该基准只用于可视化，不是账单、预算控制或付款设置。

### 最近 30 天使用情况

DeepSeek 没有在当前余额 API 中同时提供官网控制台的完整 30 天图表数据。AI Token Meter 因此使用应用内隔离网页会话（macOS WebKit、Windows WebView2）打开官方用量页，并只处理官方来源的相关 JSON 数据。当前官网分别返回每日用量与每日费用；AI Token Meter 会等待两组数据到齐、按日期合并后才替换完整缓存，避免半份响应把现有图表清空。

Windows 查看 DeepSeek 详情不会自动创建官网窗口。只有用户点击 **Sync official history** 才开始同步：opening/active 状态会暂停详情自动隐藏；已打开的官网窗口会被复用并聚焦；用户关闭、加载失败或 30 秒仍未就绪时会清理当前会话、恢复详情并允许重试；完整聚合成功后会关闭官网窗口并恢复更新后的原生图表。macOS 现有 WebKit 行为不因 Windows 专属修复而改变。

标准化后保存的字段只有：

- 日期；
- 当日成本（CNY）；
- API 请求数；
- Token 数；
- 更新时间与状态说明。

网页原始响应、Cookie、请求头、登录表单和 API Key 不会写入业务缓存。网站接口变化、未登录或网络失败时，AI Token Meter 会显示缓存或引导打开官方页面，而不会伪造数据。

## 刷新与缓存

- 启动后立即刷新；
- 正常情况下每 5 分钟刷新；
- 菜单栏支持手动刷新；
- 同一时刻只执行一轮刷新；
- 单个服务失败不会阻止其他服务更新；
- 失败时可保留最近一次成功快照，并清楚标记缓存或不可用状态。

## 通知

70% 和 90% 阈值通知只作用于存在明确上限的比例指标。AI Token Meter 会记录已发送阈值，避免在同一额度周期内重复轰炸；额度重置后可重新触发。

DeepSeek 的余额金额本身不触发额度阈值；其余额基准环是本地参考可视化。
