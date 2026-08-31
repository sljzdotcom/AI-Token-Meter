# AI Meter 全局显示字体选择实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 在 Settings 中提供 System Default、Antonio、DIN Condensed 三种全局显示字体和恢复默认按钮，使可用字体无需重启即可覆盖 AI Meter 全部自绘文字，并让当前机器最终选择 Antonio。

**架构：** 在 `AIMeterCore` 建立稳定、可测试的字体选择与 UserDefaults 存储；在 `AIMeterApp` 建立 AppKit 字体可用性目录和 SwiftUI 语义字体环境。根视图从可观察的 `AppModel` 注入选择，各视图通过统一 modifier 请求标题、正文、说明或固定字号字体，不直接读取偏好或写死字体家族。

**技术栈：** Swift 6、SwiftUI、Observation、AppKit `NSFontManager`、UserDefaults、Swift Testing、ImageRenderer、macOS Computer Use、SwiftPM Release App Bundle 与 ad-hoc 签名。

---

## 文件结构

- 创建：`Sources/AIMeterCore/Preferences/DisplayFontChoice.swift` — 三个稳定选择、显示名称和 UserDefaults 存储。
- 创建：`Tests/AIMeterCoreTests/DisplayFontPreferenceTests.swift` — 默认、保存、恢复、损坏值与标识测试。
- 修改：`Sources/AIMeterApp/AppModel.swift` — 加载、观察、设置和恢复字体偏好。
- 修改：`Tests/AIMeterAppTests/AppModelStartupTests.swift` — AppModel 默认值、即时更新和持久化测试。
- 创建：`Sources/AIMeterApp/Views/AIMeterTypography.swift` — 字体目录、语义角色、环境键、解析器和 View modifier。
- 创建：`Tests/AIMeterAppTests/TypographyTests.swift` — 可用性、回退、语义字号、字重和家族解析测试。
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift` — 浮动条和详情根作用域及文字字体迁移。
- 修改：`Sources/AIMeterApp/Views/MenuBarPanel.swift` — 菜单栏标签、面板和子视图字体迁移。
- 修改：`Sources/AIMeterApp/Views/ProviderCard.swift` — Provider 卡片字体迁移。
- 修改：`Sources/AIMeterApp/Views/CodexDetailView.swift` — Codex 详情字体迁移。
- 修改：`Sources/AIMeterApp/Views/CodexResetCreditsView.swift` — 重置券字体迁移。
- 修改：`Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift` — DeepSeek 统计与图表字体迁移。
- 修改：`Sources/AIMeterApp/Views/UsageRing.swift` — 圆环异常文字字体迁移。
- 修改：`Sources/AIMeterApp/Views/SettingsView.swift` — 字体选择、可用性状态和恢复按钮。
- 修改：`Tests/AIMeterAppTests/VisualSystemTests.swift` — 三选项展示模型及全局字体边界回归。
- 修改：`README.md`、`CHANGELOG.md`、`docs/user-guide/settings.md` — 用户文档。
- 创建：`docs/development/2026-08-31-display-font-selection.md` — TDD、构建、安装和实机验收日志。
- 修改：`docs/development/README.md`、字体设计规格和本计划 — 文档索引与完成状态。

### 任务 1：字体选择领域、偏好存储与 AppModel 状态

**文件：**
- 创建：`Sources/AIMeterCore/Preferences/DisplayFontChoice.swift`
- 创建：`Tests/AIMeterCoreTests/DisplayFontPreferenceTests.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 修改：`Tests/AIMeterAppTests/AppModelStartupTests.swift`

- [ ] **步骤 1：编写字体偏好失败测试**

创建 `DisplayFontPreferenceTests.swift`：

```swift
import Foundation
import Testing
@testable import AIMeterCore

@Suite("Display font preference")
struct DisplayFontPreferenceTests {
    @Test("Defines stable identifiers and labels for exactly three choices")
    func choices() {
        #expect(DisplayFontChoice.allCases == [.system, .antonio, .dinCondensed])
        #expect(DisplayFontChoice.system.rawValue == "system")
        #expect(DisplayFontChoice.antonio.rawValue == "antonio")
        #expect(DisplayFontChoice.dinCondensed.rawValue == "din-condensed")
        #expect(DisplayFontChoice.system.displayName == "System Default")
        #expect(DisplayFontChoice.antonio.displayName == "Antonio")
        #expect(DisplayFontChoice.dinCondensed.displayName == "DIN Condensed")
    }

    @Test("Defaults to system and round-trips a supported selection")
    func roundTrip() {
        let suite = "DisplayFontPreferenceTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let store = DisplayFontPreferenceStore(defaults: defaults)

        #expect(store.load() == .system)
        store.save(.antonio)
        #expect(store.load() == .antonio)
        store.save(.system)
        #expect(store.load() == .system)
    }

    @Test("Corrupt persisted values recover to system")
    func corruptValue() {
        let suite = "DisplayFontPreferenceTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        defaults.set("unknown-font", forKey: DisplayFontPreferenceStore.key)

        #expect(DisplayFontPreferenceStore(defaults: defaults).load() == .system)
    }
}
```

- [ ] **步骤 2：运行测试确认红灯**

运行：

```bash
swift test --filter DisplayFontPreferenceTests
```

预期：编译失败，提示 `DisplayFontChoice` 与 `DisplayFontPreferenceStore` 不存在。

- [ ] **步骤 3：实现最小领域与存储**

在 `DisplayFontChoice.swift` 实现：

```swift
import Foundation

public enum DisplayFontChoice: String, CaseIterable, Codable, Identifiable, Sendable {
    case system
    case antonio
    case dinCondensed = "din-condensed"

    public var id: String { rawValue }

    public var displayName: String {
        switch self {
        case .system: "System Default"
        case .antonio: "Antonio"
        case .dinCondensed: "DIN Condensed"
        }
    }
}

public struct DisplayFontPreferenceStore {
    public static let key = "appearance.displayFont"
    private let defaults: UserDefaults

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    public func load() -> DisplayFontChoice {
        defaults.string(forKey: Self.key)
            .flatMap(DisplayFontChoice.init(rawValue:)) ?? .system
    }

    public func save(_ choice: DisplayFontChoice) {
        defaults.set(choice.rawValue, forKey: Self.key)
    }
}
```

- [ ] **步骤 4：运行核心测试确认绿灯**

运行：

```bash
swift test --filter DisplayFontPreferenceTests
```

预期：3 项测试全部通过。

- [ ] **步骤 5：编写 AppModel 字体状态失败测试**

在 `AppModelStartupTests` 增加：

```swift
@Test("Display font defaults, updates, and persists without restarting")
@MainActor
func displayFontPreference() {
    let suiteName = "AppModelStartupTests.Font.\(UUID().uuidString)"
    let defaults = UserDefaults(suiteName: suiteName)!
    defer { defaults.removePersistentDomain(forName: suiteName) }
    let secretStore = ReadCountingSecretStore()

    let model = AppModel(defaults: defaults, secretStore: secretStore)
    #expect(model.displayFontChoice == .system)

    model.setDisplayFontChoice(.antonio)
    #expect(model.displayFontChoice == .antonio)
    #expect(DisplayFontPreferenceStore(defaults: defaults).load() == .antonio)

    model.restoreDefaultDisplayFont()
    #expect(model.displayFontChoice == .system)
}
```

- [ ] **步骤 6：运行 AppModel 测试确认红灯**

运行：

```bash
swift test --filter AppModelStartupTests
```

预期：编译失败，提示 `displayFontChoice`、`setDisplayFontChoice` 和 `restoreDefaultDisplayFont` 不存在。

- [ ] **步骤 7：把偏好接入 AppModel**

在 `AppModel` 中：

```swift
private let displayFontPreferenceStore: DisplayFontPreferenceStore
private(set) var displayFontChoice: DisplayFontChoice
```

初始化时使用同一 `defaults` 创建 store 并 `load()`；增加：

```swift
func setDisplayFontChoice(_ choice: DisplayFontChoice) {
    displayFontChoice = choice
    displayFontPreferenceStore.save(choice)
}

func restoreDefaultDisplayFont() {
    setDisplayFontChoice(.system)
}
```

不得把字体可用性放进 AppModel；AppKit 可用性属于表现层。

- [ ] **步骤 8：运行任务 1 测试确认绿灯并提交**

运行：

```bash
swift test --filter DisplayFontPreferenceTests
swift test --filter AppModelStartupTests
```

预期：字体偏好和启动测试全部通过。

提交：

```bash
git add Sources/AIMeterCore/Preferences/DisplayFontChoice.swift \
  Sources/AIMeterApp/AppModel.swift \
  Tests/AIMeterCoreTests/DisplayFontPreferenceTests.swift \
  Tests/AIMeterAppTests/AppModelStartupTests.swift
git commit -m "feat: persist global display font preference"
```

### 任务 2：字体可用性、语义字体环境与全视图迁移

**文件：**
- 创建：`Sources/AIMeterApp/Views/AIMeterTypography.swift`
- 创建：`Tests/AIMeterAppTests/TypographyTests.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift`
- 修改：`Sources/AIMeterApp/Views/MenuBarPanel.swift`
- 修改：`Sources/AIMeterApp/Views/ProviderCard.swift`
- 修改：`Sources/AIMeterApp/Views/CodexDetailView.swift`
- 修改：`Sources/AIMeterApp/Views/CodexResetCreditsView.swift`
- 修改：`Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift`
- 修改：`Sources/AIMeterApp/Views/UsageRing.swift`

- [ ] **步骤 1：编写字体目录和解析失败测试**

创建 `TypographyTests.swift`：

```swift
import AIMeterCore
import Testing
@testable import AIMeterApp

@Suite("AI Meter typography")
struct TypographyTests {
    @Test("System is always available while custom choices use registered families")
    func availability() {
        let catalog = DisplayFontCatalog(availableFamilies: ["Antonio"])
        #expect(catalog.isAvailable(.system))
        #expect(catalog.isAvailable(.antonio))
        #expect(!catalog.isAvailable(.dinCondensed))
    }

    @Test("Unavailable saved custom fonts resolve safely to system")
    func fallback() {
        let catalog = DisplayFontCatalog(availableFamilies: [])
        #expect(AIMeterTypography.resolvedFamily(for: .antonio, catalog: catalog) == nil)
        #expect(AIMeterTypography.resolvedFamily(for: .dinCondensed, catalog: catalog) == nil)
    }

    @Test("Available choices resolve to the approved family names")
    func familyNames() {
        let catalog = DisplayFontCatalog(
            availableFamilies: ["Antonio", "DIN Condensed"]
        )
        #expect(AIMeterTypography.resolvedFamily(for: .antonio, catalog: catalog) == "Antonio")
        #expect(AIMeterTypography.resolvedFamily(for: .dinCondensed, catalog: catalog) == "DIN Condensed")
    }

    @Test("Semantic roles preserve the existing visual hierarchy")
    func semanticSizes() {
        #expect(AIMeterTextStyle.largeTitle.pointSize > AIMeterTextStyle.title2.pointSize)
        #expect(AIMeterTextStyle.title2.pointSize > AIMeterTextStyle.headline.pointSize)
        #expect(AIMeterTextStyle.headline.pointSize > AIMeterTextStyle.caption.pointSize)
        #expect(AIMeterTextStyle.caption.pointSize > AIMeterTextStyle.caption2.pointSize)
    }
}
```

- [ ] **步骤 2：运行测试确认红灯**

运行：

```bash
swift test --filter TypographyTests
```

预期：编译失败，提示字体目录、解析器和语义样式不存在。

- [ ] **步骤 3：实现最小字体目录与语义解析层**

创建 `AIMeterTypography.swift`，职责保持集中：

```swift
import AIMeterCore
import AppKit
import SwiftUI

struct DisplayFontCatalog: Sendable {
    let availableFamilies: Set<String>

    init(availableFamilies: Set<String>) {
        self.availableFamilies = availableFamilies
    }

    @MainActor static var live: Self {
        Self(availableFamilies: Set(NSFontManager.shared.availableFontFamilies))
    }

    func isAvailable(_ choice: DisplayFontChoice) -> Bool {
        switch choice {
        case .system: true
        case .antonio: availableFamilies.contains("Antonio")
        case .dinCondensed: availableFamilies.contains("DIN Condensed")
        }
    }
}

enum AIMeterTextStyle: CaseIterable, Sendable {
    case largeTitle, title, title2, title3, headline, subheadline, body, caption, caption2

    var swiftUIStyle: Font.TextStyle {
        switch self {
        case .largeTitle: .largeTitle
        case .title: .title
        case .title2: .title2
        case .title3: .title3
        case .headline: .headline
        case .subheadline: .subheadline
        case .body: .body
        case .caption: .caption
        case .caption2: .caption2
        }
    }

    var pointSize: CGFloat {
        switch self {
        case .largeTitle: 34
        case .title: 28
        case .title2: 22
        case .title3: 20
        case .headline: 17
        case .subheadline: 15
        case .body: 13
        case .caption: 12
        case .caption2: 11
        }
    }
}

enum AIMeterTypography {
    static func resolvedFamily(
        for choice: DisplayFontChoice,
        catalog: DisplayFontCatalog
    ) -> String? {
        guard catalog.isAvailable(choice) else { return nil }
        switch choice {
        case .system: nil
        case .antonio: "Antonio"
        case .dinCondensed: "DIN Condensed"
        }
    }
}
```

再定义：

- `AIMeterFontToken`：语义样式或固定点数、相对样式、weight、system design；
- `AIMeterDisplayFontChoiceKey`：环境默认 `.system`；
- `.aiMeterFont(...)` modifier：从环境读取 choice，系统选择使用现有 system/rounded 设计，自定义选择使用 `Font.custom(_:size:relativeTo:)`；不可用时自动走系统分支；
- `.aiMeterFontScope(_:)`：向子树注入选择。

Headline 默认映射 semibold，显式 weight 优先；固定尺寸保留现有 `14`、`15` 和圆环比例尺寸。自定义字体仍传 `relativeTo` 以保留辅助功能文字缩放。

- [ ] **步骤 4：运行字体解析测试确认绿灯**

运行：

```bash
swift test --filter TypographyTests
```

预期：4 项测试全部通过。

- [ ] **步骤 5：把可观察字体作用域接入全部根视图**

在以下根视图的最外层内容使用：

```swift
.aiMeterFontScope(model.displayFontChoice)
```

接入位置：

- `MenuBarLabel.body`；
- `MenuBarPanel.body`；
- `SettingsView.body`；
- `FloatingStripView.body`；
- `FloatingDetailView.body`。

`FloatingStripView` 与 `FloatingDetailView` 必须在自己的 `body` 内从 `@Bindable model` 读取选择，不能只在 `NSHostingView` 创建时捕获初始值，否则 AppKit 浮窗不会即时刷新。

- [ ] **步骤 6：机械迁移所有文字字体调用**

把 `Sources/AIMeterApp` 视图中的 `.font(...)` 逐项替换为语义 modifier：

```swift
.font(.headline)
// 变为
.aiMeterFont(.headline)

.font(.system(.title2, design: .rounded, weight: .bold))
// 变为
.aiMeterFont(.title2, design: .rounded, weight: .bold)

.font(.system(size: 15, weight: .semibold))
// 变为
.aiMeterFont(size: 15, relativeTo: .body, weight: .semibold)
```

对整个容器使用的 `.font(.caption2)` 改为 `.aiMeterFont(.caption2)`；不修改 `Image`、Provider Logo、SF Symbols 或 WebKit 内容。

运行源码扫描：

```bash
rg -n "\\.font\\(" Sources/AIMeterApp --glob '*.swift'
```

预期：除 `AIMeterTypography.swift` 内集中构造 `Font` 的代码外无直接 `.font(...)` 调用。

- [ ] **步骤 7：运行视觉、详情和拖动回归**

运行：

```bash
swift test --filter TypographyTests
swift test --filter VisualSystemTests
swift test --filter FloatingStripDragShapeTests
swift test --filter FloatingStripPointerDragStateTests
swift test --filter CodexDetailPanelLayoutTests
```

预期：字体测试、背景镜像、Provider 配色、轮廓、拖动和详情尺寸测试全部通过。

- [ ] **步骤 8：提交语义字体交付物**

```bash
git add Sources/AIMeterApp/Views/AIMeterTypography.swift \
  Sources/AIMeterApp/Views/FloatingStripView.swift \
  Sources/AIMeterApp/Views/MenuBarPanel.swift \
  Sources/AIMeterApp/Views/ProviderCard.swift \
  Sources/AIMeterApp/Views/CodexDetailView.swift \
  Sources/AIMeterApp/Views/CodexResetCreditsView.swift \
  Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift \
  Sources/AIMeterApp/Views/UsageRing.swift \
  Tests/AIMeterAppTests/TypographyTests.swift
git commit -m "feat: apply selectable typography across AI Meter"
```

### 任务 3：Settings 字体选择、可用性状态和恢复默认

**文件：**
- 修改：`Sources/AIMeterApp/Views/AIMeterTypography.swift`
- 修改：`Sources/AIMeterApp/Views/SettingsView.swift`
- 修改：`Tests/AIMeterAppTests/TypographyTests.swift`
- 修改：`Tests/AIMeterAppTests/VisualSystemTests.swift`

- [ ] **步骤 1：编写 Settings 展示模型失败测试**

在 `TypographyTests` 增加：

```swift
@Test("Settings always exposes three ordered choices and marks missing fonts")
func settingsOptions() {
    let catalog = DisplayFontCatalog(availableFamilies: ["Antonio"])
    let options = DisplayFontSettingsPresentation.options(catalog: catalog)

    #expect(options.map(\.choice) == [.system, .antonio, .dinCondensed])
    #expect(options[0].isEnabled)
    #expect(options[1].isEnabled)
    #expect(!options[2].isEnabled)
    #expect(options[2].statusText == "Not installed")
}
```

在 `VisualSystemTests` 增加恢复按钮规则测试：

```swift
@Test("Restore default is enabled only for custom selections")
func restoreDefaultFontState() {
    #expect(!DisplayFontSettingsPresentation.canRestore(.system))
    #expect(DisplayFontSettingsPresentation.canRestore(.antonio))
    #expect(DisplayFontSettingsPresentation.canRestore(.dinCondensed))
}
```

- [ ] **步骤 2：运行测试确认红灯**

运行：

```bash
swift test --filter TypographyTests
swift test --filter VisualSystemTests
```

预期：编译失败，提示 `DisplayFontSettingsPresentation` 不存在。

- [ ] **步骤 3：实现最小设置展示模型**

在 `AIMeterTypography.swift` 增加：

```swift
struct DisplayFontOption: Equatable, Identifiable {
    let choice: DisplayFontChoice
    let isEnabled: Bool
    let statusText: String?
    var id: DisplayFontChoice { choice }
}

enum DisplayFontSettingsPresentation {
    static func options(catalog: DisplayFontCatalog) -> [DisplayFontOption] {
        DisplayFontChoice.allCases.map { choice in
            let enabled = catalog.isAvailable(choice)
            return DisplayFontOption(
                choice: choice,
                isEnabled: enabled,
                statusText: enabled ? nil : "Not installed"
            )
        }
    }

    @MainActor static func liveOptions() -> [DisplayFontOption] {
        options(catalog: .live)
    }

    static func canRestore(_ choice: DisplayFontChoice) -> Bool {
        choice != .system
    }
}
```

- [ ] **步骤 4：在 Appearance 中加入选择器与恢复按钮**

在 `SettingsView` 的 Appearance 分区加入：

```swift
Picker(
    "Display font",
    selection: Binding(
        get: { model.displayFontChoice },
        set: { choice in
            guard DisplayFontCatalog.live.isAvailable(choice) else { return }
            model.setDisplayFontChoice(choice)
        }
    )
) {
    ForEach(DisplayFontSettingsPresentation.liveOptions()) { option in
        HStack {
            Text(option.choice.displayName)
            if let status = option.statusText {
                Text(status).foregroundStyle(.secondary)
            }
        }
        .aiMeterFontPreview(option.choice)
        .tag(option.choice)
        .disabled(!option.isEnabled)
    }
}

Button("Restore Default Font") {
    model.restoreDefaultDisplayFont()
}
.disabled(!DisplayFontSettingsPresentation.canRestore(model.displayFontChoice))
```

`.aiMeterFontPreview(_:)` 在 `AIMeterTypography.swift` 内集中调用同一解析器，只用于让选择器每行以自身字体预览；所有其他视图继续使用 `.aiMeterFont`。增加一行 caption 说明切换即时生效、缺失字体需要在 macOS 安装后使用。

- [ ] **步骤 5：运行任务 3 测试和 AppModel 回归**

```bash
swift test --filter TypographyTests
swift test --filter VisualSystemTests
swift test --filter AppModelStartupTests
```

预期：设置展示、恢复规则、偏好即时更新和视觉测试全部通过。

- [ ] **步骤 6：构建调试版并检查 Settings 可访问性树**

运行 `swift build` 或 `bash scripts/build-app.sh`，启动候选版后通过 macOS Computer Use 检查 Settings：

- `Display font` 可访问名称存在；
- 三个选项顺序正确；
- 当前机器 Antonio 与 DIN Condensed 均可选择；
- `Restore Default Font` 的启用状态与当前选择一致。

- [ ] **步骤 7：提交 Settings 交付物**

```bash
git add Sources/AIMeterApp/Views/AIMeterTypography.swift \
  Sources/AIMeterApp/Views/SettingsView.swift \
  Tests/AIMeterAppTests/TypographyTests.swift \
  Tests/AIMeterAppTests/VisualSystemTests.swift
git commit -m "feat: add immediate display font controls"
```

### 任务 4：完整验证、安装、Antonio 最终状态与文档

**文件：**
- 修改：`README.md`
- 修改：`CHANGELOG.md`
- 修改：`docs/user-guide/settings.md`
- 创建：`docs/development/2026-08-31-display-font-selection.md`
- 修改：`docs/development/README.md`
- 修改：`docs/superpowers/specs/2026-08-31-display-font-selection-design.md`
- 修改：`docs/superpowers/plans/2026-08-31-display-font-selection.md`

- [ ] **步骤 1：运行完整自动化测试**

```bash
bash scripts/test.sh
```

预期：全部非环境门控测试通过，0 失败；记录总测试数、测试组数和按设计跳过数。

- [ ] **步骤 2：构建并验证 Release App Bundle**

```bash
bash scripts/build-app.sh
codesign --verify --deep --strict "dist/AI Meter.app"
plutil -lint "dist/AI Meter.app/Contents/Info.plist"
shasum -a 256 "dist/AI Meter.app/Contents/MacOS/AIMeterApp"
```

预期：构建、签名和 plist 校验退出码均为 0；记录候选版 SHA-256。

- [ ] **步骤 3：安全安装候选版**

完全退出 AI Meter，将 `/Applications/AI Meter.app` 移到带时间戳的 `/private/tmp` 备份目录，再用 `ditto` 安装候选版。重新启动后核对：

```bash
codesign --verify --deep --strict "/Applications/AI Meter.app"
plutil -lint "/Applications/AI Meter.app/Contents/Info.plist"
shasum -a 256 "/Applications/AI Meter.app/Contents/MacOS/AIMeterApp"
```

预期：安装版和候选版指纹完全一致，备份路径可恢复。

- [ ] **步骤 4：完成真实字体切换验收**

按顺序验收并截图：

1. 启动时保持现有 `.system`，证明迁移不擅自改变用户界面；
2. 打开 Settings，切换 Antonio，确认 Settings 当前窗口立即变化；
3. 不重启，打开菜单面板与 Claude、Codex、DeepSeek 三个详情，确认英文、数字、余额、日期和小号说明均使用 Antonio 且无截断；
4. 切换 DIN Condensed，重复检查详情与小号文字可读性；
5. 点击 `Restore Default Font`，确认恢复 San Francisco；
6. 再选择 Antonio，退出并重启 App，确认选择持久化；
7. 检查 Logo、SF Symbols、圆环、品牌颜色、深海背景、左右贴边、拖动、详情自动隐藏和 Settings 入口保持原样；
8. 最终状态保持 Antonio，并恢复用户原有右侧 97% 浮动条位置。

- [ ] **步骤 5：补全文档**

文档必须说明：

- 默认字体仍是 macOS System Default；
- Antonio 与 DIN Condensed 必须先在 macOS 安装，AI Meter 不下载或分发字体；
- 设置位置、即时生效和恢复默认操作；
- 字体缺失时选项禁用与安全回退；
- TDD 红绿证据、测试数量、Release 构建、签名、SHA-256、备份路径和实机验收；
- 当前机器最终选择 Antonio。

把设计规格状态改为“已实施并验收”，将本计划所有已完成步骤勾为 `[x]`。

- [ ] **步骤 6：提交文档检查点**

```bash
git add README.md CHANGELOG.md docs/user-guide/settings.md \
  docs/development/README.md \
  docs/development/2026-08-31-display-font-selection.md \
  docs/superpowers/specs/2026-08-31-display-font-selection-design.md \
  docs/superpowers/plans/2026-08-31-display-font-selection.md
git commit -m "docs: record display font selection acceptance"
```

- [ ] **步骤 7：完成前最终验证**

```bash
bash scripts/test.sh
bash scripts/build-app.sh
codesign --verify --deep --strict "dist/AI Meter.app"
plutil -lint "dist/AI Meter.app/Contents/Info.plist"
git diff --check
git status --short --branch
```

预期：完整测试 0 失败，Release 构建和签名通过，`git diff --check` 无输出，功能分支工作树干净。随后按 `finishing-a-development-branch` 流程让用户选择本地合并、创建 PR 或保留分支；未经选择不得推送远端或删除工作树。
