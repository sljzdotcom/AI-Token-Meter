# Provider Detail Enhancements 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 在 AI Meter 中只读显示 Codex 重置券，为 DeepSeek 提供自动同步的最近 30 天原生分析卡，并把悬浮条精简为三个带真实进度弧的大 Logo。

**架构：** 在 `UsageSnapshot` 上添加可选的提供商补充数据；Codex 继续由官方 app-server 一次读取通用额度和重置券。DeepSeek 余额基准由现有设置迁移并直接换算为消耗比例；历史用量由独立 WebKit 会话获取、规范化并缓存，UI 只观察标准化模型。

**技术栈：** Swift 6、Swift Testing、SwiftUI、Swift Charts、WebKit/WKWebView、Codex app-server JSON-RPC、Codable 原子缓存、macOS 14。

---

## 文件结构

- 创建 `Sources/AIMeterCore/Domain/ProviderSupplementalData.swift`：Codex 重置券与 DeepSeek 30 天用量的纯领域模型。
- 修改 `Sources/AIMeterCore/Domain/UsageModels.swift`：为快照增加可选补充数据，保持旧缓存兼容。
- 创建 `Sources/AIMeterCore/DeepSeek/DeepSeekHistoryNormalizer.swift`：把官网日数据裁剪、补零并聚合为 30 天模型。
- 创建 `Sources/AIMeterCore/Persistence/DeepSeekHistoryStore.swift`：原子保存标准化历史，不保存认证材料。
- 修改 `Sources/AIMeterCore/Collectors/CodexAppServerClient.swift`：解码 `rateLimitResetCredits` 并丢弃兑换 ID。
- 修改 `Sources/AIMeterCore/Presentation/AppPresentation.swift`：DeepSeek 使用余额基准指标作为圆环比例，同时保留余额文本供详情使用。
- 创建 `Sources/AIMeterApp/System/DeepSeekWebSession.swift`：持久 WebKit 登录会话、官网响应桥与同步状态。
- 创建 `Sources/AIMeterApp/Views/ProviderLogo.swift`：统一绘制本地品牌 Logo。
- 创建 `Sources/AIMeterApp/Views/CodexResetCreditsView.swift`：只读券数量和到期日。
- 创建 `Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift`：统计卡、Swift Charts 柱状图和同步状态。
- 修改 `Sources/AIMeterApp/Views/UsageRing.swift`、`FloatingStripView.swift`、`ProviderCard.swift`、`SettingsView.swift`：接入 Logo、详情和余额基准。
- 修改 `Sources/AIMeterApp/System/FloatingPanelController.swift` 与 `Sources/AIMeterCore/UI/FloatingDetailSession.swift`：DeepSeek 大卡尺寸和悬停／登录暂停自动隐藏。
- 修改 `Sources/AIMeterApp/AppModel.swift`：设置迁移、历史缓存和 Web 会话状态编排。
- 修改 `Package.swift` 与 `scripts/build-app.sh`：链接 Charts/WebKit 所需系统能力并打包本地 Logo 资源。
- 新增或修改 `Tests/AIMeterCoreTests/*`：覆盖模型兼容、Codex 券、DeepSeek 规范化、缓存、圆环计算和暂停会话。

### 任务 1：补充领域模型与向后兼容缓存

**文件：**
- 创建：`Sources/AIMeterCore/Domain/ProviderSupplementalData.swift`
- 修改：`Sources/AIMeterCore/Domain/UsageModels.swift`
- 创建：`Tests/AIMeterCoreTests/ProviderSupplementalDataTests.swift`
- 修改：`Tests/AIMeterCoreTests/SnapshotCacheTests.swift`

- [ ] **步骤 1：编写失败的模型与旧缓存测试**

```swift
@Test("Codex reset credits retain display data without redeem identifiers")
func codexCreditDisplayModel() throws {
    let expiry = Date(timeIntervalSince1970: 1_900_000_000)
    let summary = CodexResetCreditsSummary(
        availableCount: 2,
        credits: [CodexResetCreditDisplay(title: "Usage reset", expiresAt: expiry)],
        hasCompleteDetails: false
    )
    let data = try JSONEncoder().encode(summary)
    let json = try #require(String(data: data, encoding: .utf8))
    #expect(!json.contains("creditId"))
    #expect(try JSONDecoder().decode(CodexResetCreditsSummary.self, from: data) == summary)
}

@Test("Old snapshots decode without supplemental provider data")
func oldSnapshotCompatibility() throws {
    let legacy = #"{"provider":"codex","primaryMetric":null,"secondaryMetric":null,"availability":"available","fetchedAt":0,"staleAfter":300,"sourceVersion":null,"collectionStatus":"fresh","statusMessage":null}"#.data(using: .utf8)!
    let snapshot = try JSONDecoder().decode(UsageSnapshot.self, from: legacy)
    #expect(snapshot.codexResetCredits == nil)
    #expect(snapshot.deepSeekUsageHistory == nil)
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details --filter ProviderSupplementalDataTests
```

预期：编译失败，报告 `CodexResetCreditsSummary`、`DeepSeekUsageHistory` 或快照属性不存在。

- [ ] **步骤 3：实现最少模型**

```swift
public struct CodexResetCreditDisplay: Codable, Equatable, Sendable {
    public let title: String?
    public let expiresAt: Date?
}

public struct CodexResetCreditsSummary: Codable, Equatable, Sendable {
    public let availableCount: Int
    public let credits: [CodexResetCreditDisplay]
    public let hasCompleteDetails: Bool
}

public struct DeepSeekDailyUsage: Codable, Equatable, Identifiable, Sendable {
    public var id: Date { date }
    public let date: Date
    public let costCNY: Double
    public let requestCount: Int
    public let tokenCount: Int
}

public struct DeepSeekUsageHistory: Codable, Equatable, Sendable {
    public let days: [DeepSeekDailyUsage]
    public let updatedAt: Date
    public let statusMessage: String?
}
```

为 `UsageSnapshot` 添加可选 `codexResetCredits` 与 `deepSeekUsageHistory`，初始化器默认均为 `nil`；复制快照的调用点必须显式保留这两个字段。

- [ ] **步骤 4：运行专项与缓存回归**

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details --filter ProviderSupplementalDataTests
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details --filter SnapshotCacheTests
```

预期：全部通过。

- [ ] **步骤 5：Commit**

```bash
git add Sources/AIMeterCore/Domain/ProviderSupplementalData.swift Sources/AIMeterCore/Domain/UsageModels.swift Tests/AIMeterCoreTests/ProviderSupplementalDataTests.swift Tests/AIMeterCoreTests/SnapshotCacheTests.swift
git commit -m "feat: add provider supplemental data models"
```

### 任务 2：Codex 重置券官方采集与只读展示

**文件：**
- 修改：`Sources/AIMeterCore/Collectors/CodexAppServerClient.swift`
- 修改：`Tests/AIMeterCoreTests/Fixtures/fake-codex-general-and-model.sh`
- 修改：`Tests/AIMeterCoreTests/CLICollectorTests.swift`
- 创建：`Sources/AIMeterApp/Views/CodexResetCreditsView.swift`
- 修改：`Sources/AIMeterApp/Views/ProviderCard.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift`

- [ ] **步骤 1：扩展 fixture 并编写失败采集测试**

fixture 的 `result` 增加：

```json
"rateLimitResetCredits": {
  "availableCount": 2,
  "credits": [
    {"id":"discard-me-1","status":"available","resetType":"codexRateLimits","grantedAt":1890000000,"title":"Bonus reset","description":null,"expiresAt":1900000000},
    {"id":"discard-me-2","status":"available","resetType":"codexRateLimits","grantedAt":1890000001,"title":null,"description":null,"expiresAt":1900100000},
    {"id":"discard-me-3","status":"redeemed","resetType":"codexRateLimits","grantedAt":1890000002,"title":"Used","description":null,"expiresAt":1900200000}
  ]
}
```

```swift
#expect(snapshot.codexResetCredits?.availableCount == 2)
#expect(snapshot.codexResetCredits?.credits.count == 2)
#expect(snapshot.codexResetCredits?.credits.map(\.expiresAt) == [firstExpiry, secondExpiry])
#expect(snapshot.codexResetCredits?.hasCompleteDetails == true)
```

- [ ] **步骤 2：运行测试验证失败**

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details --filter CLICollectorTests/codexPrefersGeneralLimit
```

预期：FAIL，快照没有重置券摘要。

- [ ] **步骤 3：最小解码和映射**

```swift
private struct CodexRateLimitResetCreditsSummary: Decodable {
    let availableCount: Int
    let credits: [CodexRateLimitResetCredit]?
}

private struct CodexRateLimitResetCredit: Decodable {
    let status: String
    let title: String?
    let expiresAt: Int64?
}
```

仅将 `status == "available"` 映射为 `CodexResetCreditDisplay`；`hasCompleteDetails` 在 `credits != nil && filtered.count >= availableCount` 时为真。不要把 `id` 加入任何领域模型。

- [ ] **步骤 4：增加只读 SwiftUI 视图并跑回归**

`CodexResetCreditsView` 显示总数、每张券的格式化到期日和不完整明细提示；不出现 Button。将它接入两个 Codex 详情入口。

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details --filter CLICollectorTests
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details
```

预期：全部通过，原通用周额度 5% 回归仍绿。

- [ ] **步骤 5：Commit**

```bash
git add Sources/AIMeterCore/Collectors/CodexAppServerClient.swift Sources/AIMeterApp/Views/CodexResetCreditsView.swift Sources/AIMeterApp/Views/ProviderCard.swift Sources/AIMeterApp/Views/FloatingStripView.swift Tests/AIMeterCoreTests/Fixtures/fake-codex-general-and-model.sh Tests/AIMeterCoreTests/CLICollectorTests.swift
git commit -m "feat: show Codex reset credit expirations"
```

### 任务 3：Logo-only 圆环与 DeepSeek 可配置余额基准

**文件：**
- 创建：`Sources/AIMeterApp/Views/ProviderLogo.swift`
- 修改：`Sources/AIMeterApp/Views/UsageRing.swift`
- 修改：`Sources/AIMeterApp/Views/UsageVisualStyle.swift`
- 修改：`Sources/AIMeterCore/Presentation/AppPresentation.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 修改：`Sources/AIMeterApp/Views/SettingsView.swift`
- 修改：`Package.swift`
- 修改：`scripts/build-app.sh`
- 修改：`Tests/AIMeterCoreTests/AppPresentationTests.swift`

- [ ] **步骤 1：编写失败的余额基准测试**

```swift
@Test("DeepSeek ring reports balance depletion against its baseline")
func deepSeekBalanceDepletion() {
    let snapshot = UsageSnapshot(
        provider: .deepSeek,
        primaryMetric: UsageMetric(label: "Available balance", current: 77.99, limit: nil, unit: .cny, kind: .balance),
        secondaryMetric: UsageMetric(label: "Balance baseline", current: 22.01, limit: 100, unit: .cny, kind: .localBudget)
    )
    let presentation = ProviderPresentation(snapshot: snapshot)
    #expect(abs((presentation.ringFraction ?? 0) - 0.2201) < 0.0001)
    #expect(presentation.valueText == "¥77.99")
}
```

同时断言余额高于基准为 0、余额为 0 时为 1、基准小于 1 时由设置层回退为 100。

- [ ] **步骤 2：运行测试验证失败**

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details --filter AppPresentationTests/deepSeekBalanceDepletion
```

预期：FAIL，DeepSeek `ringFraction` 为 nil。

- [ ] **步骤 3：实现余额基准和展示选择**

把设置文案从 `Monthly local budget` 改为 `Balance baseline`，保留旧 `monthlyBudget` defaults 值作为一次迁移来源。快照补充指标使用：

```swift
let used = min(max(deepSeekBalanceBaseline - balanceMetric.current, 0), deepSeekBalanceBaseline)
let baselineMetric = UsageMetric(
    label: "Balance baseline",
    current: used,
    limit: deepSeekBalanceBaseline,
    unit: .cny,
    kind: .localBudget
)
```

`ProviderPresentation` 对 DeepSeek 使用该指标的 fraction/semantic，但 `valueText` 继续来自余额指标。

- [ ] **步骤 4：实现 Logo-only 圆环**

`UsageRing` 删除中心 `VStack` 和 `Text`，改为：

```swift
ProviderLogo(provider: presentation.provider)
    .frame(width: size * 0.48, height: size * 0.48)
```

`ProviderLogo` 优先加载随包资源，加载失败时使用当前 SF Symbol 回退。辅助功能标签继续包含 `presentation.title` 与 `valueText`。构建脚本把 Logo 资源复制进 `Contents/Resources/Logos`。

- [ ] **步骤 5：运行专项、全量和构建**

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details --filter AppPresentationTests
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details
./scripts/build-app.sh
```

预期：全部通过；release App 中三个 Logo 资源存在。

- [ ] **步骤 6：Commit**

```bash
git add Package.swift scripts/build-app.sh Sources/AIMeterApp/AppModel.swift Sources/AIMeterApp/Views/ProviderLogo.swift Sources/AIMeterApp/Views/UsageRing.swift Sources/AIMeterApp/Views/UsageVisualStyle.swift Sources/AIMeterApp/Views/SettingsView.swift Sources/AIMeterCore/Presentation/AppPresentation.swift Tests/AIMeterCoreTests/AppPresentationTests.swift Sources/AIMeterApp/Resources/Logos
git commit -m "feat: use logo-only provider balance rings"
```

### 任务 4：DeepSeek 30 天数据规范化与安全缓存

**文件：**
- 创建：`Sources/AIMeterCore/DeepSeek/DeepSeekHistoryNormalizer.swift`
- 创建：`Sources/AIMeterCore/Persistence/DeepSeekHistoryStore.swift`
- 创建：`Tests/AIMeterCoreTests/DeepSeekHistoryNormalizerTests.swift`
- 创建：`Tests/AIMeterCoreTests/DeepSeekHistoryStoreTests.swift`

- [ ] **步骤 1：编写失败的规范化测试**

```swift
@Test("Normalizes exactly thirty local days and fills missing dates")
func normalizesThirtyDays() throws {
    let history = DeepSeekHistoryNormalizer(calendar: utcCalendar).normalize(
        records: [
            .init(date: date("2026-08-29"), costCNY: 2.5, requestCount: 10, tokenCount: 1_000),
            .init(date: date("2026-08-29"), costCNY: 1.5, requestCount: 5, tokenCount: 500),
        ],
        endingAt: date("2026-08-30"),
        updatedAt: date("2026-08-30")
    )
    #expect(history.days.count == 30)
    #expect(history.days.last?.costCNY == 0)
    #expect(history.days.dropLast().last?.costCNY == 4)
    #expect(history.totalRequests == 15)
    #expect(history.totalTokens == 1_500)
}
```

- [ ] **步骤 2：运行测试验证失败**

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details --filter DeepSeekHistory
```

预期：编译失败，规范化器和存储不存在。

- [ ] **步骤 3：实现规范化与原子存储**

规范化器按 `Calendar.startOfDay` 合并同日记录，生成闭区间 `[end-29 days, end]`，缺失日补零。历史模型提供聚合计算属性。存储使用临时文件加 `.replaceItemAt`／`.moveItem`，只保存标准化模型。

- [ ] **步骤 4：运行专项和隐私回归**

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details --filter DeepSeekHistory
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details --filter PrivacyRegressionTests
```

预期：全部通过，缓存 JSON 不含 `cookie`、`authorization`、`api_key` 或原始响应。

- [ ] **步骤 5：Commit**

```bash
git add Sources/AIMeterCore/DeepSeek/DeepSeekHistoryNormalizer.swift Sources/AIMeterCore/Persistence/DeepSeekHistoryStore.swift Tests/AIMeterCoreTests/DeepSeekHistoryNormalizerTests.swift Tests/AIMeterCoreTests/DeepSeekHistoryStoreTests.swift
git commit -m "feat: normalize and cache DeepSeek usage history"
```

### 任务 5：DeepSeek WebKit 同步与大型原生分析卡

**文件：**
- 创建：`Sources/AIMeterApp/System/DeepSeekWebSession.swift`
- 创建：`Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift`
- 修改：`Sources/AIMeterApp/System/FloatingPanelController.swift`
- 修改：`Sources/AIMeterCore/UI/FloatingDetailSession.swift`
- 修改：`Tests/AIMeterCoreTests/FloatingDetailSessionTests.swift`

- [ ] **步骤 1：编写暂停自动隐藏的失败测试**

```swift
@Test("Paused detail does not auto-hide until resumed")
@MainActor
func pauseAndResume() async throws {
    let session = FloatingDetailSession()
    session.present(.deepSeek, autoHideAfter: .milliseconds(30))
    session.setAutoHidePaused(true)
    try await Task.sleep(for: .milliseconds(50))
    #expect(session.selectedProvider == .deepSeek)
    session.setAutoHidePaused(false, restartAfter: .milliseconds(20))
    try await Task.sleep(for: .milliseconds(40))
    #expect(session.selectedProvider == nil)
}
```

- [ ] **步骤 2：运行测试验证失败**

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details --filter FloatingDetailSessionTests/pauseAndResume
```

预期：编译失败，暂停 API 不存在。

- [ ] **步骤 3：最小实现暂停会话**

保存当前 provider 和配置时长；暂停时取消 task 但不改变选择，恢复时为同一 generation 重新启动倒计时。dismiss、切换 provider 和 shutdown 都清除暂停状态。

- [ ] **步骤 4：实现 WebKit 会话和窄桥**

`DeepSeekWebSession` 仅允许 `https://platform.deepseek.com/` 导航。document-start 用户脚本包装同源 `fetch`/XHR，复制 JSON 响应并送入解析器；消息桥拒绝非官方 origin、超过 2 MB 的 payload 和无法识别的结构。只把 `[DeepSeekDailyUsage]` 交给 AppModel；登录表单值、Cookie 和 Header 不进入 Swift 日志或缓存。

同步状态为 `signedOut`、`loading`、`ready`、`stale(message)`。打开详情时加载缓存；缓存超过 30 分钟才自动刷新，手动刷新强制重新加载官方 Usage 页面。

- [ ] **步骤 5：实现原生分析卡和动态面板**

`DeepSeekAnalyticsView` 使用 `Chart(history.days)`：

```swift
Chart(history.days) { day in
    BarMark(
        x: .value("Day", day.date, unit: .day),
        y: .value("Cost", day.costCNY)
    )
    .foregroundStyle(Color.blue.gradient)
}
```

DeepSeek 面板使用约 `620 x 520`，其他服务保持紧凑尺寸。`onHover` 暂停／恢复倒计时；登录状态持续暂停。点击空白处仍由现有屏幕坐标命中策略关闭。

- [ ] **步骤 6：运行全量测试、构建和静态安全检查**

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details
./scripts/build-app.sh
git diff --check
rg -n -i 'cookie|authorization|creditId|api[_-]?key' Sources/AIMeterApp/System/DeepSeekWebSession.swift Sources/AIMeterCore/Persistence/DeepSeekHistoryStore.swift
```

预期：测试和构建通过；代码只在明确的拒绝／过滤逻辑中提及敏感字段，不记录其值。

- [ ] **步骤 7：Commit**

```bash
git add Sources/AIMeterApp/System/DeepSeekWebSession.swift Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift Sources/AIMeterApp/AppModel.swift Sources/AIMeterApp/Views/FloatingStripView.swift Sources/AIMeterApp/System/FloatingPanelController.swift Sources/AIMeterCore/UI/FloatingDetailSession.swift Tests/AIMeterCoreTests/FloatingDetailSessionTests.swift
git commit -m "feat: add DeepSeek thirty-day analytics panel"
```

### 任务 6：安装验收、日志与合并 main

**文件：**
- 修改：`docs/development/2026-08-28-development-log.md`

- [ ] **步骤 1：最终自动化验证**

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details-final
AI_METER_RUN_CLI_SMOKE=1 swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details-final --filter CLIIntegrationSmokeTests
./scripts/build-app.sh
codesign --verify --deep --strict --verbose=2 "dist/AI Meter.app"
plutil -lint "dist/AI Meter.app/Contents/Info.plist"
file "dist/AI Meter.app/Contents/MacOS/AIMeterApp"
git diff --check
```

预期：全绿；环境依赖测试仅在明确标记时跳过。

- [ ] **步骤 2：可恢复安装和本机视觉验收**

把当前安装包移动到唯一 `/private/tmp/AI Meter.app.pre-provider-details-<timestamp>`，安装新包并启动。验收：三个圆心只有大 Logo；Codex 显示券数量和到期日；DeepSeek ¥77.99/¥100 约 22%；大卡尺寸、柱状图、悬停暂停、空白关闭和官网登录降级均正常。

- [ ] **步骤 3：记录证据并 Commit**

```bash
git add docs/development/2026-08-28-development-log.md
git commit -m "docs: record provider detail acceptance"
```

- [ ] **步骤 4：合并 main 并在合并树复测**

从主仓库切换到 `main`，确认工作树无用户未提交修改，再执行：

```bash
git merge --no-ff feat/initial-app
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-provider-details-main
```

测试通过后移除 `.worktrees/initial-app` 并删除已合并功能分支；如果主工作树存在未提交用户修改或合并后测试失败，保留分支和 worktree，停止清理并在最终报告中说明。
