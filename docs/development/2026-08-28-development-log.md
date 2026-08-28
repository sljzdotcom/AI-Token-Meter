# AI Meter 开发日志

## 记录约定

- 日期与时区：2026-08-28，Asia/Singapore。
- 分支：`feat/initial-app`。
- 工作区：`.worktrees/initial-app`。
- 每个任务记录：目标、红灯证据、绿灯证据、关键决定、已知限制、提交哈希。
- 日志不记录 API Key、OAuth 令牌、Authorization 请求头或未经去敏的 CLI 原始输出。

## 环境基线

- macOS：26.6.2（Build 25G83）。
- Xcode：26.6（Build 17F113）。
- Swift：6.3.3，arm64-apple-macosx26.0。
- Claude Code：2.1.241，路径 `/Users/millerpan/.local/bin/claude`。
- Codex CLI：0.149.0-alpha.4.3，路径 `/Users/millerpan/.local/bin/codex`。
- Git 基线提交：`d9bc9a8 chore: prepare isolated development workspace`。

## 规划节点

- 已读取并执行 `writing-plans`、`test-driven-development`、`using-git-worktrees` 与 `verification-before-completion` 流程。
- 采用 Swift Package + SwiftUI 可执行程序，避免依赖未安装的 XcodeGen。
- App 最终由可重复脚本封装为本机签名的 `.app`；核心逻辑保持可由 `swift test` 独立验证。
- 详细计划：`docs/superpowers/plans/2026-08-28-ai-meter-implementation.md`。

## 任务 1：工程骨架与统一领域模型

### 目标

- 建立 `AIMeterCore`、`AIMeterApp` 和 `AIMeterCoreTests` 三个 Swift Package 目标。
- 定义提供商、指标、快照、采集状态和采集器协议。
- 证明百分比不会在缺少有效上限时被伪造，并证明快照过期判断有效。

### 红灯证据

- 初次执行被用户级 SwiftPM/Clang 缓存权限阻止；这不是有效的行为测试失败。
- 将 cache、configuration、security、scratch 和 module cache 改到 `/private/tmp/ai-meter-spm`，并为 SwiftPM 关闭其内部二次沙盒。
- 有效红灯：`swift test --filter UsageModelsTests` 退出码 1，编译器报告 `cannot find 'UsageMetric' in scope` 和 `cannot find 'UsageSnapshot' in scope`。

### 绿灯证据

- `UsageModelsTests`：3 个测试、0 个失败。
- `swift build`：退出码 0，`Build complete`。

### 关键决定

- SwiftPM 构建产物放在 `/private/tmp/ai-meter-spm`，避免 Dropbox 目录对高频临时文件出现 I/O 问题。
- `usedFraction` 仅在 `limit > 0` 时存在，并限制到 `0...1`。
- 错误状态使用稳定枚举；用户可见的去敏说明单独保存在 `statusMessage`。
- 提交信息：`feat: scaffold AI Meter domain and app (task 1)`。

## 任务 2：终端净化与 Claude/Codex 解析器

### 红灯证据

- 解析器测试首次运行退出码 1。
- 编译器分别报告 `ANSITextSanitizer`、`ClaudeUsageParser` 和 `CodexUsageParser` 不存在，证明测试针对尚未实现的行为。

### 绿灯证据

- 第一轮最小实现后 6 个解析行为通过，ANSI 测试准确抓住 `CRLF` 被展开成两个换行的问题。
- 将连续终端换行归一化后，相关测试 7/7 通过。

### 覆盖边界

- ANSI CSI、OSC、回车覆盖和退格字符。
- Claude 英文与中文已使用比例。
- `remaining`、`left`、`剩余` 向已使用比例的反向换算。
- 行内和后继行重置说明。
- 不含额度指标的输出必须抛出 `unrecognizedOutput`，不能伪造 0%。

### 关键决定

- Claude 与 Codex 使用独立公开入口，共享内部 `TerminalUsageParser`，保证解析规则一致且 UI 不接触原始文本。
- 重置时间第一版保留官方文本；只有数据源能可靠提供绝对时间时才填 `resetAt`。
- 提交信息：`feat: parse Claude and Codex usage output (task 2)`。

## 任务 3：受控 CLI 采集器

### 红灯与诊断证据

- `/usr/bin/script` 在非交互管道中先向子进程发送 EOF，无法稳定驱动全屏 CLI，因此改用原生 `openpty`。
- Codex TUI 在初版 PTY 下逐字竖排。测试脚本确认终端尺寸为 `0 0`；为 PTY 显式设置 120 列 × 40 行后恢复正常。
- 只关闭父进程不能回收继承 PTY 的子进程；专用 fixture 会忽略 `TERM/HUP` 并保持描述符打开，初次测试耗时约 3.97 秒，超过 1.5 秒上限。
- 本机 Codex 在受限测试环境中报告 `state_5.sqlite` 只读；提升为正常用户环境后证实数据库完好，问题是沙盒权限而非用户数据损坏。
- Claude `auth status` 在未登录时仍输出有效 JSON，但退出码为 1。初版只在退出码 0 时解析，导致误入交互模式并超时；新增非零退出码 fixture 后测试准确复现为 `.transportFailure`。

### 实现与关键决定

- `PTYCommandRunner` 使用原生 `openpty`、固定 120×40 窗口、非阻塞读取和 10 ms 轮询；超时关闭 PTY、终止父进程，并在 0.5 秒宽限后强制回收。
- Claude 先调用 `claude auth status`。只要机器可读 JSON 明确 `loggedIn: false`，无论命令退出码是否为零，都返回 `.authenticationRequired`；不会启动聊天或发送模型提示。
- Codex 不再依赖 `/status` 的全屏界面，改用官方 `codex app-server` JSON-RPC：依次完成 `initialize`、`initialized`、`account/rateLimits/read`，直接读取 primary/secondary 的 `usedPercent` 与重置时间。
- 保留旧的 Codex 文本解析器用于兼容性测试，但默认采集路径使用结构化接口。
- 所有诊断和测试日志只记录状态与计数，不记录 CLI 原始账户信息、API Key 或登录令牌。

### 绿灯证据

- 子进程持有 PTY 的超时测试：约 0.66 秒通过。
- 执行器与隔离 CLI 采集器：8/8 测试通过。
- 本机只读烟雾测试：3/3 通过；Claude 认证状态可读，未登录被识别为可行动状态，Codex 成功返回额度快照。
- 全量测试：21 个测试、7 个测试组通过，0 个失败；3 个需显式启用的本机烟雾测试在常规测试中按设计跳过。
- 完整 debug 构建：退出码 0，`Build complete`。
- `git diff --check`：退出码 0；针对常见 API Key、Bearer Token 与 Telegram Bot Token 形态的源码/测试/文档扫描无匹配。

### 提交

- 计划提交：`feat: collect usage through authenticated CLI interfaces (task 3)`。
