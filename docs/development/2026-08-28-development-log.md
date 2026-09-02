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
- Git 基线提交：`c6f8992 chore: prepare isolated development workspace`。

## 规划节点

- 已读取并执行 `writing-plans`、`test-driven-development`、`using-git-worktrees` 与 `verification-before-completion` 流程。
- 采用 Swift Package + SwiftUI 可执行程序，避免依赖未安装的 XcodeGen。
- App 最终由可重复脚本封装为本机签名的 `.app`；核心逻辑保持可由 `swift test` 独立验证。
- 详细计划：`docs/design/implementation-plans/2026-08-28-ai-meter-implementation.md`。

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
- 改为固定 84×300 悬浮条与固定 262×224 详情卡两个独立面板后，重复展开、收起未再复现崩溃。

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

## 任务 7：隐私回归、打包与文档

### 红灯与诊断证据

- 隐私回归测试首次编译失败，明确报告 `SensitiveTextRedactor` 不存在；测试覆盖缓存 JSON、展示文本和 Authorization/Bearer 形态。
- 第一版打包脚本使用项目内 `.build`。当前项目位于 Dropbox 同步目录，release 中间文件读取出现 I/O 错误；将 SwiftPM 构建与状态缓存默认迁移到本机临时目录后通过，最终 App 仍输出到项目 `dist`。
- 当前桌面在最终签名包视觉复核时处于锁屏状态，自动界面读取被系统拒绝；签名包本身成功启动并稳定运行，停止前无崩溃。相同源码的 debug App 已完成悬浮条、详情、设置和开关实机视觉回归。

### 实现与关键决定

- `SensitiveTextRedactor` 在快照落盘与展示前清除 Bearer Token，以及 `sk-` / `dk-` 常见密钥形态；正常的行动提示保持不变。
- 正式打包脚本使用严格 shell 选项、隔离的本机构建缓存、release 二进制、固定 Info.plist、临时本机签名和签名自验证；不打印环境变量或凭证。
- README 记录系统要求、构建、首次配置、指标语义、隐私边界、故障排查、诊断位置和卸载流程。
- 最终视觉补充主、次额度的重置说明和快照更新时间；悬浮详情保持固定独立面板，避免重新引入动态约束崩溃。

### 最终验收证据

- 隐私专项：3/3 通过；重置说明专项：5/5 通过。
- 完整测试、production 构建、`scripts/build-app.sh`、Info.plist 校验与 `codesign --verify --deep --strict` 均退出码 0。
- 最终产物为 arm64 Mach-O：`dist/AI Meter.app`；App 包包含 release 可执行文件、Info.plist 和 `_CodeSignature/CodeResources`。
- `git diff --check` 通过；仓库工作区和全部 Git 补丁按常见 API Key、Bearer、Telegram Bot Token 形态扫描，不含真实凭证。

### 规格完成审查

- 已实现：三服务采集、5 分钟并发刷新、手动刷新、缓存降级、70%/90% 去重通知、通知点击路由、Keychain、本地月预算、登录启动、睡眠唤醒补刷、菜单栏、右侧悬浮条、设置、深浅色系统外观、可访问标签、本机打包与签名。
- 有意调整：Codex 使用官方结构化 `app-server` 用量接口，替代解析 `/status` 全屏界面；可靠性高于原设计方案且不读取登录令牌。
- 部分实现：多显示器和系统缩放已实现重定位但尚无自动化 UI 矩阵；通知权限拒绝与登录项系统错误有非阻塞处理但尚无端到端系统自动化；DeepSeek 第一版显示总余额与本地预算，未单独展示赠送余额和充值余额。
- 第一版无阻塞项；以上部分项可作为后续增强，不影响已确认的本地用量监控主流程。

### 独立代码审查与修复

- 独立审查最初判定“修完再合”，指出 PTY 父进程提前退出、Codex 忽略 SIGTERM、DeepSeek 未设置请求超时、通知状态未持久化，以及官方账户不可用却显示正常等问题。
- PTY 现在在直接子进程结束后发出停止读取请求，并有界排空已到达的输出，再关闭 master descriptor；新增父进程立即退出、独立后代继续持有 slave 6 秒的回归 fixture，采集约 0.05–0.10 秒返回，不再等到后代退出或整体超时。
- Codex 进程控制器同时持有输入/输出管道；超时关闭管道、发送 TERM，并在 0.5 秒后对仍运行进程发送 KILL。忽略 TERM 的 fixture 在约 0.11 秒返回超时错误，后台强制回收由有界宽限执行。
- DeepSeek 请求明确设置 `timeoutInterval = 10`；测试直接检查生产请求对象的 10 秒限制。
- `ThresholdEvaluator` 支持 Codable；AppModel 从 UserDefaults 恢复并在每次启用通知的评估后保存，因此同一额度周期在重启后仍保持去重。
- 展示语义现在优先尊重官方 `availability`；`is_available=false` 会保留余额但显示不可用颜色与提示，菜单栏最高风险不再使用不可用账户的百分比。
- 清理全部历史变更中的 EOF 多余空行，`git diff --check c6f8992` 退出码 0。
- 审查确认当前打包方式只适合本机：README 已明确 Apple Silicon、arm64 与 ad-hoc 签名范围；正式外部分发需另做 Developer ID、Hardened Runtime、公证和可选 universal binary。
- 首轮修复后的全量套件为 61 个测试、15 个测试组，0 个失败；另外显式启用的 3 项真实 CLI 烟雾测试与 1 项隔离 Keychain 生命周期测试为 4/4 通过。
- 最终 production 重建、App 打包、Info.plist 校验、严格签名验证及从基础提交开始的完整 `git diff --check` 均通过。

### 超时竞态的追加稳定性修复

- 提高父进程退出测试的调度容差后进行重复验证，测试进程再次无输出停滞。进程采样明确显示线程永久停在 `PTYCommandRunner.swift` 的 `Process.waitUntilExit()`；系统进程表中对应子进程已经消失，因此问题不是 fixture 仍在运行。
- 进一步定位到两个独立竞态：其一是超时任务可能早于 `RunningProcessBox.set` / `CodexProcessBox.set`，导致停止请求被永久遗漏；其二是极短生命周期子进程可能在 `waitUntilExit()` 建立有效等待前已经退出。
- 两个进程控制器现在先持久记录停止意图；如果进程随后才登记，会立即重放关闭管道、TERM 和有界 KILL 流程。清理时不会重置停止意图，避免迟到登记重新失效。
- 新增 `ProcessTerminationWaiter`，在 `Process.run()` 之前登记 termination handler，再通过一次性同步信号等待退出状态；PTY 与 Codex 都不再依赖会遗漏瞬时退出的 `waitUntilExit()`。
- 新增确定性登记屏障：PTY 与 Codex 测试均在 `Process.run()` 后、控制器登记前阻塞 0.2 秒，并在 0.05 秒触发超时；随后释放屏障，验证此前停止意图被重放并在 1 秒内有界返回，不再依赖微秒级调度概率。
- Codex 忽略信号 fixture 在 `exec` 前设置 `trap '' TERM HUP`，确保 Python 进程继承忽略状态；fixture 通过仅供测试的环境覆盖写出唯一 PID。普通超时和登记前超时两项测试都轮询至 `kill(pid, 0)` 返回 `ESRCH`，分别证明 0.5 秒 KILL 兜底与迟到登记后的停止重放最终回收了后台进程。
- 最终全量套件连续 3 次通过：每次 63 个测试、15 个测试组、0 个失败；显式真实集成测试再次 4/4 通过。
- 追加修复后重新完成 production 构建、App 打包、Info.plist 校验、arm64 架构检查、严格签名验证和从基础提交开始的完整 `git diff --check`，全部退出码为 0。

### 提交

- 计划提交：`release: package and document AI Meter v0.1.0 (task 7)`。

## 2026-08-29：Claude 登录后仍显示未登录

### 根因证据

- 用户在 Claude Code 2.1.241 中完成 OAuth 登录后，正常用户环境的 `claude auth status` 返回 `loggedIn: true`，且系统仅发现 `~/.local/bin/claude` 这一份安装。
- AI-Meter 于登录后自动刷新，Codex 与 DeepSeek 同次更新成功，但成功缓存中仍没有 Claude；界面继续显示 `Sign in`，排除旧版本、未刷新和全局刷新故障。
- 用 AI-Meter 的最小子进程环境直接调用同一 Claude CLI，稳定返回 `loggedIn: false`。逐项恢复非敏感环境变量时，仅加入 `USER=millerpan` 即恢复 `loggedIn: true`；`LOGNAME`、`TMPDIR` 或 locale 单独加入均无效。
- Finder 启动的 AI-Meter 父进程实际包含 `USER=millerpan`，断点因此明确位于 `PTYCommandRunner.controlledEnvironment()`：安全白名单在启动 Claude 子进程时丢弃了 `USER`，导致 Claude 无法找到当前用户的 OAuth 凭据。

### 测试驱动修复与验证

- 先增加真实 PTY fixture 行为测试，要求子进程看到 `USER`。修复前测试失败并输出 `user:missing`，证明测试能够捕获本次缺陷。
- 最小修复只把 `USER` 加入现有环境白名单，不扩大到完整父进程环境，也不改变凭据、缓存或网络逻辑；修复后同一测试输出当前用户名并通过。
- 完整套件更新为 64 个测试、15 个测试组，0 个失败；production App 重新构建并通过打包脚本的严格签名验证。
- 修复版已替换 `/Applications/AI Meter.app`。自动启动会同时触发已配置 DeepSeek 的真实余额请求，按敏感凭据外发规则等待用户明确授权后再完成界面验收。

## 2026-08-30：悬浮详情的自动收起与外部点击关闭

### 红灯证据

- 新增“详情自动隐藏时长”测试后，尚未实现的阶段编译器报告 `cannot find 'DetailAutoHideInterval' in scope`；这证明固定选项、默认值和持久化回退的测试先于实现存在。
- 新增共享详情会话与点击区域测试后，尚未实现的阶段编译器报告 `FloatingDetailSession` 与 `FloatingPanelHitPolicy` 不存在；测试先锁定了单一选中项、失效定时器和两个面板外点击的行为。

### 最小实现与监听策略

- 自动收起设置只接受 3、5、8、15、30 秒，缺失或无效持久化值安全回退到默认 8 秒；`AppModel` 通过 `UserDefaults` 读取并保存该设置，设置页的 Appearance 区域提供选择器。
- 一个共享 `FloatingDetailSession` 管理圆环与通知打开的详情。每次展示都会更新会话代数并取消旧任务，因此旧定时器或同一服务的旧选择不能关闭新的详情。
- AppKit 监听同时注册本地和全局左右鼠标按下事件：本地监听覆盖本 App，返回原事件；全局监听补充其他 App 中的点击。两者都先捕获事件时的会话标识、时间戳和屏幕坐标，再仅在坐标同时位于悬浮条和详情卡之外时关闭。
- 坐标转换对带窗口的本地事件使用 `convertPoint(toScreen:)`；全局事件没有窗口时降级使用其已处于屏幕坐标系的 `locationInWindow`。控制器释放时移除两个 monitor 和屏幕参数观察者。

### 本次自动化验证

- 2026-08-30 11:00 +0800 执行完整 `swift test --disable-sandbox`（隔离 SwiftPM 和 Clang 缓存）：74 个测试、17 个测试组通过，0 个失败。
- 4 个需显式启用的环境相关检查按设计跳过：1 个 Keychain 生命周期测试和 3 个已安装 CLI 烟雾测试。本次没有启用它们，且没有触发外部 DeepSeek 请求。

### 本机界面验收

- 待用户明确授权，未执行。本次没有替换 `/Applications/AI Meter.app`，没有启动 App，也没有读取或外发已保存的 DeepSeek API Key。
- 授权后才可验证五项本机界面行为：默认 8 秒收起、设置时长生效、点击任一详情浮层外立即关闭、面板内点击不误关，以及悬浮条隐藏／恢复后的行为。

### 打包与产物验证

- 2026-08-30 11:01 +0800，`./scripts/build-app.sh` 成功完成 release 构建、工作区 `dist/AI Meter.app` 的 ad-hoc 签名和脚本内签名自验证，退出码 0。
- 随后独立执行 `codesign --verify --deep --strict --verbose=2`、`plutil -lint`、`file` 与 `git diff --check`，均退出码 0；App 包在磁盘上有效、Info.plist 为 OK、可执行文件为 `Mach-O 64-bit executable arm64`，且无空白差异错误。

## 2026-08-30：Claude 专属用量工作区与真实 CLI 修复

### 根因证据

- Claude OAuth 登录本身有效，`claude auth status` 能在受控环境返回已登录；AI Meter 失败并非账号或网络问题。
- Finder 启动的 App 工作目录没有稳定的项目语义。Claude Code 首次进入该目录会展示工作区信任页，原采集器把它当作普通交互并最终超时。
- 为 AI Meter 建立 `~/Library/Application Support/AI Meter/ClaudeUsageWorkspace` 后，人工完成一次官方 Claude 工作区确认；没有自动回答信任问题，也没有使用跳过权限参数。
- 信任完成后的首轮真实采集仍超时。真实终端验证显示 Claude Code 2.1.251 的 `/usage` 页面文案仍含 `Current session`、`Resets` 和周额度，解析器规则有效。
- 最终根因是 PTY 把命令末尾写成 LF；Claude 的全屏输入框需要终端 Enter 对应的 CR。命令文字进入界面但没有提交，因此直到总时限才退出。

### 实现与安全边界

- `CommandRequest` 支持可选工作目录、输入稳定等待、每行终止符和输出停止短语；默认值保持其他调用方原行为。
- Claude 的认证预检与 `/usage` 都运行在专属空目录；交互启动使用 `--ax-screen-reader --safe-mode`，等待 3 秒后以 CR 提交一次 `/usage`。
- 捕获首个 `Resets` 后立即结束只读采集，不再提前排队 `/exit`；总时限为 20 秒，兼顾 CLI 初始化与网络读取。
- 工作区未授权时映射为独立的 `setupRequired` 状态。界面提供一次性设置按钮，生成路径安全转义的本地 `.command` 并交由用户在 Terminal 明确确认。
- 实现不读取 OAuth URL、登录令牌或 Claude 配置内容；开发日志不保存账号、组织、密钥或完整原始终端输出。

### 测试驱动与真实验收

- 工作目录、工作区信任与设置入口均先建立失败测试，再做最小实现；Claude 请求契约继续锁定专属目录、安全参数、单一 `/usage`、3 秒等待、CR 提交、20 秒总时限和 `Resets` 停止短语。
- 回归测试先确认旧请求仍发送 `["/usage", "/exit"]`、没有输入等待且没有 CR 终止符，再分别转绿。
- 修复后 Claude 采集专项 8/8 通过；真实安装 CLI 的 Claude 用量采集在约 4.5–4.7 秒内返回可解析快照。
- 显式真实 CLI 集成套件 3/3 通过：Claude 认证状态、Claude `/usage` 和 Codex 官方 `app-server` 用量读取均成功。
- 全量套件一次冷构建并行运行暴露测试替身的 0.1 秒启动窗口过窄：超时先于 PID 文件写入，后续测试辅助读取报错。专项连续 5 次通过确认生产超时映射未回归；测试窗口调整为 0.5 秒并继续验证 0.5 秒 KILL 兜底和最终 PID 回收。
- 调整后完整套件 85 个测试、18 个测试组通过，0 个失败；4 个需显式环境许可的测试按设计跳过，真实 CLI 另行显式运行并通过。

### 构建、安装与恢复点

- `scripts/build-app.sh` 完成 release 构建、ad-hoc 签名与脚本内验证；独立 `codesign --verify --deep --strict` 和 Info.plist 校验通过。
- 构建产物与 `/Applications/AI Meter.app` 的 `AIMeterApp` 均为 arm64 Mach-O，SHA-256 一致：`2c17ee35bfe16e97d66e2e988c2819396966dd8710787c5022db32c0f1cd0fca`。
- 安装前版本已可恢复地移动到 `/private/tmp/AI Meter.app.pre-cr-fix-20260830-1529`。
- 新 ad-hoc 构建首次读取既有 DeepSeek Keychain 项目时，macOS 重新要求用户确认访问；最终三卡界面验收在该系统确认完成后记录。

## 2026-08-30：Claude、Codex 与圆环用量准确性

### 根因证据

- Claude 2.1.251 的真实 `/usage` 页面在两个 0% 用量窗口之后包含 `+50% weekly limits promo`。通用解析器接受页面上的任意百分比，并沿用上一额度标签，因此促销数字覆盖了真实周额度，缓存错误保存为 50%。
- Codex 官方 `account/rateLimits/read` 顶层通用额度返回每周已使用 5%；同一响应的 `rateLimitsByLimitId.codex_bengalfox` 是 Spark 专用的 5 小时 0% 和每周 0%。旧实现因顶层 secondary 为空，错误地用第一个具有 secondary 的模型专用额度整体替换通用额度。
- `UsageRing` 对 0 或 nil 使用固定的 4% 最小弧长；同时展示模型的弧长取主、次额度最大值，中央文字却固定取主额度，造成视觉与文字可表达不同窗口。

### 测试驱动修复

- Claude 新增脱敏真实结构 fixture。修复前测试明确得到 secondary 0.50；最小修复要求百分比行包含英文 `used/remaining/left` 或中文 `已用/使用/剩余/可用`，促销说明不再生成或覆盖指标。Claude 解析专项 4/4 通过。
- Codex 新增完整结构化响应 fixture：通用周额度 5%，Spark 0%/0%。修复前返回 `5h limit = 0%` 与 secondary 0%；移除模型专用整体替换后，返回唯一 `Weekly limit = 5%`。采集器专项 9/9 通过。
- 展示新增最高窗口一致性与零弧测试。修复前 `ringFraction` 不存在且中央文字仍为主额度；修复后只在官方百分比指标中选择最高窗口，`valueText`、`fraction` 与 `ringFraction` 共用该指标。0%、nil 和余额型指标不绘制前景弧。
- 展示专项 9/9 通过；完整套件 89 个测试、18 个测试组通过，0 个失败。4 个环境相关测试按设计跳过；显式真实 CLI 集成 3/3 通过。

### 构建与安装

- release 构建、ad-hoc 签名、Info.plist 校验和 arm64 架构检查通过。
- 构建产物与 `/Applications/AI Meter.app` 的 SHA-256 均为 `ee68057b88f353b345905942abc044aa73e42f44e903097c38563b5fa58645e5`。
- 安装前版本已移动到 `/private/tmp/AI Meter.app.pre-usage-accuracy-20260830-1547`，可用于恢复。
- 新签名版本首次读取既有 DeepSeek Keychain 项目时由 macOS 要求用户再次确认；确认后执行新鲜快照和三卡视觉验收。

### 最终本机验收

- 用户于 2026-08-30 15:53 +0800 在 macOS 钥匙串窗口选择“始终允许”后，安装版在 15:53:41 写入新鲜缓存；Claude 主、次额度均为 0%，Codex 顶层通用周额度为 5% 且不再附带 Spark 专用 secondary，DeepSeek 余额为 ¥77.99。
- 15:54 +0800 的屏幕验收确认：Claude 0% 仅显示灰色底环，不绘制伪进度；Codex 5% 显示与数值相符的小段绿色弧；DeepSeek 余额只显示金额，不绘制百分比前景弧。
- 至此，促销百分比误识别、Codex 模型专用额度覆盖通用额度、0% 最小伪弧以及文字／圆环窗口不一致四类准确性问题均已按方案 A 闭环。

## 2026-08-30：品牌圆环、Codex 重置券与 DeepSeek 30 天详情

### 实现与隐私边界

- 悬浮条三个圆心均改为本地打包的品牌 Logo，不再显示名称、百分比或余额文字；无障碍描述仍保留服务名、数值和详情状态。
- Claude 与 Codex 继续使用官方额度的已用比例。DeepSeek 改为可配置余额基准，默认 ¥100，圆环显示 `(基准 - 当前余额) / 基准` 的已消耗比例；余额 ¥77.99 对应约 22% 已用弧线。
- Codex 官方 app-server 的同一响应现在同时解码 `rateLimitResetCredits`。应用只保留可用数量、标题和到期日，不解码、不缓存兑换 ID，也不提供兑换按钮。
- DeepSeek 详情新增 620 × 520 原生统计卡，显示最近 30 天费用、API 请求数、Token 数和逐日费用柱状图。首次使用通过内嵌的 `platform.deepseek.com` 官方页面登录；之后复用 macOS WebKit 会话。
- WebKit 桥只接受 HTTPS 官方主机、最多 2 MB 且结构可识别的 JSON，并只输出日期、费用、请求数、Token 数四类标准化数据。Cookie、请求头、登录表单、原始响应和 API Key 均不写日志或历史缓存；官网结构变化时退回已缓存数据和官网入口。
- 详情悬停和登录状态会暂停自动隐藏；恢复后重新按设置时长计时。切换服务、关闭、退出和外部空白点击都会清除暂停状态。

### 测试驱动与自动化验证

- 补充数据模型、Codex 重置券映射、DeepSeek 余额基准、30 天补零聚合、原子缓存、官网响应解析和详情暂停均先运行失败测试，再做最小实现并转绿。
- 最终完整测试为 100 个测试、22 个测试组、0 个失败；4 个需显式环境许可的检查按设计跳过。
- 随后显式启用真实 CLI 集成，Claude 认证、Claude `/usage` 和 Codex app-server 三项检查 3/3 通过。
- release 构建、ad-hoc 签名、严格签名复核、Info.plist 校验、arm64 架构检查和 `git diff --check` 全部通过。构建可执行文件 SHA-256 为 `6a9095b9af0efc7696ba2b243b45f059d97972baf2901d0e8daedd0dd8b520d8`。

### 安装与本机验收

- 安装前版本已移动到 `/private/tmp/AI Meter.app.pre-provider-details-20260830-164129`，可用于恢复；新构建已安装到 `/Applications/AI Meter.app`，安装版与构建版 SHA-256 完全一致。
- 真实账号刷新后界面显示 Claude 0%、Codex 周额度 7%、DeepSeek 余额 ¥77.99；Codex 快照同时包含 1 张可用重置券及其到期字段。
- 屏幕验收确认三个圆心仅显示放大的 Claude、OpenAI/Codex、DeepSeek Logo；Claude 0% 无伪进度，Codex 7% 为小段绿色弧，DeepSeek ¥77.99/¥100 为约 22% 绿色弧。
- DeepSeek 首次官方登录详情打开 15 秒后仍保持显示，证明登录态暂停自动隐藏；点击其他应用后立即关闭。Codex 普通详情在当前 8 秒设置下等待 10 秒后自动关闭。

## 2026-08-30：公共文档体系与目录整理

### 文档范围

- 重写根目录 README，使项目定位、当前版本、截图、功能、三服务数据来源、安装、首次配置、圆环口径、目录结构、验证命令、隐私和许可状态与当前源码一致。
- 新增统一文档入口，并按 `user-guide`、`architecture`、`development`、`design` 和 `assets` 划分长期维护职责。
- 新增安装、服务指标、设置、故障排查、架构、代码库结构、隐私安全、开发环境、测试、发布、提交历史、贡献规范和安全报告文档。
- 新增 Keep a Changelog 风格的 `CHANGELOG.md`：`0.1.0` 记录 2026-08-28 初始发布；其后已合入但未修改 Bundle 版本的功能明确归入 `Unreleased`，没有伪造 Git tag 或远程 Release。
- 历史设计规格和实施计划从内部命名的 `docs/superpowers` 移至 `docs/design`。原始决策文字保留，设计索引明确说明它们不等于当前用户配置。
- 代码目录没有为追求形式而重排；通过 `repository-structure.md` 明确 App、Core、测试、脚本和文档的职责与新增文件放置规则。

### 数据与隐私文档修正

- 删除旧 README 中已经过时的 DeepSeek“本地月预算”说明，改为当前的 ¥100 默认余额基准公式及官方 30 天用量聚合。
- 明确 DeepSeek 网页历史使用 App 自己的隔离 WebKit 会话，不读取外部浏览器 Cookie；登录会话由 WebKit 管理，业务缓存只保存日期、成本、请求数和 Token 数。
- 明确 Codex 重置额度为只读展示，不缓存兑换 ID、不自动消耗额度；Claude/Codex 凭证继续由官方 CLI 管理。
- 没有为仓库擅自选择许可证；README 和贡献指南都说明当前源代码可见不等于获得再分发许可。

### 可移植测试入口的根因与修复

- 文档验收首次直接执行 `swift test` 时，在受限环境稳定失败。错误明确指向 `~/.cache/clang/ModuleCache` 和用户级 SwiftPM cache/configuration/security 不可写；测试二进制尚未开始运行。
- 同一轮 `scripts/build-app.sh` 成功，因为现有构建脚本已经把 SwiftPM 状态和 Clang module cache 隔离到系统临时目录。对照证明根因是测试命令的缓存路径假设，不是产品代码或测试失败。
- 用完全相同的隔离参数执行一次最小验证后，100 个测试全部通过。随后新增 `scripts/test.sh`，复用临时目录模式、关闭 SwiftPM 内层沙盒，并把额外过滤参数原样传给 `swift test`。
- 所有 README、开发、测试、贡献和发布文档改用 `bash scripts/test.sh`；真实 CLI 与 Keychain 环境变量仍可与过滤参数组合使用。

### 验证证据

- 2026-08-30 17:00 +0800，`bash scripts/test.sh`：100 个测试、22 个测试组、0 个失败；1 个 Keychain 与 3 个真实 CLI 检查按设计跳过。
- `bash scripts/build-app.sh`：release 构建、App Bundle 组装、ad-hoc 签名和脚本内签名验证通过。
- 2026-08-30 17:01 +0800，独立 `plutil -lint` 与 `codesign --verify --deep --strict --verbose=2` 通过；可执行文件为 arm64 Mach-O，Bundle 版本为 `0.1.0`（build `1`）。
- 全仓库 Markdown 相对链接检查通过，所有本地目标存在；`bash -n scripts/test.sh` 与 `git diff --check` 通过。
- README 截图为从安装版 AI Meter 采集的 84 × 300 像素脱敏悬浮条，不包含账户名称、百分比、余额或登录信息。

### Git 节点

- `0f9852a docs: establish public project documentation`：建立公共文档体系并整理历史设计目录。
- `e3381ea chore: add portable test runner`：增加可移植测试入口并统一文档命令。
