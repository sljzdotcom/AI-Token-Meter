# AI Token Meter Claude 详情与本机活动设计规格

> **2026-09-02 展示变更：** 本规格最初设计的 Token 构成和模型排行卡片已由 `REQ-20260901-008` 取消，内联隐私提示随后由 `REQ-20260902-009` 移除。当前详情显示官方额度、本机会话数、活跃日、Token 总量和每日趋势；底层聚合字段与隐私保护继续保留。参见 [卡片精简规格](2026-09-02-claude-detail-card-removal-design.md)和[隐私说明移除规格](2026-09-02-claude-detail-privacy-note-removal-design.md)。

**日期：** 2026-09-01  
**状态：** 已实施并验收
**范围：** Claude 详情窗口、Claude 本机活动采集与展示  
**需求：** `REQ-20260901-006`

## 1. 背景与数据边界

Claude 详情目前复用通用紧凑视图，只展示 `/usage` 返回的当前会话额度、每周额度和重置时间。Codex 与 DeepSeek 已有专用详情，因此 Claude 在信息密度和视觉层级上明显偏弱。

Anthropic 没有向个人 Pro/Max 订阅公开稳定的跨 Claude Web、Desktop、移动端和 Claude Code 的 30 天明细接口。官方组织分析与 Admin Usage API 面向 Team、Enterprise 或 Console/API 组织，不能代表个人订阅总量。因此本功能严格区分两种口径：

1. **Official quota**：继续通过 Claude Code `/usage` 获得个人订阅的当前额度窗口与重置时间；
2. **Last 30 days · This Mac**：只聚合当前 Mac 上 Claude Code 本地记录中的结构化计量元数据，不声称是官方总量。

## 2. 目标

1. Claude 详情页采用和 Codex 一致的“额度优先、活动补充”信息架构；
2. 官方额度始终置顶，并清楚显示当前会话和每周窗口；
3. 增加当前 Mac 最近 30 天的 token、会话、活跃日和每日趋势；
4. 本机活动不可读时不影响官方额度刷新，也不把缺失显示为零；
5. 不读取、保存、展示或上传聊天正文、提示词、回复、文件路径和项目名称。

## 3. 详情页布局

新增独立的 `ClaudeDetailView`，不再使用 `compactDetail`。详情窗口宽度为 390 pt，高度依据可见屏幕在 500–590 pt 范围内自适应；内容超出时只滚动本机活动区域，标题和官方额度保持可见。

从上到下为：

1. **标题区**：Claude Logo、`Claude`、副标题 `Official quota · Local activity`、主要额度百分比；
2. **Official quota**：当前会话与每周额度两张卡片，各自显示百分比、进度条和重置时间；
3. **Last 30 days · This Mac**：
   - 总 Token；
   - Sessions；
   - Active days；
   - 30 根每日 token 柱形趋势；
4. **口径说明**：`Aggregates Claude Code activity readable on this Mac only.`；
5. **页脚**：采集状态及更新时间。

趋势图没有数据时显示明确的空状态，不绘制伪造柱形。模型名称采用安全短名，只来自 `message.model`，不显示项目或会话名称。

## 4. 数据模型

在补充数据模型中增加：

- `ClaudeDailyActivity`：日期、input、output、cache token；
- `ClaudeModelActivity`：模型 ID、token 数；
- `ClaudeLocalActivitySummary`：30 天每日数组、会话数、活跃日、token 构成、模型汇总、更新时间。

`UsageSnapshot` 增加可选的 `claudeLocalActivity`。该字段必须被现有脱敏复制、缓存快照与刷新协调路径显式保留；它只包含聚合数字和模型 ID，不包含内容字段。

总 Token 定义为：

`input_tokens + output_tokens + cache_creation_input_tokens + cache_read_input_tokens`

所有计数使用带溢出保护的 `Int64`；负数、非数字和无法识别的记录被忽略。

## 5. 本机读取与隐私

新增 `ClaudeLocalActivityReader`，从 `~/.claude/projects` 中枚举最近 30 天内更新的 Claude Code JSONL 会话文件：

- 只解析 `timestamp`、`message.model` 和 `message.usage` 白名单字段；
- 未知 JSON 字段全部忽略；
- 主会话文件计入 session 数，`subagents` 文件只计入 token 与每日活动，避免把子代理重复算作用户会话；
- 逐文件、逐行处理并设置单行及单文件安全上限；异常文件跳过，不中断其他文件；
- 不记录原始行，不把任何正文写入日志或缓存；
- 不访问 Claude Web/Desktop 数据，不发起网络请求。

读取在 utility 优先级后台任务执行。官方 `/usage` 和本机活动并行采集；本机读取失败时 `claudeLocalActivity = nil`，官方快照照常返回。

## 6. 错误与状态

- `~/.claude/projects` 不存在或不可读：本机区域显示 `Local Claude activity is unavailable; official quota data is unaffected.`；
- 最近 30 天无记录：显示合法的零活动空状态，区别于不可用；
- 单个文件损坏或过大：跳过并继续，不暴露文件名；
- Claude 未登录或 `/usage` 失败：沿用现有登录/错误状态；本机统计不能伪装成官方额度成功；
- 日期按当前系统日历和时区归档，窗口包含今天在内共 30 个自然日。

## 7. 非目标

- 不抓取 Claude 网页、Desktop 或移动端历史；
- 不接入需要 Admin Key 的组织 Usage API；
- 不估算 Pro/Max 额度的 token 上限或把本机 token 换算成官方百分比；
- 不展示提示词、回复、项目、文件、命令或会话标题；
- 不改变浮动条圆环的官方额度口径。

## 8. 测试与验收

自动化测试覆盖：

1. 最小 JSONL 固件只抽取白名单计量字段；正文中的数字不被误算；
2. 30 天边界、时区、活跃日、session 与 subagent 规则；
3. input/output/cache/token 总数、模型聚合和整数溢出；
4. 损坏、超限、空文件和不可读目录的降级；
5. 官方采集成功但本机失败时仍返回官方快照；
6. `UsageSnapshot` 的脱敏、复制和缓存路径保留聚合数据；
7. Claude 详情可访问性标签明确区分 Official 与 This Mac；
8. 390 pt 宽度、较矮屏幕和长模型名下无截断或越界。

真实验收必须确认：Claude 详情信息密度与 Codex 接近；官方额度与 Claude 客户端一致；本机区域明确标记 `This Mac`；关闭、自动隐藏、置前和字体偏好行为无回归。
