# 详情浮层自动收起实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 为右侧悬浮仪表的详情窗增加可配置的自动隐藏时间，并在点击浮层之外时立即关闭且同步清除圆环选择状态。

**架构：** `AIMeterCore` 提供可测试的隐藏时间选项、详情会话状态和点击区域判断；`AIMeterApp` 的 `FloatingPanelController` 负责 AppKit 面板、鼠标监听和渲染。SwiftUI 悬浮条与控制器共享同一个会话对象，所有关闭路径统一调用 `dismiss()`。

**技术栈：** Swift 6、SwiftUI、AppKit、Observation、Swift Testing、UserDefaults、SwiftPM

---

## 文件结构

- 创建 `Sources/AIMeterCore/UI/DetailAutoHideInterval.swift`：定义 3/5/8/15/30 秒选项与 8 秒回退规则。
- 创建 `Sources/AIMeterCore/UI/DetailAutoHidePreferenceStore.swift`：封装 UserDefaults 保存、恢复和非法值回退。
- 创建 `Sources/AIMeterCore/UI/FloatingDetailSession.swift`：管理选择、切换、可取消倒计时和状态回调。
- 创建 `Sources/AIMeterCore/UI/FloatingPanelHitPolicy.swift`：用屏幕坐标判断点击是否位于悬浮条或详情面板内。
- 创建 `Tests/AIMeterCoreTests/DetailAutoHideIntervalTests.swift`：验证选项、默认值和非法持久化值。
- 创建 `Tests/AIMeterCoreTests/FloatingDetailSessionTests.swift`：验证切换、超时、旧计时器隔离和点击区域。
- 修改 `Sources/AIMeterApp/AppModel.swift`：读取、保存并规范化详情自动隐藏秒数。
- 修改 `Sources/AIMeterApp/Views/SettingsView.swift`：增加固定选项 Picker。
- 修改 `Sources/AIMeterApp/Views/FloatingStripView.swift`：移除本地重复选择状态，改用共享会话。
- 修改 `Sources/AIMeterApp/System/FloatingPanelController.swift`：接入会话、自动隐藏和本地/全局鼠标监听。
- 修改 `README.md`：记录默认值、设置位置和点击外部关闭行为。
- 修改 `docs/development/2026-08-28-development-log.md`：记录红灯、实现、界面和发布验证证据。

### 任务 1：隐藏时间选项与设置持久化

**文件：**
- 创建：`Sources/AIMeterCore/UI/DetailAutoHideInterval.swift`
- 创建：`Sources/AIMeterCore/UI/DetailAutoHidePreferenceStore.swift`
- 创建：`Tests/AIMeterCoreTests/DetailAutoHideIntervalTests.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 修改：`Sources/AIMeterApp/Views/SettingsView.swift`

- [ ] **步骤 1：编写失败的选项与回退测试**

```swift
import Foundation
import Testing
@testable import AIMeterCore

@Suite("Detail auto-hide interval")
struct DetailAutoHideIntervalTests {
    @Test("Offers the approved fixed choices with eight seconds as default")
    func choicesAndDefault() {
        #expect(DetailAutoHideInterval.allCases.map(\.rawValue) == [3, 5, 8, 15, 30])
        #expect(DetailAutoHideInterval.default.rawValue == 8)
    }

    @Test("Restores supported values and rejects unsupported persisted values")
    func restoresPersistedValue() {
        #expect(DetailAutoHideInterval(storedSeconds: 15).rawValue == 15)
        #expect(DetailAutoHideInterval(storedSeconds: 0) == .default)
        #expect(DetailAutoHideInterval(storedSeconds: 999) == .default)
        #expect(DetailAutoHideInterval(storedSeconds: nil) == .default)
    }

    @Test("Persists a supported choice and falls back from corrupt stored values")
    func persistsChoice() throws {
        let suiteName = "com.millerpan.AIMeter.interval-tests.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let store = DetailAutoHidePreferenceStore(defaults: defaults)

        #expect(store.load() == .default)
        store.save(.fifteenSeconds)
        #expect(DetailAutoHidePreferenceStore(defaults: defaults).load() == .fifteenSeconds)
        defaults.set(999, forKey: "detailAutoHideSeconds")
        #expect(store.load() == .default)
    }
}
```

- [ ] **步骤 2：运行测试并确认红灯原因正确**

运行：

```bash
CLANG_MODULE_CACHE_PATH=/private/tmp/ai-meter-spm/clang-module-cache \
swift test --disable-sandbox \
  --cache-path /private/tmp/ai-meter-spm/cache \
  --config-path /private/tmp/ai-meter-spm/config \
  --security-path /private/tmp/ai-meter-spm/security \
  --scratch-path /private/tmp/ai-meter-spm/build \
  --filter DetailAutoHideIntervalTests
```

预期：编译失败，明确报告 `DetailAutoHideInterval` 不存在。

- [ ] **步骤 3：实现固定选项和安全回退**

```swift
public enum DetailAutoHideInterval: Int, CaseIterable, Identifiable, Sendable {
    case threeSeconds = 3
    case fiveSeconds = 5
    case eightSeconds = 8
    case fifteenSeconds = 15
    case thirtySeconds = 30

    public static let `default` = DetailAutoHideInterval.eightSeconds
    public var id: Int { rawValue }

    public init(storedSeconds: Int?) {
        self = storedSeconds.flatMap(Self.init(rawValue:)) ?? .default
    }
}
```

- [ ] **步骤 4：运行聚焦测试确认绿灯**

运行步骤 2 的相同命令。预期：3 个测试通过，0 个失败。

- [ ] **步骤 5：把设置接入 AppModel 与设置页**

实现 `DetailAutoHidePreferenceStore`：

```swift
import Foundation

public struct DetailAutoHidePreferenceStore {
    private let defaults: UserDefaults
    private let key: String

    public init(
        defaults: UserDefaults = .standard,
        key: String = "detailAutoHideSeconds"
    ) {
        self.defaults = defaults
        self.key = key
    }

    public func load() -> DetailAutoHideInterval {
        DetailAutoHideInterval(storedSeconds: defaults.object(forKey: key) as? Int)
    }

    public func save(_ interval: DetailAutoHideInterval) {
        defaults.set(interval.rawValue, forKey: key)
    }
}
```

在 `AppModel` 中增加 store 和属性，并在初始化时读取：

```swift
private let detailAutoHidePreferenceStore: DetailAutoHidePreferenceStore
var detailAutoHideSeconds: Int

self.detailAutoHidePreferenceStore = DetailAutoHidePreferenceStore(defaults: defaults)
detailAutoHideSeconds = detailAutoHidePreferenceStore.load().rawValue

func setDetailAutoHideSeconds(_ seconds: Int) {
    let interval = DetailAutoHideInterval(storedSeconds: seconds)
    detailAutoHideSeconds = interval.rawValue
    detailAutoHidePreferenceStore.save(interval)
}
```

在 `SettingsView` 的 Appearance 区域增加：

```swift
Picker(
    "Detail auto-hide",
    selection: Binding(
        get: { model.detailAutoHideSeconds },
        set: { model.setDetailAutoHideSeconds($0) }
    )
) {
    ForEach(DetailAutoHideInterval.allCases) { interval in
        Text("\(interval.rawValue) seconds").tag(interval.rawValue)
    }
}
```

- [ ] **步骤 6：构建 App 验证设置接线**

运行：

```bash
CLANG_MODULE_CACHE_PATH=/private/tmp/ai-meter-spm/clang-module-cache \
swift build --disable-sandbox \
  --cache-path /private/tmp/ai-meter-spm/cache \
  --config-path /private/tmp/ai-meter-spm/config \
  --security-path /private/tmp/ai-meter-spm/security \
  --scratch-path /private/tmp/ai-meter-spm/build
```

预期：`Build complete`，无编译错误。

- [ ] **步骤 7：提交任务 1**

```bash
git add Sources/AIMeterCore/UI/DetailAutoHideInterval.swift \
  Sources/AIMeterCore/UI/DetailAutoHidePreferenceStore.swift \
  Tests/AIMeterCoreTests/DetailAutoHideIntervalTests.swift \
  Sources/AIMeterApp/AppModel.swift Sources/AIMeterApp/Views/SettingsView.swift
git commit -m "feat: add configurable detail auto-hide interval"
```

### 任务 2：共享详情会话、计时与外部点击关闭

**文件：**
- 创建：`Sources/AIMeterCore/UI/FloatingDetailSession.swift`
- 创建：`Sources/AIMeterCore/UI/FloatingPanelHitPolicy.swift`
- 创建：`Tests/AIMeterCoreTests/FloatingDetailSessionTests.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift`
- 修改：`Sources/AIMeterApp/System/FloatingPanelController.swift`

- [ ] **步骤 1：编写失败的会话与点击区域测试**

```swift
import CoreGraphics
import Testing
@testable import AIMeterCore

@Suite("Floating detail session")
struct FloatingDetailSessionTests {
    @Test("Tapping providers toggles and switches one shared selection")
    @MainActor
    func togglesSelection() {
        let session = FloatingDetailSession()
        session.toggle(.claude, autoHideAfter: .seconds(30))
        #expect(session.selectedProvider == .claude)
        session.toggle(.codex, autoHideAfter: .seconds(30))
        #expect(session.selectedProvider == .codex)
        session.toggle(.codex, autoHideAfter: .seconds(30))
        #expect(session.selectedProvider == nil)
    }

    @Test("A stale timeout cannot dismiss a newly selected provider")
    @MainActor
    func staleTimeoutDoesNotDismissNewSelection() async throws {
        let session = FloatingDetailSession()
        session.present(.claude, autoHideAfter: .milliseconds(20))
        try await Task.sleep(for: .milliseconds(10))
        session.present(.codex, autoHideAfter: .seconds(1))
        try await Task.sleep(for: .milliseconds(30))
        #expect(session.selectedProvider == .codex)
        session.dismiss()
    }

    @Test("The active provider is cleared when its timeout expires")
    @MainActor
    func timeoutDismisses() async throws {
        let session = FloatingDetailSession()
        session.present(.deepSeek, autoHideAfter: .milliseconds(20))
        try await Task.sleep(for: .milliseconds(50))
        #expect(session.selectedProvider == nil)
    }

    @Test("Only clicks outside both panels request dismissal")
    func hitPolicy() {
        let strip = CGRect(x: 300, y: 100, width: 84, height: 300)
        let detail = CGRect(x: 28, y: 138, width: 262, height: 224)
        #expect(!FloatingPanelHitPolicy.isOutside(CGPoint(x: 320, y: 150), strip: strip, detail: detail))
        #expect(!FloatingPanelHitPolicy.isOutside(CGPoint(x: 100, y: 200), strip: strip, detail: detail))
        #expect(FloatingPanelHitPolicy.isOutside(CGPoint(x: 10, y: 10), strip: strip, detail: detail))
    }
}
```

- [ ] **步骤 2：运行测试并确认红灯原因正确**

运行：

```bash
CLANG_MODULE_CACHE_PATH=/private/tmp/ai-meter-spm/clang-module-cache \
swift test --disable-sandbox \
  --cache-path /private/tmp/ai-meter-spm/cache \
  --config-path /private/tmp/ai-meter-spm/config \
  --security-path /private/tmp/ai-meter-spm/security \
  --scratch-path /private/tmp/ai-meter-spm/build \
  --filter FloatingDetailSessionTests
```

预期：编译失败，报告 `FloatingDetailSession` 和 `FloatingPanelHitPolicy` 不存在。

- [ ] **步骤 3：实现最小共享会话**

```swift
import Foundation
import Observation

@MainActor
@Observable
public final class FloatingDetailSession {
    public private(set) var selectedProvider: UsageProvider?
    @ObservationIgnored public var onSelectionChange: ((UsageProvider?) -> Void)?
    @ObservationIgnored private var autoHideTask: Task<Void, Never>?

    public init() {}

    public func toggle(_ provider: UsageProvider, autoHideAfter duration: Duration) {
        if selectedProvider == provider {
            dismiss()
        } else {
            present(provider, autoHideAfter: duration)
        }
    }

    public func present(_ provider: UsageProvider, autoHideAfter duration: Duration) {
        setSelection(provider)
        autoHideTask?.cancel()
        autoHideTask = Task { [weak self] in
            do { try await Task.sleep(for: duration) } catch { return }
            guard self?.selectedProvider == provider else { return }
            self?.dismiss()
        }
    }

    public func dismiss() {
        autoHideTask?.cancel()
        autoHideTask = nil
        setSelection(nil)
    }

    private func setSelection(_ provider: UsageProvider?) {
        selectedProvider = provider
        onSelectionChange?(provider)
    }
}
```

- [ ] **步骤 4：实现最小点击区域策略**

```swift
import CoreGraphics

public enum FloatingPanelHitPolicy {
    public static func isOutside(
        _ point: CGPoint,
        strip: CGRect,
        detail: CGRect
    ) -> Bool {
        !strip.contains(point) && !detail.contains(point)
    }
}
```

- [ ] **步骤 5：运行聚焦测试确认绿灯**

运行步骤 2 的相同命令。预期：4 个测试通过，0 个失败。

- [ ] **步骤 6：让 SwiftUI 悬浮条使用共享会话**

把 `FloatingStripView` 的本地 `@State selectedProvider` 替换为：

```swift
@Bindable var session: FloatingDetailSession
let onProviderTap: (UsageProvider) -> Void
```

按钮只调用 `onProviderTap(presentation.provider)`，缩放和动画改为观察 `session.selectedProvider`。这样超时和外部点击通过 `session.dismiss()` 后会同步取消圆环高亮。

- [ ] **步骤 7：在 FloatingPanelController 接入会话、渲染与监听**

控制器增加：

```swift
private let session = FloatingDetailSession()
private var localMouseMonitor: Any?
private var globalMouseMonitor: Any?
```

初始化时把圆环点击映射到：

```swift
session.toggle(
    provider,
    autoHideAfter: .seconds(model.detailAutoHideSeconds)
)
```

把 `session.onSelectionChange` 连接到单一 `renderSelection(_:)`：有 provider 时更新 `FloatingDetailView` 并显示面板，nil 时隐藏详情。通知点击使用 `session.present`，`hide()` 使用 `session.dismiss()`。

安装本地与全局鼠标监听：

```swift
localMouseMonitor = NSEvent.addLocalMonitorForEvents(
    matching: [.leftMouseDown, .rightMouseDown]
) { [weak self] event in
    self?.dismissForOutsideClick(at: NSEvent.mouseLocation)
    return event
}

globalMouseMonitor = NSEvent.addGlobalMonitorForEvents(
    matching: [.leftMouseDown, .rightMouseDown]
) { [weak self] _ in
    Task { @MainActor in
        self?.dismissForOutsideClick(at: NSEvent.mouseLocation)
    }
}
```

`dismissForOutsideClick` 仅在详情已选择且 `FloatingPanelHitPolicy.isOutside` 返回 true 时调用 `session.dismiss()`。控制器销毁时移除两个 monitor 和已有 screen observer。

- [ ] **步骤 8：构建并运行聚焦回归**

运行：

```bash
CLANG_MODULE_CACHE_PATH=/private/tmp/ai-meter-spm/clang-module-cache \
swift test --disable-sandbox \
  --cache-path /private/tmp/ai-meter-spm/cache \
  --config-path /private/tmp/ai-meter-spm/config \
  --security-path /private/tmp/ai-meter-spm/security \
  --scratch-path /private/tmp/ai-meter-spm/build \
  --filter 'DetailAutoHideIntervalTests|FloatingDetailSessionTests'
```

随后运行任务 1 步骤 6 的完整构建命令。预期：全部聚焦测试通过且 App 构建成功。

- [ ] **步骤 9：提交任务 2**

```bash
git add Sources/AIMeterCore/UI/FloatingDetailSession.swift \
  Sources/AIMeterCore/UI/FloatingPanelHitPolicy.swift \
  Tests/AIMeterCoreTests/FloatingDetailSessionTests.swift \
  Sources/AIMeterApp/Views/FloatingStripView.swift \
  Sources/AIMeterApp/System/FloatingPanelController.swift
git commit -m "feat: dismiss floating details automatically"
```

### 任务 3：文档、完整验证与本机验收

**文件：**
- 修改：`README.md`
- 修改：`docs/development/2026-08-28-development-log.md`

- [ ] **步骤 1：更新用户说明和开发证据**

在 README 的悬浮条说明中记录：详情默认 8 秒收起、设置位置、点击浮层外立即关闭。在开发日志中记录红灯测试输出、最小实现、测试数量、AppKit 监听降级策略和本机界面验收结果；不记录鼠标坐标、API Key 或服务原始响应。

- [ ] **步骤 2：运行完整自动化套件**

```bash
CLANG_MODULE_CACHE_PATH=/private/tmp/ai-meter-spm/clang-module-cache \
swift test --disable-sandbox \
  --cache-path /private/tmp/ai-meter-spm/cache \
  --config-path /private/tmp/ai-meter-spm/config \
  --security-path /private/tmp/ai-meter-spm/security \
  --scratch-path /private/tmp/ai-meter-spm/build
```

预期：全部测试通过，0 个失败；真实 CLI 与 Keychain 条目在未显式启用时按设计跳过。

- [ ] **步骤 3：生成并验证本机 App 包**

```bash
./scripts/build-app.sh
codesign --verify --deep --strict --verbose=2 'dist/AI Meter.app'
plutil -lint 'dist/AI Meter.app/Contents/Info.plist'
file 'dist/AI Meter.app/Contents/MacOS/AIMeterApp'
git diff --check
```

预期：打包、签名、plist 和 diff 检查均退出 0；二进制为 arm64。

- [ ] **步骤 4：取得启动授权后执行本机界面验收**

AI-Meter 启动会读取已保存的 DeepSeek API Key 并向官方余额接口发起请求。只有在用户明确允许该凭据外发后，才替换 `/Applications/AI Meter.app`、启动应用，并按规格的五项本机界面验收逐一验证。

- [ ] **步骤 5：提交任务 3**

```bash
git add README.md docs/development/2026-08-28-development-log.md
git commit -m "docs: document transient floating details"
```

- [ ] **步骤 6：最终状态检查**

```bash
git status --short
git log --oneline -5
```

预期：工作区干净，三个任务均有独立提交；若本机界面验收因未取得启动授权而待执行，日志必须明确标记为待用户授权，不能宣称已完成。
