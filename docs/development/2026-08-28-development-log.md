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

## 任务 3：CLI 采集器（暂停检查点）

### 已完成

- 原生 `openpty` 执行器已通过真实测试脚本验证：固定输入、子进程退出码和超时终止均有效。
- Claude/Codex 采集器与真实解析器的隔离集成测试通过。
- `/usr/bin/script` 方案已根据真实失败证据弃用：它在非交互管道中会先向子进程发送 EOF。

### 重启后继续处理

- 本机 Claude CLI 的真实烟雾测试在 10 秒后超时，原因是 `/usage` 弹层需要分阶段关闭后再退出。
- 本机 Codex CLI 的真实烟雾测试快速返回非零状态，需要确认启动参数、当前工作区信任状态或 `/status` 输入时序。
- 下一步为 `CommandRequest` 增加带延时的输入事件，或者采用 CLI/app-server 已提供的只读状态接口；保持不发送模型提示、不读取令牌。

### 验证状态

- 隔离执行器与 fake CLI：5/5 测试通过。
- 本机真实 CLI 烟雾测试：0/2，通过前不得把任务 3 标记完成。
