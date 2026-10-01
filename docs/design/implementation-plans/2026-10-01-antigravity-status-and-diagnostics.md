# Antigravity 缓存状态与本地诊断实施计划

> **面向 AI 代理的工作者：** 必需子技能：遵循仓库TDD和验证规则逐任务实现。步骤使用复选框跟踪进度。

**目标：** 让 macOS 的 Antigravity 缓存、当前连接状态、持久暂停原因和本地阶段诊断彼此一致且隐私安全。

**架构：** Core以枚举化失败原因持久化Gemini暂停状态，并记录有界的阶段诊断事件；AppModel组合采集、缓存与暂停状态，macOS界面据此显示上次数据、暂停原因和复制摘要。所有失败仍保持暂停，Windows实现与其他Provider不变。

**技术栈：** Swift 6、Swift Testing、SwiftUI/AppKit、Codable、现有ProcessGroupCommandRunner。

---

## 文件职责

- 创建 `Sources/AIMeterCore/Diagnostics/GeminiDiagnostic.swift`：诊断阶段/结果白名单类型、有界本地Store和摘要格式化器。
- 修改 `Sources/AIMeterCore/Collectors/CommandRunner.swift` 与 `ProcessGroupCommandRunner.swift`：公开受限输出标记，不保留内容。
- 修改 `Sources/AIMeterCore/Collectors/UsageCollector.swift` 与 `GeminiCollector.swift`：typed原因、各阶段计时/记录、可选阶段独立记诊断。
- 修改 `Sources/AIMeterCore/Coordination/RefreshBackoff.swift` 与 `RefreshCoordinator.swift`：持久化暂停原因和旧记录迁移，保持所有暂停不可自动/手动清除。
- 修改 `Sources/AIMeterCore/Accounts/ServiceAccountStatus.swift`、`Sources/AIMeterCore/UI/ProviderDataState.swift`、`Sources/AIMeterApp/AppModel.swift`：分离缓存、连接、暂停语义。
- 修改 `Sources/AIMeterApp/Views/ServiceAccountStatusView.swift`、`ServicesSettingsView.swift`、`GeminiDetailView.swift`、`FloatingStripView.swift`、`UsageRing.swift`、`ProviderDetailPresentation.swift`、`GeminiInstallationGuide.swift`：一致显示状态和明确恢复操作。
- 修改英文、简体和繁体资源：为fresh、last-known、paused reason、copy诊断状态提供文案。
- 创建/修改 `Tests/AIMeterCoreTests/GeminiDiagnosticTests.swift`、`GeminiCollectorTests.swift`、`RefreshCoordinatorTests.swift` 与 `Tests/AIMeterAppTests/GeminiAvailabilityTests.swift`、Settings/UI测试：按任务覆盖失败先行。
- 修改 `docs/design/README.md`、`docs/requirements-backlog.md`；完成后新增开发记录、0.10.5/build33发布说明与公开发布证据。

## 任务 1：定义有界、无正文的诊断记录

**文件：** 新建 `Sources/AIMeterCore/Diagnostics/GeminiDiagnostic.swift`、`Tests/AIMeterCoreTests/GeminiDiagnosticTests.swift`。

- [x] 先为事件摘要构造单测：输入仅包含 `usage / timedOut / 1234ms / truncated` 时，输出稳定包含这些字段；模型没有stdout、路径或环境字符串字段。
- [x] 为20条历史和16 KiB文件上限编写Store测试：写入超量条目后只保留最近事件且编码文件小于等于16 KiB；注入目录写入失败时不抛出到采集调用方。
- [x] 运行 `swift test --filter GeminiDiagnosticTests`，确认新测试因类型缺失失败。
- [x] 实现带 `Codable` 白名单枚举与记录；Store对日期、阶段、类别、耗时和截断标志持久化，不接收任意字符串；摘要生成器只格式化最近记录。
- [x] 再运行 `swift test --filter GeminiDiagnosticTests`，确认通过。

## 任务 2：逐阶段记录CLI结果和耗时

**文件：** 修改 `CommandRunner.swift`、`ProcessGroupCommandRunner.swift`、`UsageCollector.swift`、`GeminiCollector.swift`；测试 `GeminiCollectorTests.swift` 与进程组runner测试。

- [x] 为collector增加可注入诊断Store和可控时钟；用FakeRunner断言成功/失败分别记录 `version` 与 `usage`，且记录时长、类别、时间和截断标记。
- [x] 添加用例：已成功解析usage后，model/catalog超时只产生可选阶段诊断并仍返回成功quota快照。
- [x] 添加用例：超时、明确认证错误、限流、未知输出、输出上限及取消分别保留类别；异常记录不包含FakeRunner输出字符串。
- [x] 运行 `swift test --filter 'GeminiCollectorTests|ProcessGroupCommandRunnerTests'`，确认新行为先失败。
- [x] 增加 typed 输出上限错误/截断结果，不向诊断Store传输出正文；在每个阶段边界测时并记录，主采集错误正常上抛；可选阶段捕获后只记自己的诊断。
- [x] 再运行上述过滤测试，确认通过并保留进程组超时/取消杀进程合同。

## 任务 3：持久化失败原因与迁移旧暂停

**文件：** 修改 `RefreshBackoff.swift`、`RefreshCoordinator.swift`；扩展 `RefreshBackoffTests.swift`、`RefreshCoordinatorTests.swift`。

- [x] 添加RED测试：timeout、authentication、cancel和未知结果均写成持久暂停并保持分类；重启、manual refresh和自动refresh不调用collector；仅成功显式登录恢复路径清除暂停。
- [x] 添加旧JSON fixture测试：旧 `authentication`、旧 `suspended` 且无原因的记录重载为永久 `unknown` 暂停；不把旧原因推断为登录错误。
- [x] 添加回归：只有quota成功时的可选model/catalog失败不会写入刷新暂停。
- [x] 运行 `swift test --filter 'RefreshCoordinatorTests|RefreshBackoffTests'`，确认新用例先失败。
- [x] 为backoff加入可选reason，初始化时将Gemini旧authentication迁移为suspended/unknown；新失败写入明确白名单reason，任何pause均不因启动、唤醒、定时或manual操作清除。
- [x] 再运行过滤测试，确认通过。

## 任务 4：连接/缓存/暂停状态和恢复UI

**文件：** 修改 `ServiceAccountStatus.swift`、`ProviderDataState.swift`、`AppModel.swift`、指定SwiftUI视图与三语言资源；扩展 `GeminiAvailabilityTests.swift`、`ServiceAccountSettingsTests.swift`、`AppLocalizationTests.swift`、floating/detail测试。

- [x] 先写测试：fresh quota为当前可用；cached quota映射last-known并保留原fetchedAt；带reason暂停显示明确暂停；旧reason显示未知；只有明确认证显示Sign in；识别到CLI的timeout不显示安装指南或Retry。
- [x] 为悬浮/菜单操作状态增加可访问的paused语义：auth可显示需处理，非认证pause只显示暂停标记/描述，不冒充登录要求。
- [x] 执行新增App测试筛选，确认测试因旧映射和旧按钮可见而失败。
- [x] 实现状态映射、具体但无误导的中英繁文案、条件化登录/安装/检查入口，以及Services中的Copy diagnostic summary本地剪贴板动作；复制内容来自白名单Store。
- [x] 再运行新增UI测试，并检查三语言所有新key完整一致。

## 任务 5：完整验证、独立复审和本地交付

**文件：** 更新需求台账、`docs/design/README.md` 和开发记录。

- [x] 运行聚焦Swift测试，再运行完整 `swift test`、正式macOS app构建和 `scripts/check-docs.sh`。
- [x] 运行现有Windows前端/ Rust测试，作为无改动回归证据；不得改变Windows行为。
- [x] 检查diff隐私字段、存储上限、历史状态迁移、自动刷新入口和缺省按钮路径；保留真实M4 Max未验证限制。
- [x] 由独立审查代理核对规格与验收项，模型明确使用 `gpt-6-luna` / `high`；修正所有阻断性发现并复验。
- [ ] 用户已明确授权发布：候选分支已完成本地实现与验证；待恢复GitHub CLI有效认证后推送候选分支、创建PR、合并main、建立tag/Release并更新三个更新源，最后匿名验证资产、签名、校验和与更新清单。远端凭据受限时保留本地可审查提交，不伪报发布完成。

## 完成标准

所有RED测试确认过预期失败，GREEN测试、macOS完整验证、文档检查和Windows回归通过；独立审查无未解决严重问题；旧暂停继续持久；本地提交可检查；公开发布流程完成且证据归档；M4 Max现场是否恢复单独记录为未验证，供用户用应用内复制诊断摘要决定后续是否反馈。
