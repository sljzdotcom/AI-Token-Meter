# AI Meter 视觉系统与贴边浮岛实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 把 AI Meter 改造成可左右吸附、可拖动记忆位置的贴边浮岛，统一三种详情页的深色玻璃视觉，校正服务 Logo 的视觉大小，并加入无文字仪表指针 App Icon。

**架构：** 在 `AIMeterCore` 中保存与平台无关的侧边偏好和持久化位置，在 `AIMeterApp` 中用纯几何布局器把偏好解析为 `NSPanel` 坐标；控制器只协调拖动、屏幕变化和详情展开。SwiftUI 通过集中视觉主题、自定义浮岛 Shape 和品牌光学校正规则消费状态，打包脚本从确定性 Swift 绘制器生成 App Icon。

**技术栈：** Swift 6、SwiftUI、AppKit `NSPanel`/`NSScreen`、Observation、UserDefaults、Core Graphics、Swift Testing、macOS `iconutil`/`codesign`。

---

## 文件结构

### 新建

- `Sources/AIMeterCore/Preferences/FloatingStripPosition.swift`：侧边偏好、已解析侧边、持久化位置与 UserDefaults 存储。
- `Sources/AIMeterApp/System/FloatingStripLayout.swift`：纯几何布局、吸附侧解析、垂直位置换算与详情面板定位。
- `Sources/AIMeterApp/System/FloatingStripDisplayState.swift`：控制器与 SwiftUI 共享的当前侧边和拖动状态。
- `Sources/AIMeterApp/Views/AIMeterVisualTheme.swift`：颜色、表面、间距、圆角、阴影与玻璃卡组件。
- `Sources/AIMeterApp/Views/FloatingStripShape.swift`：可左右镜像的无缝反向半圆浮岛轮廓。
- `Sources/AIMeterApp/Views/ProviderLogoStyle.swift`：Claude、Codex、DeepSeek 的集中光学校正。
- `scripts/generate-app-icon.swift`：按规格绘制无文字仪表指针并生成完整 iconset。
- `Tests/AIMeterCoreTests/FloatingStripPositionTests.swift`：默认值、持久化、损坏数据与夹紧测试。
- `Tests/AIMeterAppTests/FloatingStripLayoutTests.swift`：左右贴边、自动吸附、垂直夹紧、详情展开与屏幕回退测试。
- `Tests/AIMeterAppTests/VisualSystemTests.swift`：浮岛镜像边界、视觉尺寸规则和 App Icon 配置测试。
- `docs/development/2026-08-31-visual-system-edge-docking.md`：失败证据、实现、视觉、测试、安装和 Git 节点日志。

### 修改

- `Sources/AIMeterApp/AppModel.swift`：暴露侧边偏好、保存位置，并通知窗口控制器立即重排。
- `Sources/AIMeterApp/System/FloatingPanelController.swift`：使用纯布局器、处理拖动、屏幕切换和左右详情展开。
- `Sources/AIMeterApp/Views/FloatingStripView.swift`：新浮岛 Shape、拖动柄、统一玻璃表面和详情容器。
- `Sources/AIMeterApp/Views/ProviderLogo.swift`：应用集中光学校正。
- `Sources/AIMeterApp/Views/UsageRing.swift`：统一 60 点圆环与渐变状态呈现。
- `Sources/AIMeterApp/Views/CodexDetailView.swift`：应用统一详情面板、卡片和文字层级。
- `Sources/AIMeterApp/Views/CodexResetCreditsView.swift`：使用统一券卡表面和状态颜色。
- `Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift`：应用统一详情面板、统计卡和图表颜色。
- `Sources/AIMeterApp/Views/ProviderCard.swift`：菜单卡复用统一表面和 Logo 校正。
- `Sources/AIMeterApp/Views/SettingsView.swift`：增加 Automatic / Left / Right，并修正“right-side”旧文案。
- `Sources/AIMeterApp/Resources/Info.plist`：声明 `AppIcon.icns`。
- `scripts/build-app.sh`：生成并安装 App Icon 后再签名。
- `README.md`、`CHANGELOG.md`、`docs/user-guide/settings.md`、`docs/user-guide/getting-started.md`、`docs/architecture/repository-structure.md`、`docs/development/testing.md`、`docs/development/README.md`、`docs/development/commit-history.md`：更新功能、操作、结构和验证记录。

## 基线

- 分支：`codex/visual-system-edge-docking`
- 工作区：`.worktrees/visual-system-edge-docking`
- 起点：`515d91e`
- `bash scripts/test.sh`：113 个测试、26 个套件、0 失败；4 个环境依赖检查按设计跳过。

---

### 任务 1：侧边偏好与位置持久化

**文件：**
- 创建：`Sources/AIMeterCore/Preferences/FloatingStripPosition.swift`
- 创建：`Tests/AIMeterCoreTests/FloatingStripPositionTests.swift`

- [ ] **步骤 1：编写失败的偏好和存储测试**

```swift
@Suite("Floating strip position")
struct FloatingStripPositionTests {
    @Test("Defaults to automatic on the right at vertical center")
    func defaults() {
        let defaults = UserDefaults(suiteName: #function)!
        defaults.removePersistentDomain(forName: #function)
        let value = FloatingStripPositionStore(defaults: defaults).load()
        #expect(value.preference == .automatic)
        #expect(value.lastResolvedEdge == .right)
        #expect(value.normalizedCenterY == 0.5)
    }

    @Test("Clamps corrupt vertical positions and rejects unknown edges")
    func corruptValues() {
        let defaults = UserDefaults(suiteName: #function)!
        defaults.set("future", forKey: "floatingStrip.edgePreference")
        defaults.set(4.0, forKey: "floatingStrip.normalizedCenterY")
        let value = FloatingStripPositionStore(defaults: defaults).load()
        #expect(value.preference == .automatic)
        #expect(value.normalizedCenterY == 1)
    }
}
```

- [ ] **步骤 2：运行测试确认类型不存在**

运行：`bash scripts/test.sh --filter FloatingStripPositionTests`

预期：编译失败，提示找不到 `FloatingStripPositionStore` 和相关类型。

- [ ] **步骤 3：实现最小的值类型和存储**

```swift
public enum FloatingStripEdgePreference: String, CaseIterable, Codable, Sendable {
    case automatic, left, right
}

public enum FloatingStripEdge: String, Codable, Sendable {
    case left, right
}

public struct FloatingStripPosition: Equatable, Sendable {
    public var preference: FloatingStripEdgePreference
    public var lastResolvedEdge: FloatingStripEdge
    public var normalizedCenterY: Double
    public var screenIdentifier: String?
}

public struct FloatingStripPositionStore: Sendable {
    public func load() -> FloatingStripPosition
    public func save(_ position: FloatingStripPosition)
}
```

实现要求：缺失或未知枚举值回退到 `.automatic` / `.right`；`normalizedCenterY` 在读写时夹紧到 `0...1`；屏幕标识允许为空；键名集中在存储类型内部。

- [ ] **步骤 4：补充保存再读取测试并运行通过**

运行：`bash scripts/test.sh --filter FloatingStripPositionTests`

预期：该套件通过，且测试结束时清理 suite UserDefaults。

- [ ] **步骤 5：提交位置模型**

```bash
git add Sources/AIMeterCore/Preferences/FloatingStripPosition.swift Tests/AIMeterCoreTests/FloatingStripPositionTests.swift
git commit -m "feat: persist floating strip position"
```

---

### 任务 2：纯几何贴边布局

**文件：**
- 创建：`Sources/AIMeterApp/System/FloatingStripLayout.swift`
- 创建：`Tests/AIMeterAppTests/FloatingStripLayoutTests.swift`

- [ ] **步骤 1：编写失败的左右贴边、自动吸附和详情展开测试**

```swift
@Suite("Floating strip layout")
struct FloatingStripLayoutTests {
    let visible = CGRect(x: 100, y: 50, width: 1200, height: 800)
    let strip = CGSize(width: 108, height: 356)

    @Test("Places the island flush against either screen edge")
    func flushEdges() {
        #expect(FloatingStripLayout.stripFrame(in: visible, size: strip, edge: .right, normalizedCenterY: 0.5).maxX == visible.maxX)
        #expect(FloatingStripLayout.stripFrame(in: visible, size: strip, edge: .left, normalizedCenterY: 0.5).minX == visible.minX)
    }

    @Test("Automatic chooses the nearest side while fixed choices win")
    func resolvesEdge() {
        #expect(FloatingStripLayout.resolvedEdge(preference: .automatic, current: .right, proposedMidX: 200, visibleFrame: visible) == .left)
        #expect(FloatingStripLayout.resolvedEdge(preference: .right, current: .left, proposedMidX: 200, visibleFrame: visible) == .right)
    }

    @Test("Detail always opens toward the desktop interior")
    func detailDirection() {
        let rightStrip = FloatingStripLayout.stripFrame(in: visible, size: strip, edge: .right, normalizedCenterY: 0.5)
        let frame = FloatingStripLayout.detailFrame(size: CGSize(width: 390, height: 520), stripFrame: rightStrip, edge: .right, visibleFrame: visible)
        #expect(frame.maxX < rightStrip.minX)
    }
}
```

- [ ] **步骤 2：运行测试确认布局器不存在**

运行：`bash scripts/test.sh --filter FloatingStripLayoutTests`

预期：编译失败，提示找不到 `FloatingStripLayout`。

- [ ] **步骤 3：实现无 AppKit 状态依赖的布局器**

```swift
enum FloatingStripLayout {
    static let detailGap: CGFloat = 9

    static func stripFrame(
        in visibleFrame: CGRect,
        size: CGSize,
        edge: FloatingStripEdge,
        normalizedCenterY: Double
    ) -> CGRect

    static func resolvedEdge(
        preference: FloatingStripEdgePreference,
        current: FloatingStripEdge,
        proposedMidX: CGFloat,
        visibleFrame: CGRect
    ) -> FloatingStripEdge

    static func normalizedCenterY(for frame: CGRect, in visibleFrame: CGRect) -> Double

    static func detailFrame(
        size: CGSize,
        stripFrame: CGRect,
        edge: FloatingStripEdge,
        visibleFrame: CGRect
    ) -> CGRect
}
```

实现要求：浮岛始终完整位于 `visibleFrame`；左右间距为 0；详情间距为 9；详情高度或宽度过大时夹紧到屏幕内侧 8 点；归一化值按可移动中心区间计算，而不是直接除以屏幕高度。

- [ ] **步骤 4：补充顶底夹紧、极小屏幕和归一化往返测试**

运行：`bash scripts/test.sh --filter FloatingStripLayoutTests`

预期：所有几何测试通过，浮点比较使用小于 `0.001` 的容差。

- [ ] **步骤 5：提交纯布局器**

```bash
git add Sources/AIMeterApp/System/FloatingStripLayout.swift Tests/AIMeterAppTests/FloatingStripLayoutTests.swift
git commit -m "feat: calculate edge-docked panel layout"
```

---

### 任务 3：窗口拖动、左右吸附和设置

**文件：**
- 创建：`Sources/AIMeterApp/System/FloatingStripDisplayState.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 修改：`Sources/AIMeterApp/System/FloatingPanelController.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift`
- 修改：`Sources/AIMeterApp/Views/SettingsView.swift`
- 测试：`Tests/AIMeterAppTests/FloatingStripLayoutTests.swift`

- [ ] **步骤 1：先扩展失败测试，覆盖固定侧边与拖动结束解析**

```swift
@Test("Fixed sides ignore horizontal drag but keep the proposed vertical center")
func fixedSideDrag() {
    let result = FloatingStripLayout.resolvedPlacement(
        preference: .left,
        current: .right,
        proposedFrame: CGRect(x: 1100, y: 300, width: 108, height: 356),
        visibleFrame: visible
    )
    #expect(result.edge == .left)
    #expect(result.normalizedCenterY > 0 && result.normalizedCenterY < 1)
}
```

- [ ] **步骤 2：运行测试确认 `resolvedPlacement` 不存在**

运行：`bash scripts/test.sh --filter FloatingStripLayoutTests`

预期：编译失败，提示没有 `resolvedPlacement`。

- [ ] **步骤 3：实现显示状态、AppModel 设置和立即重排回调**

```swift
@MainActor @Observable
final class FloatingStripDisplayState {
    var resolvedEdge: FloatingStripEdge = .right
    var isDragging = false
}

// AppModel
var floatingStripPosition: FloatingStripPosition
var floatingPositionHandler: (() -> Void)?

func setFloatingStripEdgePreference(_ preference: FloatingStripEdgePreference) {
    floatingStripPosition.preference = preference
    positionStore.save(floatingStripPosition)
    floatingPositionHandler?()
}
```

`AppModel` 初始化时从 `FloatingStripPositionStore` 读取；保存拖动结果的方法同时更新内存、UserDefaults 和回调。设置 Picker 使用 `FloatingStripEdgePreference.allCases`，显示 `Automatic`、`Left`、`Right`；显示开关文案改为 `Show floating meter`。

- [ ] **步骤 4：把控制器定位切换到纯布局器**

```swift
private let displayState = FloatingStripDisplayState()
private var dragStartFrame: CGRect?

private func beginStripDrag() { ... }
private func updateStripDrag(translation: CGSize) { ... }
private func endStripDrag() { ... }
private func persistResolvedPlacement(screen: NSScreen, frame: CGRect) { ... }
```

控制器使用 108 × 356 的浮岛窗口；右侧 `maxX == visibleFrame.maxX`，左侧 `minX == visibleFrame.minX`。拖动柄开始时捕获窗口 frame；拖动中允许跨屏幕；结束时按模式解析侧边、弹性动画吸附、保存屏幕编号和归一化位置。屏幕参数变化时优先恢复已保存屏幕，否则回退主屏幕并夹紧。

- [ ] **步骤 5：在拖动柄挂接手势，服务圆环仍只响应点击**

```swift
Capsule()
    .fill(AIMeterVisualTheme.tertiaryText.opacity(0.45))
    .frame(width: 25, height: 3)
    .contentShape(Rectangle().inset(by: -8))
    .gesture(
        DragGesture(coordinateSpace: .global)
            .onChanged(onStripDragChanged)
            .onEnded(onStripDragEnded)
    )
    .accessibilityLabel("Move floating meter")
    .accessibilityHint("Drag vertically, or drag to another edge in Automatic mode")
```

不得把 DragGesture 放到整个浮岛，避免破坏三个 Button 的点击、VoiceOver 和详情开关行为。

- [ ] **步骤 6：运行布局、详情关闭和交互测试**

运行：`bash scripts/test.sh --filter FloatingStripLayoutTests`  
运行：`bash scripts/test.sh --filter FloatingDetailSessionTests`  
运行：`bash scripts/test.sh --filter InteractivePanelTests`

预期：新增布局测试通过；既有详情选择、外部点击、自动隐藏和 DeepSeek 键盘焦点测试保持通过。

- [ ] **步骤 7：提交贴边交互**

```bash
git add Sources/AIMeterApp/AppModel.swift Sources/AIMeterApp/System/FloatingStripDisplayState.swift Sources/AIMeterApp/System/FloatingPanelController.swift Sources/AIMeterApp/Views/FloatingStripView.swift Sources/AIMeterApp/Views/SettingsView.swift Tests/AIMeterAppTests/FloatingStripLayoutTests.swift
git commit -m "feat: dock and drag the floating meter"
```

---

### 任务 4：统一视觉主题、浮岛轮廓和品牌 Logo 大小

**文件：**
- 创建：`Sources/AIMeterApp/Views/AIMeterVisualTheme.swift`
- 创建：`Sources/AIMeterApp/Views/FloatingStripShape.swift`
- 创建：`Sources/AIMeterApp/Views/ProviderLogoStyle.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift`
- 修改：`Sources/AIMeterApp/Views/ProviderLogo.swift`
- 修改：`Sources/AIMeterApp/Views/UsageRing.swift`
- 创建：`Tests/AIMeterAppTests/VisualSystemTests.swift`

- [ ] **步骤 1：编写失败的几何和光学校正测试**

```swift
@Suite("AI Meter visual system")
struct VisualSystemTests {
    @Test("Provider marks are optically balanced")
    func providerScales() {
        #expect(ProviderLogoStyle.opticalScale(for: .claude) > 1)
        #expect(ProviderLogoStyle.opticalScale(for: .codex) == 1)
        #expect(ProviderLogoStyle.opticalScale(for: .deepSeek) < 1)
    }

    @Test("Both island orientations occupy the full panel bounds")
    func shapeBounds() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
        #expect(FloatingStripShape(edge: .right).path(in: rect).boundingRect == rect)
        #expect(FloatingStripShape(edge: .left).path(in: rect).boundingRect == rect)
    }
}
```

- [ ] **步骤 2：运行测试确认视觉类型不存在**

运行：`bash scripts/test.sh --filter VisualSystemTests`

预期：编译失败，提示找不到 `ProviderLogoStyle` 和 `FloatingStripShape`。

- [ ] **步骤 3：实现主题、玻璃卡和反向半圆 Shape**

```swift
enum AIMeterVisualTheme {
    static let glassBase = Color(red: 0.027, green: 0.039, blue: 0.063)
    static let glassElevated = Color(red: 0.051, green: 0.071, blue: 0.114)
    static let cardSurface = Color(red: 0.090, green: 0.114, blue: 0.161)
    static let mintAccent = Color(red: 0.329, green: 0.929, blue: 0.776)
    static let violetAccent = Color(red: 0.467, green: 0.412, blue: 1.0)
    static let panelCornerRadius: CGFloat = 25
    static let cardCornerRadius: CGFloat = 15
}
```

`FloatingStripShape.path(in:)` 用一条连续 Path 绘制顶部内收曲线、中段和底部内收曲线；左侧通过水平镜像同一路径产生，不能叠加独立端帽或描边。主题根据 `accessibilityReduceTransparency` 和 `accessibilityDifferentiateWithoutColor` 提供实色回退和更高对比度。

- [ ] **步骤 4：实现集中光学校正并应用到 ProviderLogo**

```swift
enum ProviderLogoStyle {
    static func opticalScale(for provider: UsageProvider) -> CGFloat {
        switch provider {
        case .claude: 1.16
        case .codex: 1.0
        case .deepSeek: 0.92
        }
    }
}
```

`ProviderLogo` 在固定 frame 内应用 `scaleEffect`；同一规则自动覆盖悬浮条、菜单卡和详情页。视觉验收后只允许以小步调整这三个常量，不在调用点增加第二套倍率。

- [ ] **步骤 5：把浮岛改为 60 点圆环和连续玻璃形状**

```swift
.background {
    FloatingStripShape(edge: displayState.resolvedEdge)
        .fill(AIMeterVisualTheme.floatingGlass)
        .shadow(
            color: .black.opacity(0.34),
            radius: 18,
            x: displayState.resolvedEdge == .right ? -6 : 6,
            y: 8
        )
}
```

删除旧 `RoundedRectangle(cornerRadius: 34)`、12 点窗口外边距和图片背景；状态环继续使用语义颜色，正常状态可使用 mint 到 violet 的克制渐变，警告和危险仍使用现有黄/红色。

- [ ] **步骤 6：运行视觉规则与完整 App 测试**

运行：`bash scripts/test.sh --filter VisualSystemTests`  
运行：`bash scripts/test.sh --filter AIMeterAppTests`

预期：视觉规则测试通过，现有 App 测试无回归。

- [ ] **步骤 7：提交视觉基础设施**

```bash
git add Sources/AIMeterApp/Views/AIMeterVisualTheme.swift Sources/AIMeterApp/Views/FloatingStripShape.swift Sources/AIMeterApp/Views/ProviderLogoStyle.swift Sources/AIMeterApp/Views/FloatingStripView.swift Sources/AIMeterApp/Views/ProviderLogo.swift Sources/AIMeterApp/Views/UsageRing.swift Tests/AIMeterAppTests/VisualSystemTests.swift
git commit -m "feat: unify the floating meter visual system"
```

---

### 任务 5：统一 Claude、Codex 和 DeepSeek 详情页

**文件：**
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift`
- 修改：`Sources/AIMeterApp/Views/CodexDetailView.swift`
- 修改：`Sources/AIMeterApp/Views/CodexResetCreditsView.swift`
- 修改：`Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift`
- 修改：`Sources/AIMeterApp/Views/ProviderCard.swift`
- 测试：`Tests/AIMeterAppTests/VisualSystemTests.swift`

- [ ] **步骤 1：先添加失败的视觉层级常量测试**

```swift
@Test("Panel, card, and capsule geometry forms a strict hierarchy")
func geometryHierarchy() {
    #expect(AIMeterVisualTheme.panelCornerRadius > AIMeterVisualTheme.cardCornerRadius)
    #expect(AIMeterVisualTheme.cardCornerRadius > AIMeterVisualTheme.capsuleInsetRadius)
    #expect(AIMeterVisualTheme.panelPadding == 20)
}
```

- [ ] **步骤 2：运行测试确认缺少完整主题常量**

运行：`bash scripts/test.sh --filter VisualSystemTests`

预期：编译失败，提示缺少 `capsuleInsetRadius` 或 `panelPadding`。

- [ ] **步骤 3：完善主题并统一详情容器**

```swift
extension View {
    func aiMeterDetailSurface() -> some View {
        foregroundStyle(AIMeterVisualTheme.primaryText)
            .padding(AIMeterVisualTheme.panelPadding)
            .background(
                RoundedRectangle(cornerRadius: AIMeterVisualTheme.panelCornerRadius, style: .continuous)
                    .fill(AIMeterVisualTheme.detailGlass)
                    .shadow(color: .black.opacity(0.30), radius: 20, x: 0, y: 9)
            )
    }
}
```

Claude compact detail、Codex 详情和 DeepSeek analytics 都使用同一主表面；卡片统一使用 `cardSurface` 和内侧高光，不加明显白边。主标题/关键数值使用 `primaryText`，标签使用 `secondaryText`，更新时间/来源使用 `tertiaryText`。

- [ ] **步骤 4：按已确认原型调整 Codex 内容层级**

保持现有数据和顺序，只改变呈现：顶部 Logo/标题/23% 关键值；双额度卡；Reset credits 渐变弱底色卡；三项本机统计；底部来源与更新时间。进度条正常状态为 mint 到 violet，过期和警告继续使用语义色。

- [ ] **步骤 5：同步 DeepSeek、Claude 和菜单卡视觉**

DeepSeek 图表卡使用相同表面，柱形图由纯蓝改为 mint/violet 渐变；登录 WebView 保留可见的键盘焦点轮廓。Claude 保持设置按钮的主操作语义。`ProviderCard` 只复用卡片表面与 Logo 校正，不扩大菜单尺寸。

- [ ] **步骤 6：运行 App 测试并做可访问性编译检查**

运行：`bash scripts/test.sh --filter AIMeterAppTests`  
运行：`bash scripts/test.sh --filter AppPresentationTests`

预期：详情交互、Codex 自适应高度、展示模型和视觉层级测试通过。

- [ ] **步骤 7：提交详情页统一改版**

```bash
git add Sources/AIMeterApp/Views/FloatingStripView.swift Sources/AIMeterApp/Views/CodexDetailView.swift Sources/AIMeterApp/Views/CodexResetCreditsView.swift Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift Sources/AIMeterApp/Views/ProviderCard.swift Tests/AIMeterAppTests/VisualSystemTests.swift
git commit -m "feat: restyle provider detail panels"
```

---

### 任务 6：无文字仪表指针 App Icon 与打包

**文件：**
- 创建：`scripts/generate-app-icon.swift`
- 修改：`scripts/build-app.sh`
- 修改：`Sources/AIMeterApp/Resources/Info.plist`
- 测试：`Tests/AIMeterAppTests/VisualSystemTests.swift`

- [ ] **步骤 1：编写失败的 Info.plist 配置测试**

```swift
@Test("App bundle declares the generated meter icon")
func appIconConfiguration() throws {
    let projectRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    let data = try Data(contentsOf: projectRoot.appending(path: "Sources/AIMeterApp/Resources/Info.plist"))
    let plist = try PropertyListSerialization.propertyList(from: data, format: nil) as! [String: Any]
    #expect(plist["CFBundleIconFile"] as? String == "AppIcon")
}
```

- [ ] **步骤 2：运行测试确认当前 plist 没有图标声明**

运行：`bash scripts/test.sh --filter appIconConfiguration`

预期：FAIL，实际值为 `nil`。

- [ ] **步骤 3：实现确定性图标绘制器**

```swift
let sizes = [16, 32, 128, 256, 512]
for size in sizes {
    renderMeterIcon(points: size, scale: 1, destination: iconsetURL)
    renderMeterIcon(points: size, scale: 2, destination: iconsetURL)
}
```

绘制要求：深靛色圆角方形底板、青绿到蓝紫分段环、一根浅色指针；无文字、无服务商标志、无随机参数。脚本使用 Core Graphics/AppKit 生成标准 `icon_16x16.png` 至 `icon_512x512@2x.png`。

- [ ] **步骤 4：在构建脚本生成 icns 并写入 Bundle**

```bash
ICONSET_DIR="$BUILD_DIR/AppIcon.iconset"
swift "$PROJECT_DIR/scripts/generate-app-icon.swift" "$ICONSET_DIR"
iconutil -c icns "$ICONSET_DIR" -o "$CONTENTS_DIR/Resources/AppIcon.icns"
test -s "$CONTENTS_DIR/Resources/AppIcon.icns"
```

`Info.plist` 增加 `<key>CFBundleIconFile</key><string>AppIcon</string>`。图标在签名前生成；任何尺寸缺失、`iconutil` 失败或结果为空都让构建失败。

- [ ] **步骤 5：运行配置测试与 Release 构建验证**

运行：`bash scripts/test.sh --filter appIconConfiguration`  
运行：`bash scripts/build-app.sh`  
运行：`plutil -extract CFBundleIconFile raw "dist/AI Meter.app/Contents/Info.plist"`  
运行：`test -s "dist/AI Meter.app/Contents/Resources/AppIcon.icns"`

预期：测试通过；输出为 `AppIcon`；icns 存在且非空；签名验证继续通过。

- [ ] **步骤 6：提交 App Icon 管线**

```bash
git add scripts/generate-app-icon.swift scripts/build-app.sh Sources/AIMeterApp/Resources/Info.plist Tests/AIMeterAppTests/VisualSystemTests.swift
git commit -m "feat: package the AI Meter app icon"
```

---

### 任务 7：文档、真实视觉验收、安装和分支收尾

**文件：**
- 创建：`docs/development/2026-08-31-visual-system-edge-docking.md`
- 修改：`README.md`
- 修改：`CHANGELOG.md`
- 修改：`docs/assets/ai-meter-floating-strip.jpeg`
- 修改：`docs/user-guide/settings.md`
- 修改：`docs/user-guide/getting-started.md`
- 修改：`docs/architecture/repository-structure.md`
- 修改：`docs/development/testing.md`
- 修改：`docs/development/README.md`
- 修改：`docs/development/commit-history.md`

- [ ] **步骤 1：更新用户与架构文档**

README 截图替换为真实安装版贴边浮岛；功能列表说明左右/自动吸附、拖动柄和新 App Icon。设置指南解释 Automatic/Left/Right 与垂直位置记忆。目录文档加入新布局、主题、Shape、图标生成脚本和测试职责。

- [ ] **步骤 2：补全开发日志与 Changelog**

开发日志记录：旧版 12 点空白和 Logo 视觉大小证据、TDD 红灯/绿灯、最终常量、左右屏幕测试、图标打包、视觉截图、正式构建、安装哈希、Git 节点和已知边界。`CHANGELOG.md` 只写面向用户的 Unreleased 变化，不记录原型过程。

- [ ] **步骤 3：运行完整自动化验证**

运行：`bash scripts/test.sh`

预期：所有测试和套件通过；环境依赖检查只能按既有条件跳过，不得出现新增跳过或失败。把实际测试数量写回 README badge 与测试指南。

- [ ] **步骤 4：运行差异和隐私检查**

运行：`git diff --check`  
运行：`rg -n "AI_METER_VISUAL_CAPTURE|Bearer |sk-[A-Za-z0-9]" Sources Tests docs README.md CHANGELOG.md || true`

预期：无空白错误；无临时视觉钩子；无凭证、Cookie 或账户原始响应。

- [ ] **步骤 5：构建正式安装包并验证**

运行：`bash scripts/build-app.sh`  
运行：`plutil -lint "dist/AI Meter.app/Contents/Info.plist"`  
运行：`codesign --verify --deep --strict --verbose=2 "dist/AI Meter.app"`  
运行：`file "dist/AI Meter.app/Contents/MacOS/AIMeterApp"`  
运行：`test -s "dist/AI Meter.app/Contents/Resources/AppIcon.icns"`

预期：Info.plist、签名、arm64 可执行文件和 AppIcon 均通过。

- [ ] **步骤 6：安装前备份并执行真实界面验收**

安全退出当前 `/Applications/AI Meter.app`，把它移动到唯一的 `/private/tmp/AI Meter.app.pre-visual-system-<timestamp>`，再复制新构建。比较构建版和安装版可执行文件 SHA-256 后启动。

逐项验证：

1. 默认右侧 0 间距贴边，轮廓无框线或接缝；
2. Auto 从右拖到左后吸附、重启后仍在左侧和相同垂直位置；
3. Left/Right 设置立即生效，固定侧边时仍能上下拖动；
4. 多显示器切换和分辨率变化后浮岛保持可见；
5. 详情在两侧都朝桌面内部展开，外部点击和自动隐藏正常；
6. Claude、Codex、DeepSeek Logo 视觉大小接近；
7. 三种详情页与确认原型一致，DeepSeek 登录仍能输入；
8. Dock、Finder 和设置中的仪表指针 App Icon 清晰且无文字；
9. 减少透明度、增强对比度和 VoiceOver 可用。

- [ ] **步骤 7：提交文档和验收证据**

```bash
git add README.md CHANGELOG.md docs
git commit -m "docs: verify visual system and edge docking"
```

- [ ] **步骤 8：请求代码审查并处理发现**

审查范围：`main...HEAD`。重点检查窗口坐标、多显示器回退、UserDefaults 迁移、手势与按钮冲突、可访问性、图标打包和视觉常量是否重复。任何修复单独提交并重新执行步骤 3–6。

- [ ] **步骤 9：准备合并主分支**

确认 `git status --short` 为空、完整测试和 Release 构建是修复后的最新结果，再使用 finishing-a-development-branch 流程本地合入 `main`。合并后在 `main` 再运行完整测试和构建，重新安装最终 main 产物，并在提交历史记录合并节点。
