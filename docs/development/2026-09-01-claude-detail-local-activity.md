# Claude 专用详情与本机活动开发验收日志

> **后续变更：** 2026-09-02 按 `REQ-20260901-008` 精简展示，Token composition 与 Top models 两张卡片已从详情页移除；本日志以下内容保留原始实施和验收事实。参见 [精简日志](2026-09-02-claude-detail-card-removal.md)。

**日期：** 2026-09-01
**需求：** `REQ-20260901-006`
**结果：** 已完成并安装验收

## 背景与目标

Claude 原来只使用通用紧凑详情，信息密度明显低于 Codex 与 DeepSeek。个人 Claude Pro/Max 没有适合本应用直接使用的稳定跨 Web、Desktop、移动端和多设备 30 天明细接口，因此本阶段采用两层口径：上方保留 Claude Code `/usage` 官方额度，下方补充明确标记为 **Last 30 days · This Mac** 的本机 Claude Code 活动。

本机活动只用于解释这台 Mac 上 Claude Code 的使用情况，不参与官方额度百分比，也不推算账户总额度。

## 实现范围

- 新增 `ClaudeDailyActivity`、`ClaudeModelActivity`、`ClaudeLocalActivitySummary`，并让安全快照、缓存、刷新协调与 App 状态显式保留聚合结果。
- 新增 `ClaudeLocalActivityReader`，只读扫描 `~/.claude/projects/**/*.jsonl`。
- 官方 `/usage` 与本机读取并行执行；官方失败立即返回，本机最多等待 2 秒，失败或超时只让本机区域显示不可用，不改变官方额度结果。
- 新增 390 pt 宽的 Claude 专用详情页，包含：
  - Current session 与 Weekly 官方额度；
  - Sessions、Active days、Total tokens；
  - 固定 30 个自然日的每日 Token 柱图；
  - Input、Output、Cache 构成；
  - Token 数最多的三个模型及各自占比。
- 零 Token、未知或不安全的模型标识不进入 Top models；无活动时显示明确空状态。
- 详情高度按屏幕可用高度自适应；标题与官方额度固定可见，只有本机区域可滚动。原有置前、自动隐藏、外部点击关闭与左右贴边行为保持不变。

## 数据与隐私边界

Reader 的 JSON 解码结构只声明以下白名单字段：

- `timestamp`；
- `sessionId`；
- `message.model`；
- `message.usage` 中的 input、output、cache creation 与 cache read 数字。

提示词、回复、项目路径、分支、文件名、会话标题和命令不会进入领域模型、缓存或日志。读取器按 64 KiB 分块流式读取，只扫描 30 日窗口内最近修改的文件，并跳过符号链接、损坏记录、负计数、超过 2 MiB 的单行及超过 256 MiB 的文件；单次扫描最多处理 4096 个文件、512 MiB 和 10 秒。模型标识限长 80 UTF-8 字节，只接受字母、数字和 `-._:/`，旧缓存解码时同样重新规范化。主会话按 `sessionId` 去重；`subagents` 的 Token 可以计入活动，但不虚增主会话数。

## 测试先行证据

开发过程分别以编译失败或断言失败证明测试先于实现存在，再逐项完成：

- 聚合模型与快照传递；
- 30 日边界、时区、空目录、损坏记录、超限行和符号链接；
- 主会话与 subagent 计数规则；
- 官方成功/本机失败的独立降级；
- 官方失败立即返回、本机读取 2 秒超时与单次在途保护；
- 旧缓存模型规范化、不安全模型过滤、最近修改时间剪枝与流式读取合同；
- 自适应面板尺寸与专用详情路由；
- 零 Token 模型过滤、显式空状态、模型占比和官方/本机无障碍分区。

最终完整命令使用隔离 SwiftPM/Clang 缓存执行，结果为：

- **295 项测试、58 个测试组通过；**
- **0 个失败；**
- 安装型 CLI 冒烟测试中的 3 项环境门控测试按设计跳过，不属于失败。

静态检查结果：

- `git diff --check` 通过；
- Reader 未声明 prompt、cwd、git branch、slug 等正文或项目字段；
- 源码和构建资源未新增 TTF、OTF、WOFF 或 WOFF2 字体文件。

## Release、安装与真实数据验收

以 `AI_METER_INCLUDE_WIDGET=0` 生成不含未签名 Widget 的正式候选包。验证结果：

- `codesign --verify --deep --strict` 通过；
- 主程序为 arm64 Mach-O；
- Bundle 版本为 0.1.0（build 1）；
- 候选版和 `/Applications/AI Token Meter.app` 主程序 SHA-256 均为 `b958f72c6da409f37c841b3cd9fdfb39dbc0f6b5fdad0a5dd2761afe37c4df45`；
- 覆盖安装前版本已保存在 `/private/tmp/AI Token Meter-pre-final-review-20260901-2321.app`，可用于本机临时回退。

最终安装版刷新后的真实 Claude 快照为：

- Official current session：0%；
- Official weekly：0%；
- 本机窗口：30 天；
- Sessions：7；
- Active days：4；
- Input：542；
- Output：140,404；
- Cache：15,086,463；
- Total：15,227,409；
- Top models 仅保留 `claude-sonnet-5` 与 `claude-haiku-4-5-20251001`，内部 `<synthetic>` 零 Token 行已消失。

通过已安装 App 的辅助功能状态确认：点击 Claude 后状态从 `Detail closed` 变为 `Detail open`，8 秒后自动回到 `Detail closed`；浮动条保持右侧、50% 高度。Claude 详情使用非激活 `NSPanel`，当前自动化只捕获到浮动条而不能完整截取详情内容，因此本日志不把缺失的整页截图冒充视觉证据；布局、内容与可访问性由自动化测试和真实快照共同覆盖。

## Git 节点

- `274c53c`：增加 Claude 本机活动快照数据；
- `2256677`：聚合本机 Claude Code 活动；
- `48bd60d`：把可选本机活动接入 Claude 采集；
- `75d4e97`：实现丰富 Claude 专用详情；
- `0831d49`：隐藏零 Token 模型行；
- `e56c6bc`：落实独立审查提出的资源边界、超时、缓存规范化、布局、空状态与无障碍加固；
- `f60a3a7`：复审闭环目录枚举和底层取消边界，补回额度重置时间无障碍信息，并修复极端计数占比；
- `d32fbaa`：通过最终复审后合入 `main`。

## 已知边界

- 本机统计不包含 Claude Web、Desktop、移动端、其他 Mac 或其他设备；
- 没有本机目录与目录不可读是两种不同状态，后者不会伪装成零；
- Widget 真实签名与 Gallery 验收仍按 `REQ-20260901-003` 延期，不属于本阶段范围。
