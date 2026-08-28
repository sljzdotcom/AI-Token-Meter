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

## 任务 4：DeepSeek 余额与 Keychain

### 协议核对

- 以 DeepSeek 官方余额文档为准：`GET https://api.deepseek.com/user/balance`，Bearer 认证，响应包含 `is_available` 与 `balance_infos`。
- 官方文档：`https://api-docs.deepseek.com/api/get-user-balance/`。
- 官方错误码文档：`https://api-docs.deepseek.com/quick_start/error_codes/`；401 映射认证失败，429 映射限流，500/503 等映射传输失败。

### 红灯证据

- 首次运行新增测试时编译失败，明确报告 `DeepSeekClient`、`DeepSeekBudget` 与 `KeychainStore` 不存在。
- 为采集器补充测试后再次编译失败，明确报告 `DeepSeekCollector` 不存在。
- 受限执行环境中的真实 Keychain 调用返回 `errSecParam (-50)`；相同测试在正常用户权限环境通过，证明是测试沙盒限制而非查询实现错误。真实 Keychain 生命周期测试因此改为通过 `AI_METER_RUN_KEYCHAIN_TESTS=1` 显式启用。

### 实现与安全边界

- 余额客户端只向固定官方地址发 GET 请求，并只读取官方 JSON 字段；错误响应正文不进入错误或日志。
- 多币种响应按设置选择 CNY 或 USD。官方余额使用 `.balance` 且没有 `limit`，因此不会伪造“已使用百分比”。
- 可选月度预算必须同时提供本地累计消费，生成独立 `.localBudget` 指标；余额变化不会被当成本月消费。
- `DeepSeekCollector` 通过 `SecretStore` 读取密钥；缺失或空密钥直接返回 `.authenticationRequired`，不会产生网络请求。
- 生产 Keychain service 固定为 `com.millerpan.AIMeter.deepseek`，使用 `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`，密钥不随 iCloud Keychain 同步。
- Keychain 生命周期测试使用随机 service 名，覆盖新增、替换、读取和删除，并在测试结束时清理。

### 绿灯证据

- DeepSeek HTTP、预算和采集器：9/9 测试通过；包含空密钥在发起网络请求前即被拒绝的回归测试。
- 正常用户权限下的 Keychain 隔离集成测试：1/1 通过。
- 全量测试：31 个测试、9 个测试组通过，0 个失败；Keychain 和 3 个本机 CLI 集成测试在普通测试中按设计跳过。
- 完整 debug 构建：退出码 0，`Build complete`。

### 提交

- 计划提交：`feat: add DeepSeek balance and secure key storage (task 4)`。

## 任务 5：缓存、并发刷新与阈值判断

### 红灯证据

- 缓存测试首次编译失败，明确报告 `SnapshotCache` 不存在。
- 刷新协调测试首次编译失败，明确报告 `RefreshCoordinator` 不存在。
- 阈值测试首次编译失败，明确报告 `ThresholdEvaluator` 不存在。
- 初版把人类可读倒计时作为周期标识；新增“51 分钟→50 分钟”测试后出现重复 warning，准确证明该逻辑会制造重复通知。

### 实现与关键决定

- `SnapshotCache` 使用带版本号的 JSON envelope 和原子文件替换；损坏 JSON 被视为空缓存，时间戳原样保存，调用方可准确判断过期。
- `RefreshCoordinator` 为 actor：三服务并发采集，输出固定按 Claude、Codex、DeepSeek 排序；重叠刷新共享同一个 Task，不会重复调用 CLI/API。
- 单服务失败不会阻塞其他服务；存在上次成功快照时返回 `.cached` 并附去敏行动提示，不存在缓存时返回无指标的稳定错误状态。
- 只有成功快照会更新持久缓存；错误状态不会覆盖最后一份好数据。
- 阈值规则为 70% warning、90% critical；一次从 65% 跳到 93% 只发 critical。相同周期不重复通知，使用率跌到 10% 以下或可靠的 `resetAt` 变化后重新解锁。
- `resetDescription` 不参与周期身份计算，因为动态倒计时文本每分钟变化，不能代表真实的新周期。
- 无上限余额指标和缓存快照不会触发用量警报。

### 绿灯与稳定性证据

- 缓存：3/3 测试通过。
- 并发刷新：4/4 测试通过；三个 0.2 秒采集器的组合刷新约 0.21–0.23 秒完成。
- 阈值：7/7 测试通过。
- 一次全量测试在构建后无输出停滞并被人工中止；新模块组合测试 14/14 通过，随后完整套件连续三次 45/45 通过（约 0.96–1.13 秒），未复现停滞。
- 完整 debug 构建：退出码 0，`Build complete`。

### 提交

- 计划提交：`feat: coordinate refresh cache and usage alerts (task 5)`。

## 任务 6：菜单栏、悬浮条与系统设置

### 红灯与诊断证据

- 展示模型测试首次编译失败，明确报告 `ProviderPresentation`、`MenuBarSummary` 和颜色语义不存在。
- DeepSeek 本地预算测试先覆盖余额减少、充值和跨月重置，再实现持久追踪器；避免把余额本身错误地显示为“本月已用”。
- 初版在同一个 `NSPanel` 中动态改变悬浮条和详情卡尺寸；点击圆环时 AppKit 在 `_postWindowNeedsUpdateConstraints` 触发 `SIGTRAP`。崩溃报告和稳定复现共同定位到非激活透明面板的动态约束更新。
- 改为固定 84×300 悬浮条与固定 262×190 详情卡两个独立面板后，重复展开、收起未再复现崩溃。

### 实现与关键决定

- `MenuBarExtra` 提供三服务摘要、手动刷新、更新时间、设置和退出入口；菜单栏值使用所有有界指标中的最高风险。
- 右侧悬浮条使用三个高对比细圆环；Claude、Codex 展示用量百分比，DeepSeek 展示真实余额，点击后在左侧展开独立详情卡。
- 设置保存悬浮条、通知、登录启动与本地月预算等非敏感偏好；DeepSeek API Key 只经 `SecretStore` 写入 Keychain。
- 5 分钟自动刷新在唤醒后立即补刷；重叠刷新仍由核心协调器合并。
- 通知载荷只保存提供商标识，用户点击后路由至对应详情卡；不把原始响应或凭证写进通知。
- 增加只读 `AI_METER_DEMO_MODE=1` 视觉验收入口：仅装载固定示例快照，不读取 Keychain、不调用 CLI、也不访问网络。

### 绿灯与界面证据

- 展示模型与本地预算新增测试全部通过；完整套件为 52 个测试、14 个测试组，0 个失败。
- 完整 debug 构建退出码 0，`git diff --check` 退出码 0，源码与文档的常见 API Key、Bearer 和 Telegram Token 形态扫描无匹配。
- 原生 App 演示模式实机验证通过：设置页全部内容可见；悬浮条开关可即时隐藏并恢复；三个圆环具备可访问名称；Claude 详情可反复展开与收起且进程稳定。

### 提交

- 计划提交：`feat: add menu bar floating meter and settings (task 6)`。
