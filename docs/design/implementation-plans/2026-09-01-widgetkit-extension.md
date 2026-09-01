# AI Token Meter WidgetKit 扩展实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 为 AI Token Meter 增加安全读取主应用脱敏快照的 macOS Small、Medium、Large 原生 Widget，并形成可验证的签名、打包和安装流程。

**架构：** `AIMeterCore` 新增纯 Foundation 的 Widget 共享快照、构建器和原子存储；`AIMeterApp` 每次状态发布后写入 App Group 并请求 WidgetKit 重载；新的 SwiftPM 可执行 target `AIMeterWidgetExtension` 负责时间线与三种 SwiftUI 布局。Release 脚本继续构建现有应用，并仅在提供有效 Apple 签名和 Team ID 时把 `.appex` 嵌入主应用，避免产生无法读取共享数据的伪成功 Widget。

**技术栈：** Swift 6、Swift Package Manager、SwiftUI、WidgetKit、Foundation Codable、App Group、Swift Testing、macOS 14+、codesign。

---

## 文件结构

- 修改 `Package.swift`：增加 `AIMeterWidgetExtension` 可执行产品/target 和 `AIMeterWidgetExtensionTests`。
- 创建 `Sources/AIMeterCore/Widget/WidgetSnapshotModels.swift`：定义只含展示字段的版本化共享模型。
- 创建 `Sources/AIMeterCore/Widget/WidgetSnapshotBuilder.swift`：从统一快照生成固定三 Provider、重置和重置券摘要。
- 创建 `Sources/AIMeterCore/Widget/WidgetSnapshotStore.swift`：解析 App Group 容器并原子存取 JSON。
- 创建 `Tests/AIMeterCoreTests/WidgetSnapshotBuilderTests.swift`：覆盖展示值、比例、状态、重置选择和固定顺序。
- 创建 `Tests/AIMeterCoreTests/WidgetSnapshotStoreTests.swift`：覆盖原子往返、损坏、未知版本和隐私边界。
- 创建 `Sources/AIMeterApp/System/WidgetSnapshotPublisher.swift`：主应用写入共享快照并触发 WidgetKit 重载。
- 修改 `Sources/AIMeterApp/AppModel.swift`：在启动、刷新和 DeepSeek 基准变化后发布 Widget 快照。
- 修改 `Sources/AIMeterApp/Resources/Info.plist`：声明 `aitokenmeter://open`，签名构建时注入 App Group 标识。
- 创建 `Tests/AIMeterAppTests/WidgetSnapshotPublisherTests.swift`：验证发布时机、禁用降级与重载边界。
- 创建 `Sources/AIMeterWidgetExtension/AITokenMeterWidget.swift`：WidgetBundle、配置、TimelineProvider 和深链。
- 创建 `Sources/AIMeterWidgetExtension/WidgetTimelineSource.swift`：只读共享存储并生成当前时间线。
- 创建 `Sources/AIMeterWidgetExtension/WidgetLayoutPolicy.swift`：三种尺寸的字段和降级契约。
- 创建 `Sources/AIMeterWidgetExtension/Views/WidgetRootView.swift`：按 WidgetFamily 路由。
- 创建 `Sources/AIMeterWidgetExtension/Views/WidgetProviderLogo.swift`：使用现有本地 Logo 资源。
- 创建 `Sources/AIMeterWidgetExtension/Views/WidgetProgressRing.swift`：进度与语义状态环。
- 创建 `Sources/AIMeterWidgetExtension/Views/SmallWidgetView.swift`：三个 Logo 状态框，无可见文字。
- 创建 `Sources/AIMeterWidgetExtension/Views/MediumWidgetView.swift`：三张额度卡。
- 创建 `Sources/AIMeterWidgetExtension/Views/LargeWidgetView.swift`：Provider 列表、最近重置和重置券摘要。
- 创建 `Sources/AIMeterWidgetExtension/Views/WidgetDeepSeaBackground.swift`：深海背景与可访问性降级。
- 创建 `Sources/AIMeterWidgetExtension/Resources/Info.plist`：Widget `.appex` 元数据模板。
- 创建 `Sources/AIMeterWidgetExtension/Resources/AITokenMeterWidget.entitlements`：App Sandbox 与 App Group 模板。
- 复制/复用 `Sources/AIMeterWidgetExtension/Resources/Logos/**` 和 `Backgrounds/floating-strip-deep-sea.png`：Widget 本地资源。
- 创建 `Tests/AIMeterWidgetExtensionTests/WidgetLayoutPolicyTests.swift`：三种尺寸、字段优先级、空/缓存/不可用状态。
- 创建 `Tests/AIMeterWidgetExtensionTests/WidgetSourceContractTests.swift`：Small 无可见文字、系统字体、支持尺寸和深链源码契约。
- 修改 `scripts/build-app.sh`：编译、封装、签名和嵌入 Widget，保留无 Widget 的临时签名构建。
- 创建 `scripts/verify-widget-bundle.sh`：验证 `.appex`、Bundle ID、资源、签名和双方 App Group entitlement。
- 修改 `Tests/AIMeterAppTests/AppBundleMetadataTests.swift`：验证 URL scheme、Widget Bundle ID 和构建脚本保护条件。
- 修改 `README.md`、`CHANGELOG.md`、`docs/architecture/overview.md`、`docs/architecture/repository-structure.md`、`docs/user-guide/getting-started.md`、`docs/user-guide/settings.md`、`docs/user-guide/troubleshooting.md`、`docs/development/release-process.md`、`docs/development/testing.md`、`docs/next-phase-requirements.md`：完整记录功能、签名要求、使用和排障。
- 创建 `docs/development/2026-09-01-widgetkit-extension.md`：记录红绿测试、签名探针、构建、安装和验收证据。
- 修改 `docs/development/commit-history.md`：记录规格、计划、实现、验收和合并节点。

## 任务 1：共享 Widget 快照合同、构建器与存储

**文件：**
- 创建：`Sources/AIMeterCore/Widget/WidgetSnapshotModels.swift`
- 创建：`Sources/AIMeterCore/Widget/WidgetSnapshotBuilder.swift`
- 创建：`Sources/AIMeterCore/Widget/WidgetSnapshotStore.swift`
- 创建：`Tests/AIMeterCoreTests/WidgetSnapshotBuilderTests.swift`
- 创建：`Tests/AIMeterCoreTests/WidgetSnapshotStoreTests.swift`

- [ ] **步骤 1：编写共享模型和构建器失败测试**

测试构造 Claude 18%、Codex primary 5% / secondary 31%、DeepSeek 余额 ¥77.99 / 本地消耗 22.01%，断言固定 Provider 顺序、Codex 选择风险更高窗口、DeepSeek 显示余额但使用消耗比例：

```swift
@Test("Widget snapshot keeps provider order and the same summary semantics as the app")
func widgetSnapshotSummary() throws {
    let snapshots = [
        UsageSnapshot(
            provider: .deepSeek,
            primaryMetric: UsageMetric(label: "Balance", current: 77.99, limit: nil, unit: .cny, kind: .balance),
            secondaryMetric: UsageMetric(label: "Balance baseline", current: 22.01, limit: 100, unit: .cny, kind: .localBudget)
        ),
        UsageSnapshot(
            provider: .codex,
            primaryMetric: UsageMetric(label: "Session", current: 5, limit: 100, unit: .percent),
            secondaryMetric: UsageMetric(label: "Weekly", current: 31, limit: 100, unit: .percent)
        ),
        UsageSnapshot(
            provider: .claude,
            primaryMetric: UsageMetric(label: "Session", current: 18, limit: 100, unit: .percent)
        ),
    ]

    let envelope = WidgetSnapshotBuilder().build(snapshots: snapshots, generatedAt: .init(timeIntervalSince1970: 1_000))

    #expect(envelope.version == 1)
    #expect(envelope.providers.map(\.provider) == [.claude, .codex, .deepSeek])
    #expect(envelope.providers.map(\.valueText) == ["18%", "31%", "¥77.99"])
    #expect(envelope.providers[1].fraction == 0.31)
    #expect(envelope.providers[2].fraction == 0.2201)
}
```

另写独立测试覆盖：

- 未来 `resetAt` 按时间选择最早项；没有可解析日期时按 Claude、Codex 顺序保留首个脱敏 `resetDescription`；
- Codex 重置券只保留数量和最早到期时间；
- 缺失 Provider 自动补 `.unavailable` 占位，仍保持三项固定顺序；
- `cached`、过期和不可用状态映射为对应 Widget 语义；
- 所有比例限制到 `0...1`，无界指标不伪造比例。

- [ ] **步骤 2：运行专项测试并验证红灯**

运行：

```bash
scripts/test.sh --filter WidgetSnapshotBuilderTests
```

预期：FAIL，提示 `WidgetSnapshotBuilder`、`WidgetSnapshotEnvelope` 或相关类型不存在；失败必须来自功能缺失而不是 fixture 编译错误。

- [ ] **步骤 3：实现最小共享模型与构建器**

在 `WidgetSnapshotModels.swift` 定义：

```swift
public enum WidgetProvider: String, Codable, CaseIterable, Sendable {
    case claude, codex, deepSeek
}

public enum WidgetSnapshotSemantic: String, Codable, Sendable {
    case normal, warning, critical, stale, unavailable
}

public struct WidgetProviderSnapshot: Codable, Equatable, Sendable {
    public let provider: WidgetProvider
    public let valueText: String
    public let detailText: String
    public let fraction: Double?
    public let semantic: WidgetSnapshotSemantic
    public let fetchedAt: Date?
    public let expiresAt: Date?
}

public struct WidgetResetSummary: Codable, Equatable, Sendable {
    public let provider: WidgetProvider
    public let label: String
    public let text: String
    public let resetAt: Date?
}

public struct WidgetResetCreditsSummary: Codable, Equatable, Sendable {
    public let availableCount: Int
    public let nearestExpiration: Date?
}

public struct WidgetSnapshotEnvelope: Codable, Equatable, Sendable {
    public let version: Int
    public let generatedAt: Date
    public let providers: [WidgetProviderSnapshot]
    public let nextReset: WidgetResetSummary?
    public let codexResetCredits: WidgetResetCreditsSummary?
}
```

`WidgetSnapshotBuilder.build(snapshots:generatedAt:)` 必须重用 `ProviderPresentation`，不得复制另一套百分比或语义规则。构建后按 `WidgetProvider.allCases` 生成固定顺序，并对所有字符串再次调用 `SensitiveTextRedactor.redact`。

- [ ] **步骤 4：编写存储失败测试**

使用临时目录验证：

```swift
@Test("Widget store atomically round-trips only version one envelopes")
func widgetStoreRoundTrip() throws {
    let directory = FileManager.default.temporaryDirectory
        .appendingPathComponent("ai-token-widget-\(UUID().uuidString)", isDirectory: true)
    let store = WidgetSnapshotStore(directoryURL: directory)
    let expected = WidgetSnapshotEnvelope.fixture

    try store.save(expected)
    #expect(try store.load() == expected)

    try Data(#"{"version":2}"#.utf8).write(to: store.fileURL, options: .atomic)
    #expect(try store.load() == nil)
}
```

再增加隐私回归：把常见 Authorization、API Key、Cookie、邮箱、手机号和 reset credit ID 放进输入快照可显示字符串，保存后读取原始 JSON，断言这些字面值均不存在。

- [ ] **步骤 5：运行存储测试并验证红灯**

运行：

```bash
scripts/test.sh --filter WidgetSnapshotStoreTests
```

预期：FAIL，提示 `WidgetSnapshotStore` 不存在。

- [ ] **步骤 6：实现原子存储并验证绿灯**

`WidgetSnapshotStore` 提供：

```swift
public init(directoryURL: URL, fileName: String = "widget-snapshot.json")
public init?(appGroupIdentifier: String, fileManager: FileManager = .default)
public func save(_ envelope: WidgetSnapshotEnvelope) throws
public func load() throws -> WidgetSnapshotEnvelope?
```

`load()` 对文件缺失、无法解码和 `version != 1` 返回 `nil`；`save()` 使用排序 JSON 与 `.atomic`。然后运行：

```bash
scripts/test.sh --filter WidgetSnapshotBuilderTests
scripts/test.sh --filter WidgetSnapshotStoreTests
git diff --check
```

预期：专项测试通过，差异检查无输出。

- [ ] **步骤 7：保存任务 1 节点**

```bash
git add Sources/AIMeterCore/Widget Tests/AIMeterCoreTests/WidgetSnapshotBuilderTests.swift Tests/AIMeterCoreTests/WidgetSnapshotStoreTests.swift
git commit -m "feat: add privacy-safe Widget snapshots"
```

## 任务 2：主应用发布共享快照并提供安全深链

**文件：**
- 创建：`Sources/AIMeterApp/System/WidgetSnapshotPublisher.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 修改：`Sources/AIMeterApp/Resources/Info.plist`
- 创建：`Tests/AIMeterAppTests/WidgetSnapshotPublisherTests.swift`
- 修改：`Tests/AIMeterAppTests/AppModelStartupTests.swift`

- [ ] **步骤 1：编写 Publisher 和 AppModel 发布时机失败测试**

`WidgetSnapshotPublisher` 使用可注入 store 与 reload closure：

```swift
@MainActor
@Test("Publishing stores the sanitized envelope before requesting a reload")
func publishesBeforeReload() throws {
    let recorder = WidgetPublishingRecorder()
    let publisher = WidgetSnapshotPublisher(
        save: recorder.save,
        reload: recorder.reload
    )

    publisher.publish([UsageSnapshot(provider: .claude)])

    #expect(recorder.events == ["save", "reload"])
}
```

为 `AppModel` 注入 recorder，分别验证：

- demo 启动发布一次；
- 正常 refresh 完成发布一次；
- `setDeepSeekBalanceBaseline` 重算后发布一次；
- 发布失败不会改变 `snapshots`、刷新状态或通知；
- 没有 App Group plist key 时 publisher 为禁用状态且不报错。

- [ ] **步骤 2：运行测试并验证红灯**

运行：

```bash
scripts/test.sh --filter WidgetSnapshotPublisherTests
scripts/test.sh --filter AppModelStartupTests
```

预期：新测试因 Publisher 类型/注入点缺失失败；现有 AppModel 测试仍编译。

- [ ] **步骤 3：实现 Publisher 与发布时机**

`WidgetSnapshotPublisher` 的生产工厂从主 Bundle 读取 `AIWidgetAppGroupIdentifier`。存在且可创建 `WidgetSnapshotStore` 时，保存 `WidgetSnapshotBuilder` 输出后调用：

```swift
WidgetCenter.shared.reloadTimelines(ofKind: AITokenMeterWidgetContract.kind)
```

`AITokenMeterWidgetContract.kind` 放在共享模型文件中，字面值固定为 `com.millerpan.AIMeter.usage`。AppModel 在 demo 启动、refresh 更新和 DeepSeek 基准重算后调用 publisher。错误只通过注入的安全日志 closure 报告类别，不记录 JSON 或字符串内容。

- [ ] **步骤 4：增加主应用 URL scheme**

在 `Info.plist` 增加：

```xml
<key>CFBundleURLTypes</key>
<array>
  <dict>
    <key>CFBundleURLName</key>
    <string>com.millerpan.AIMeter</string>
    <key>CFBundleURLSchemes</key>
    <array><string>aitokenmeter</string></array>
  </dict>
</array>
```

不解析 query、path 或凭证；任何 `aitokenmeter://open` 只激活应用。

- [ ] **步骤 5：运行专项和完整核心测试**

```bash
scripts/test.sh --filter WidgetSnapshotPublisherTests
scripts/test.sh --filter AppModelStartupTests
scripts/test.sh --filter AppBundleMetadataTests
scripts/test.sh --filter PrivacyRegressionTests
git diff --check
```

预期：全部通过，主应用原有启动和隐私行为无回归。

- [ ] **步骤 6：保存任务 2 节点**

```bash
git add Sources/AIMeterApp/System/WidgetSnapshotPublisher.swift Sources/AIMeterApp/AppModel.swift Sources/AIMeterApp/Resources/Info.plist Tests/AIMeterAppTests/WidgetSnapshotPublisherTests.swift Tests/AIMeterAppTests/AppModelStartupTests.swift
git commit -m "feat: publish app state for WidgetKit"
```

## 任务 3：WidgetKit target、时间线与三种已确认布局

**文件：**
- 修改：`Package.swift`
- 创建：`Sources/AIMeterWidgetExtension/AITokenMeterWidget.swift`
- 创建：`Sources/AIMeterWidgetExtension/WidgetTimelineSource.swift`
- 创建：`Sources/AIMeterWidgetExtension/WidgetLayoutPolicy.swift`
- 创建：`Sources/AIMeterWidgetExtension/Views/WidgetRootView.swift`
- 创建：`Sources/AIMeterWidgetExtension/Views/WidgetProviderLogo.swift`
- 创建：`Sources/AIMeterWidgetExtension/Views/WidgetProgressRing.swift`
- 创建：`Sources/AIMeterWidgetExtension/Views/SmallWidgetView.swift`
- 创建：`Sources/AIMeterWidgetExtension/Views/MediumWidgetView.swift`
- 创建：`Sources/AIMeterWidgetExtension/Views/LargeWidgetView.swift`
- 创建：`Sources/AIMeterWidgetExtension/Views/WidgetDeepSeaBackground.swift`
- 创建：`Sources/AIMeterWidgetExtension/Resources/Info.plist`
- 创建：`Sources/AIMeterWidgetExtension/Resources/AITokenMeterWidget.entitlements`
- 创建/复用：`Sources/AIMeterWidgetExtension/Resources/Logos/**`
- 创建/复用：`Sources/AIMeterWidgetExtension/Resources/Backgrounds/floating-strip-deep-sea.png`
- 创建：`Tests/AIMeterWidgetExtensionTests/WidgetLayoutPolicyTests.swift`
- 创建：`Tests/AIMeterWidgetExtensionTests/WidgetSourceContractTests.swift`

- [ ] **步骤 1：增加 target 骨架与失败布局测试**

`Package.swift` 增加：

```swift
.executable(name: "AIMeterWidgetExtension", targets: ["AIMeterWidgetExtension"]),
.executableTarget(
    name: "AIMeterWidgetExtension",
    dependencies: ["AIMeterCore"],
    exclude: ["Resources/Info.plist", "Resources/AITokenMeterWidget.entitlements"],
    resources: [.copy("Resources/Logos"), .copy("Resources/Backgrounds")],
    swiftSettings: [.unsafeFlags(["-application-extension"])]
),
.testTarget(
    name: "AIMeterWidgetExtensionTests",
    dependencies: ["AIMeterWidgetExtension"]
),
```

先只创建空 target 目录与测试，测试要求：

```swift
@Test("The widget supports exactly the approved three families")
func approvedFamilies() {
    #expect(WidgetLayoutPolicy.supportedFamilies == [.systemSmall, .systemMedium, .systemLarge])
}

@Test("Small exposes logos and rings without visible text fields")
func smallIsLogoOnly() {
    #expect(WidgetLayoutPolicy.visibleFields(for: .systemSmall) == [.logos, .rings])
}
```

中型期望 `.logos, .providerNames, .values, .detailLabels, .progressBars`；大型在此基础上增加 `.nextReset, .resetCredits`。

- [ ] **步骤 2：运行测试并验证红灯**

```bash
scripts/test.sh --filter WidgetLayoutPolicyTests
```

预期：FAIL，提示 `WidgetLayoutPolicy` 不存在。

- [ ] **步骤 3：实现最小布局政策和时间线源**

实现 `WidgetLayoutPolicy`、`WidgetEntry`、`WidgetTimelineSource`：

- Bundle 中没有 App Group key或共享文件缺失时返回三项 unavailable 占位；
- gallery preview 使用固定示例，不读取真实账户；
- timeline 使用当前 entry，并以 `Date().addingTimeInterval(30 * 60)` 作为 `.after` 建议；
- `now >= expiresAt` 时把对应 Provider 语义提升为 `.stale`；
- 不发网络请求、不执行 CLI、不读 Keychain。

- [ ] **步骤 4：实现 Small 并锁定无文字合同**

`SmallWidgetView` 只遍历三个 Provider 生成 `WidgetProgressRing` + `WidgetProviderLogo`。源码合同测试读取 `SmallWidgetView.swift`，断言不含 `Text(`、`Button(`、`Link(`，并断言包含 `accessibilityLabel`。这使“小型无可见文字”不能在后续布局调整中悄悄回归。

Logo 的视觉尺寸统一使用单一校准表；状态环沿用 Provider 品牌色，语义异常时调用现有的语义颜色/符号映射。

- [ ] **步骤 5：实现 Medium 与 Large**

- `MediumWidgetView`：三张等宽卡片，固定顺序，显示 Logo、Provider 名、主值、短标签和进度条；
- `LargeWidgetView`：左侧三行 Provider，右上最近重置，右下 Codex 重置券；
- 空值和未知状态使用设计规格中的字面文案；
- 根视图统一应用 `WidgetDeepSeaBackground`、系统字体、`containerBackground` 和 `widgetURL(URL(string: "aitokenmeter://open"))`；
- 不读取 App 的 Antonio/DIN 偏好，不添加交互按钮。

- [ ] **步骤 6：运行 Widget 测试和真实编译**

```bash
scripts/test.sh --filter WidgetLayoutPolicyTests
scripts/test.sh --filter WidgetSourceContractTests
swift build --product AIMeterWidgetExtension
git diff --check
```

预期：测试通过，WidgetKit 可执行文件编译成功，差异检查无输出。

- [ ] **步骤 7：保存任务 3 节点**

```bash
git add Package.swift Sources/AIMeterWidgetExtension Tests/AIMeterWidgetExtensionTests
git commit -m "feat: add three-size WidgetKit views"
```

## 任务 4：安全打包、签名保护与 Bundle 验证

**文件：**
- 修改：`scripts/build-app.sh`
- 创建：`scripts/verify-widget-bundle.sh`
- 修改：`Tests/AIMeterAppTests/AppBundleMetadataTests.swift`
- 创建：`Tests/AIMeterAppTests/WidgetBuildScriptTests.swift`

- [ ] **步骤 1：编写构建脚本保护合同失败测试**

测试读取脚本并验证以下稳定合同：

- Widget 可执行名 `AIMeterWidgetExtension`；
- `.appex` 目标为 `Contents/PlugIns/AITokenMeterWidget.appex`；
- 显式 `AI_METER_INCLUDE_WIDGET=1` 时必须同时检查 `AI_METER_CODESIGN_IDENTITY` 与 `AI_METER_WIDGET_TEAM_ID`；
- Widget 先签名、主应用后签名；
- 主应用与 Widget 使用同一个计算出的 App Group；
- 缺少签名时不得写入 `AIWidgetAppGroupIdentifier` 或嵌入 `.appex`。

- [ ] **步骤 2：运行构建脚本测试并验证红灯**

```bash
scripts/test.sh --filter WidgetBuildScriptTests
```

预期：FAIL，脚本尚未包含 Widget 打包合同。

- [ ] **步骤 3：实现条件式 Widget 打包**

保留当前普通构建。新增环境变量：

```text
AI_METER_INCLUDE_WIDGET=auto|0|1   默认 auto
AI_METER_CODESIGN_IDENTITY         可选覆盖；默认自动选择第一项 Apple Development 身份
AI_METER_WIDGET_TEAM_ID            可选覆盖；默认从所选证书 Subject OU 读取 Apple Team ID
```

行为：

- `0`：不编译/嵌入 Widget；主应用临时签名；
- `auto`：自动检测签名和 Team ID；两者都存在才嵌入，否则输出明确的 `Widget skipped` 提示并继续普通主应用构建；
- `1`：缺少任一值立即失败，输出在 Xcode 登录 Apple Account 的一次性指引；
- App Group 固定计算为 `${AI_METER_WIDGET_TEAM_ID}.com.millerpan.AIMeter`；
- 编译 `AIMeterWidgetExtension` release product；
- 创建标准 `.appex/Contents/MacOS` 与 `Resources`，复制 Info.plist、资源 bundle 和可执行文件；
- 在临时副本中把双方 entitlements 和主应用 Info.plist 的 App Group 标识替换为实际值；
- 先签 `.appex`，再签 `.app`，最后严格深度验证。

- [ ] **步骤 4：实现 Bundle 验证脚本**

`verify-widget-bundle.sh <app-path> <expected-team-id>` 必须检查：

```bash
test -x "$APP/Contents/PlugIns/AITokenMeterWidget.appex/Contents/MacOS/AIMeterWidgetExtension"
plutil -lint "$APP/Contents/PlugIns/AITokenMeterWidget.appex/Contents/Info.plist"
codesign --verify --deep --strict --verbose=2 "$APP"
```

再读取主应用与扩展 entitlements，断言两边都包含 `${TEAM_ID}.com.millerpan.AIMeter`，扩展还包含 `com.apple.security.app-sandbox = true`。脚本不得输出完整 entitlements 之外的凭证或用户文件内容。

- [ ] **步骤 5：验证普通构建和签名缺失保护**

```bash
AI_METER_INCLUDE_WIDGET=0 scripts/build-app.sh
test ! -e "dist/AI Token Meter.app/Contents/PlugIns/AITokenMeterWidget.appex"
AI_METER_INCLUDE_WIDGET=1 scripts/build-app.sh
```

预期：第一条构建并验证普通主应用；第二条在当前无签名环境以明确的签名缺失消息失败，不生成半成品 Widget。

- [ ] **步骤 6：运行完整自动化验证**

```bash
scripts/test.sh
swift build --product AIMeterWidgetExtension
git diff --check
```

预期：完整测试全部通过，仅现有三个环境授权型测试按设计跳过；Widget target 编译成功；差异检查无输出。

- [ ] **步骤 7：保存任务 4 节点**

```bash
git add scripts/build-app.sh scripts/verify-widget-bundle.sh Tests/AIMeterAppTests/AppBundleMetadataTests.swift Tests/AIMeterAppTests/WidgetBuildScriptTests.swift
git commit -m "build: package signed WidgetKit extension"
```

## 任务 5：文档、签名安装与真实桌面验收

**文件：**
- 修改：`README.md`
- 修改：`CHANGELOG.md`
- 修改：`docs/architecture/overview.md`
- 修改：`docs/architecture/repository-structure.md`
- 修改：`docs/user-guide/getting-started.md`
- 修改：`docs/user-guide/settings.md`
- 修改：`docs/user-guide/troubleshooting.md`
- 修改：`docs/development/release-process.md`
- 修改：`docs/development/testing.md`
- 修改：`docs/next-phase-requirements.md`
- 创建：`docs/development/2026-09-01-widgetkit-extension.md`
- 修改：`docs/development/commit-history.md`

- [ ] **步骤 1：更新用户与架构文档**

文档必须说明：

- 如何在桌面编辑模式中添加 AI Token Meter Widget；
- Small/Medium/Large 各自显示什么；
- Widget 数据来自主应用脱敏快照，刷新受系统预算控制；
- Widget 不存储凭证、不登录、不兑换重置券；
- 首次开发签名配置与 `AI_METER_INCLUDE_WIDGET=1` 的构建方式；
- `Widget skipped`、Gallery 找不到、数据过期和 App Group 不匹配的排障；
- 普通无 Widget 构建仍受支持。

- [ ] **步骤 2：运行文档与完整回归检查**

```bash
scripts/test.sh
git diff --check
```

再运行仓库现有 Markdown 相对链接检查流程。预期测试全绿、相对链接无断链、差异检查无输出。

- [ ] **步骤 3：处理一次性签名前置条件**

运行：

```bash
security find-identity -v -p codesigning
```

如果仍为 `0 valid identities found`，停止真实安装，不伪造成功；向用户提供唯一必要操作：在 Xcode > Settings > Accounts 登录 Apple Account，并让 Xcode 创建 Apple Development 证书。代码、测试、文档和无 Widget 主应用构建可以完成，但“桌面 Widget 已安装”不得标记完成。

- [ ] **步骤 4：有签名后构建并验证 Widget 应用**

获得实际身份与 Team ID 后运行：

```bash
AI_METER_INCLUDE_WIDGET=1 scripts/build-app.sh
scripts/verify-widget-bundle.sh "dist/AI Token Meter.app"
```

构建脚本从钥匙串的实际 Apple Development 证书自动解析身份和 Team ID；验证脚本从安装包主签名读取 Team ID，并据此检查双方 App Group。自动检测失败时才允许通过同名环境变量覆盖，覆盖值必须来自 `security` 的实际输出，绝不猜测或提交进仓库。

- [ ] **步骤 5：安全替换安装版并保留备份**

退出当前 AI Token Meter。把 `/Applications/AI Token Meter.app` 移动到带时间戳的 `/private/tmp/AI-Token-Meter-pre-widget-<timestamp>/` 备份，再复制新构建；验证安装版签名与构建版可执行文件 SHA-256 一致后启动。

- [ ] **步骤 6：真实 Widget Gallery 与桌面验收**

按规格逐项验证：

1. Gallery 可找到 AI Token Meter；
2. Small 只有三个 Logo 状态框和环，无可见文字；
3. Medium 三卡数值与主应用一致；
4. Large 的最近重置和重置券数量/到期正确；
5. 主应用刷新后 Widget 更新；退出主应用后缓存与过期状态正确；
6. 点击 Widget 只打开主应用；
7. 浅/深色、提高对比度和减少透明度均可读；
8. 共享 JSON 不含敏感值。

将截图、签名摘要、测试计数、已知 WidgetKit 调度延迟和安装哈希写入开发日志，不记录账户凭证。

- [ ] **步骤 7：保存文档与验收节点**

```bash
git add README.md CHANGELOG.md docs
git commit -m "docs: document WidgetKit setup and acceptance"
```

- [ ] **步骤 8：最终审查和合并准备**

调用 requesting-code-review 做规格与实现审查；修复后重新运行完整测试、Widget 编译、签名验证和 `git diff --check`。全部通过后调用 finishing-a-development-branch，并按用户既有授权合并到 `main`；没有有效签名时只能合并代码，必须把真实安装验收明确标为受外部签名阻塞，不能写成已完成。
