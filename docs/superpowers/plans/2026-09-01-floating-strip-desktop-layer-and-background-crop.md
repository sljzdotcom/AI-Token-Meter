# AI Token Meter 浮动条桌面层与背景裁切实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现本计划。每个任务严格执行红灯、绿灯、重构、提交，并使用复选框（`- [ ]`）跟踪进度。

**目标：** 让浮动条与详情成为真正的桌面层内容：普通应用和全屏应用可以自然覆盖它；同时在不修改原始深海图片的前提下，以 `1.22×` 等比裁切消除上下黑边，使背景连续覆盖完整上下肩部。

**架构：** 新增一个纯 AppKit 表现策略，集中定义桌面窗口层级与 Space 集合行为；新增一个可注入通知中心的活动 Space 观察器，控制器收到通知后统一关闭详情并重新定位。背景表现层仅返回带方向的等比缩放，`FloatingStripSurface` 在完整 S 形轮廓内一次性合成玻璃、图片与暗色遮罩。原始 PNG 始终作为只读输入资产。

**技术栈：** Swift 6、AppKit `NSPanel`/`NSWorkspace`、SwiftUI、Swift Testing、`ImageRenderer`、Swift Package Manager、macOS Computer Use、Release App Bundle 与 ad-hoc 签名。

**设计规格：** `docs/superpowers/specs/2026-09-01-floating-strip-desktop-layer-and-background-crop-design.md`

---

## 文件结构

- 创建：`Sources/AIMeterApp/System/FloatingPanelPresentationPolicy.swift` — 桌面级别与集合行为的唯一来源。
- 创建：`Tests/AIMeterAppTests/FloatingPanelPresentationPolicyTests.swift` — 层级、Space 行为与面板应用测试。
- 创建：`Sources/AIMeterApp/System/ActiveSpaceChangeObserver.swift` — 可注销、可注入的活动 Space 通知桥接器。
- 创建：`Tests/AIMeterAppTests/ActiveSpaceChangeObserverTests.swift` — 通知传递、注销和重复通知测试。
- 修改：`Sources/AIMeterApp/System/FloatingPanelController.swift` — 应用策略、监听 Space、关闭详情和重新定位。
- 修改：`Sources/AIMeterApp/Views/FloatingStripBackground.swift` — `1.22×` 等比缩放与单一轮廓裁切。
- 修改：`Tests/AIMeterAppTests/VisualSystemTests.swift` — 合成黑边图片的肩部像素、等比缩放与左右镜像回归。
- 修改：`README.md`、`CHANGELOG.md`、`docs/user-guide/settings.md`、`docs/user-guide/troubleshooting.md` — 用户可见行为和故障说明。
- 创建：`docs/development/2026-09-01-floating-strip-desktop-layer-and-background-crop.md` — TDD、构建、安装和实机验收日志。
- 修改：`docs/development/README.md`、`docs/next-phase-requirements.md`、设计规格和本计划 — 索引、状态与验收证据。

## 受保护资产基线

实现前后都必须执行：

```bash
shasum -a 256 Sources/AIMeterApp/Resources/Backgrounds/floating-strip-deep-sea.png
git diff --exit-code -- Sources/AIMeterApp/Resources/Backgrounds/floating-strip-deep-sea.png
```

预期 SHA-256：

```text
43ae960bf58a5ddcf2b416362c00d7bfcdcc5764f9af50316285454b2a813b6d
```

禁止修改、重新导出或覆盖该文件。

---

## 任务 1：建立统一的桌面层窗口策略

**文件：**
- 创建：`Sources/AIMeterApp/System/FloatingPanelPresentationPolicy.swift`
- 创建：`Tests/AIMeterAppTests/FloatingPanelPresentationPolicyTests.swift`
- 修改：`Sources/AIMeterApp/System/FloatingPanelController.swift`

- [ ] **步骤 1：编写窗口策略失败测试**

创建 `FloatingPanelPresentationPolicyTests.swift`：

```swift
import AppKit
import Testing
@testable import AIMeterApp

@Suite("Floating panel presentation policy")
struct FloatingPanelPresentationPolicyTests {
    @Test("Desktop panels remain below ordinary app windows")
    func desktopLevel() {
        let desktopIcons = Int(CGWindowLevelForKey(.desktopIconWindow))

        #expect(FloatingPanelPresentationPolicy.level.rawValue > desktopIcons)
        #expect(FloatingPanelPresentationPolicy.level.rawValue < NSWindow.Level.normal.rawValue)
    }

    @Test("Desktop panels join ordinary Spaces without entering full screen")
    func collectionBehavior() {
        let behavior = FloatingPanelPresentationPolicy.collectionBehavior

        #expect(behavior.contains(.canJoinAllSpaces))
        #expect(behavior.contains(.stationary))
        #expect(behavior.contains(.ignoresCycle))
        #expect(!behavior.contains(.fullScreenAuxiliary))
    }

    @Test("The same policy is applied to strip and detail panels")
    @MainActor
    func appliesToEveryPanel() {
        let first = NSPanel()
        let second = NSPanel()

        FloatingPanelPresentationPolicy.apply(to: first)
        FloatingPanelPresentationPolicy.apply(to: second)

        #expect(first.level == FloatingPanelPresentationPolicy.level)
        #expect(second.level == first.level)
        #expect(first.collectionBehavior == FloatingPanelPresentationPolicy.collectionBehavior)
        #expect(second.collectionBehavior == first.collectionBehavior)
    }
}
```

- [ ] **步骤 2：运行定向测试确认红灯**

运行：

```bash
bash scripts/test.sh --filter FloatingPanelPresentationPolicyTests
```

预期：编译失败，提示 `FloatingPanelPresentationPolicy` 不存在。

- [ ] **步骤 3：实现最小窗口策略**

创建 `FloatingPanelPresentationPolicy.swift`：

```swift
import AppKit

enum FloatingPanelPresentationPolicy {
    static let level = NSWindow.Level(
        rawValue: Int(CGWindowLevelForKey(.desktopIconWindow)) + 1
    )

    static let collectionBehavior: NSWindow.CollectionBehavior = [
        .canJoinAllSpaces,
        .stationary,
        .ignoresCycle,
    ]

    @MainActor
    static func apply(to panel: NSPanel) {
        panel.level = level
        panel.collectionBehavior = collectionBehavior
    }
}
```

- [ ] **步骤 4：让控制器只从策略读取桌面语义**

在 `FloatingPanelController.makePanel(nonactivating:)` 中删除：

```swift
panel.level = .floating
panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary]
```

在其余面板属性设置完后调用：

```swift
FloatingPanelPresentationPolicy.apply(to: panel)
```

保留 `nonactivating`、透明、无阴影、`hidesOnDeactivate` 和键盘能力，不为 strip/detail 建立不同层级。

- [ ] **步骤 5：运行定向与交互回归测试确认绿灯**

运行：

```bash
bash scripts/test.sh --filter FloatingPanelPresentationPolicyTests
bash scripts/test.sh --filter InteractivePanelTests
```

预期：新策略 3 项测试和既有交互面板测试全部通过。

- [ ] **步骤 6：提交任务 1**

```bash
git add Sources/AIMeterApp/System/FloatingPanelPresentationPolicy.swift Sources/AIMeterApp/System/FloatingPanelController.swift Tests/AIMeterAppTests/FloatingPanelPresentationPolicyTests.swift
git commit -m "fix: place floating panels on desktop layer"
```

---

## 任务 2：在 Space 变化时安全关闭详情

**文件：**
- 创建：`Sources/AIMeterApp/System/ActiveSpaceChangeObserver.swift`
- 创建：`Tests/AIMeterAppTests/ActiveSpaceChangeObserverTests.swift`
- 修改：`Sources/AIMeterApp/System/FloatingPanelController.swift`

- [ ] **步骤 1：编写观察器生命周期失败测试**

创建 `ActiveSpaceChangeObserverTests.swift`：

```swift
import Foundation
import Testing
@testable import AIMeterApp

@Suite("Active Space change observer")
struct ActiveSpaceChangeObserverTests {
    @Test("Every active Space notification is delivered once")
    @MainActor
    func delivery() {
        let center = NotificationCenter()
        let name = Notification.Name("ActiveSpaceChangeObserverTests.delivery")
        var deliveries = 0
        let observer = ActiveSpaceChangeObserver(center: center, name: name) {
            deliveries += 1
        }

        center.post(name: name, object: nil)
        center.post(name: name, object: nil)

        #expect(deliveries == 2)
        observer.invalidate()
    }

    @Test("Invalidation stops future delivery and is idempotent")
    @MainActor
    func invalidation() {
        let center = NotificationCenter()
        let name = Notification.Name("ActiveSpaceChangeObserverTests.invalidation")
        var deliveries = 0
        let observer = ActiveSpaceChangeObserver(center: center, name: name) {
            deliveries += 1
        }

        observer.invalidate()
        observer.invalidate()
        center.post(name: name, object: nil)

        #expect(deliveries == 0)
    }
}
```

- [ ] **步骤 2：运行测试确认红灯**

```bash
bash scripts/test.sh --filter ActiveSpaceChangeObserverTests
```

预期：编译失败，提示 `ActiveSpaceChangeObserver` 不存在。

- [ ] **步骤 3：实现可注入且可注销的通知桥接器**

创建 `ActiveSpaceChangeObserver.swift`：

```swift
import AppKit

@MainActor
final class ActiveSpaceChangeObserver {
    private let center: NotificationCenter
    private var token: NSObjectProtocol?

    init(
        center: NotificationCenter = NSWorkspace.shared.notificationCenter,
        name: Notification.Name = NSWorkspace.activeSpaceDidChangeNotification,
        onChange: @escaping @MainActor () -> Void
    ) {
        self.center = center
        token = center.addObserver(forName: name, object: nil, queue: .main) { _ in
            MainActor.assumeIsolated {
                onChange()
            }
        }
    }

    func invalidate() {
        guard let token else { return }
        center.removeObserver(token)
        self.token = nil
    }

    isolated deinit {
        if let token {
            center.removeObserver(token)
        }
    }
}
```

若 Swift 6 对 `isolated deinit` 捕获属性提出隔离错误，只做满足同一契约的最小调整：让 `deinit` 调用线程安全的 `NotificationCenter.removeObserver`；不得改用永不注销的 selector 观察器。

- [ ] **步骤 4：将 Space 生命周期接入控制器**

在 `FloatingPanelController` 增加：

```swift
private var activeSpaceObserver: ActiveSpaceChangeObserver?
```

在初始化完成面板、会话回调和初次定位后注册：

```swift
activeSpaceObserver = ActiveSpaceChangeObserver { [weak self] in
    self?.handleActiveSpaceChange()
}
```

新增：

```swift
private func handleActiveSpaceChange() {
    session.dismiss()
    detailPanel.makeFirstResponder(nil)
    detailPanel.orderOut(nil)
    positionPanels()
    if stripPanel.isVisible {
        stripPanel.orderFrontRegardless()
    }
}
```

在 `deinit` 中显式 `activeSpaceObserver?.invalidate()`，便于代码审查识别生命周期边界。处理函数不得调用任何保存位置的方法，因此 Space 切换不会改写屏幕、侧边或垂直百分比。

- [ ] **步骤 5：运行观察器、详情会话与位置回归测试**

```bash
bash scripts/test.sh --filter ActiveSpaceChangeObserverTests
bash scripts/test.sh --filter FloatingDetailSessionTests
bash scripts/test.sh --filter FloatingStripPositionTests
```

预期：通知每次只转发一次；注销后不再转发；详情关闭和位置持久化既有测试继续通过。

- [ ] **步骤 6：提交任务 2**

```bash
git add Sources/AIMeterApp/System/ActiveSpaceChangeObserver.swift Sources/AIMeterApp/System/FloatingPanelController.swift Tests/AIMeterAppTests/ActiveSpaceChangeObserverTests.swift
git commit -m "fix: dismiss floating detail across Space changes"
```

---

## 任务 3：等比裁切深海背景并覆盖上下肩部

**文件：**
- 修改：`Sources/AIMeterApp/Views/FloatingStripBackground.swift`
- 修改：`Tests/AIMeterAppTests/VisualSystemTests.swift`

- [ ] **步骤 1：先写缩放合同失败测试**

在 `VisualSystemTests` 增加：

```swift
@Test("Background crop uses equal axis magnitudes and mirrors only X")
func floatingBackgroundScale() {
    let right = FloatingStripBackgroundPresentation.scale(for: .right)
    let left = FloatingStripBackgroundPresentation.scale(for: .left)

    #expect(right.width == 1.22)
    #expect(right.height == 1.22)
    #expect(left.width == -right.width)
    #expect(left.height == right.height)
    #expect(abs(left.width) == abs(left.height))
}
```

- [ ] **步骤 2：加入带黑边合成图的肩部像素失败测试**

在同一测试文件增加：

```swift
@Test("Uniform crop removes black gutters from both visible shoulders")
@MainActor
func floatingBackgroundFillsShoulders() throws {
    let background = try blackGutterBlueCenterImage()

    for edge in [FloatingStripEdge.left, .right] {
        let rendered = try renderSurface(edge: edge, backgroundImage: background)
        let attachedX = edge == .right ? 107 : 0
        for y in [20, 336] {
            let color = try rgb(atX: attachedX, y: y, in: rendered)
            #expect(color.blue > 100)
            #expect(color.blue > color.red)
        }
    }
}
```

并增加工厂方法，生成 `108 × 356` RGBA 图片：顶部 `0..<32` 和底部 `324..<356` 为黑色，中间为纯蓝色，alpha 全为 255。该比例模拟受保护 3× 资源中的约 96px 黑边。

- [ ] **步骤 3：运行视觉测试确认红灯**

```bash
bash scripts/test.sh --filter VisualSystemTests
```

预期：缩放 API 尚不存在；若仅临时补 API 而不改渲染，肩部像素仍为黑色，证明测试能够捕获原问题。

- [ ] **步骤 4：实现方向明确的统一等比缩放**

将 `FloatingStripBackgroundPresentation` 改为：

```swift
enum FloatingStripBackgroundPresentation {
    static let scrimOpacity = 0.38
    static let contentScale: CGFloat = 1.22

    static func scale(for edge: FloatingStripEdge) -> CGSize {
        CGSize(
            width: edge == .left ? -contentScale : contentScale,
            height: contentScale
        )
    }
}
```

删除旧的 `horizontalScale(for:)`，避免同时存在两个缩放来源。

- [ ] **步骤 5：在完整 S 形中一次性合成并裁切**

把 `FloatingStripSurface.body` 重组为一个 `ZStack`：

```swift
ZStack {
    FloatingStripShape(edge: edge)
        .fill(AIMeterVisualTheme.floatingGlass)

    if let backgroundImage {
        let scale = FloatingStripBackgroundPresentation.scale(for: edge)
        Image(nsImage: backgroundImage)
            .resizable()
            .scaledToFill()
            .scaleEffect(x: scale.width, y: scale.height, anchor: .center)
            .overlay {
                Color.black.opacity(FloatingStripBackgroundPresentation.scrimOpacity)
            }
            .accessibilityHidden(true)
    }
}
.clipShape(FloatingStripShape(edge: edge))
```

图片、scrim 和 fallback 共享同一个最终形状裁切。不得把 `.scaleEffect` 放到包含 Provider Logo/圆环的 `FloatingStripView` 上；镜像范围必须仍只限于背景 Surface。

- [ ] **步骤 6：运行全部视觉、拖动与交互回归测试**

```bash
bash scripts/test.sh --filter VisualSystemTests
bash scripts/test.sh --filter FloatingStripDragShapeTests
bash scripts/test.sh --filter FloatingStripPointerDragStateTests
```

预期：上下肩部为蓝色内容；左右背景相反；形状边界、fallback、无外部阴影、Logo 光学校准和拖动测试继续通过。

- [ ] **步骤 7：验证受保护资产并提交任务 3**

```bash
shasum -a 256 Sources/AIMeterApp/Resources/Backgrounds/floating-strip-deep-sea.png
git diff --exit-code -- Sources/AIMeterApp/Resources/Backgrounds/floating-strip-deep-sea.png
git add Sources/AIMeterApp/Views/FloatingStripBackground.swift Tests/AIMeterAppTests/VisualSystemTests.swift
git commit -m "fix: crop strip background across both shoulders"
```

预期哈希仍为 `43ae960bf58a5ddcf2b416362c00d7bfcdcc5764f9af50316285454b2a813b6d`。

---

## 任务 4：全量验证、实机验收、文档与候选安装

**文件：**
- 修改：`README.md`
- 修改：`CHANGELOG.md`
- 修改：`docs/user-guide/settings.md`
- 修改：`docs/user-guide/troubleshooting.md`
- 创建：`docs/development/2026-09-01-floating-strip-desktop-layer-and-background-crop.md`
- 修改：`docs/development/README.md`
- 修改：`docs/next-phase-requirements.md`
- 修改：`docs/superpowers/specs/2026-09-01-floating-strip-desktop-layer-and-background-crop-design.md`
- 修改：`docs/superpowers/plans/2026-09-01-floating-strip-desktop-layer-and-background-crop.md`

- [ ] **步骤 1：运行完整测试并记录精确结果**

```bash
bash scripts/test.sh
```

预期：不少于基线 170 项测试，新测试全部通过，0 failures。把最终测试数、套件数、耗时和命令记录到开发日志，不写“应该通过”。

- [ ] **步骤 2：执行静态合同与资产保护检查**

```bash
rg -n "fullScreenAuxiliary|panel.level = \.floating" Sources/AIMeterApp/System
shasum -a 256 Sources/AIMeterApp/Resources/Backgrounds/floating-strip-deep-sea.png
git diff --exit-code -- Sources/AIMeterApp/Resources/Backgrounds/floating-strip-deep-sea.png
```

预期：源代码不再命中旧的全屏辅助或浮动层赋值；背景哈希精确匹配基线；资产无 diff。

- [ ] **步骤 3：构建、签名并生成候选 App**

先阅读现有构建脚本帮助和最近开发日志，复用项目既有命令，不发明第二套打包流程。至少执行：

```bash
bash scripts/build-app.sh
codesign --verify --deep --strict build/AI\ Meter.app
```

若脚本输出的实际 App 路径不同，以脚本返回路径为准。记录 Release 构建结果、签名验证和 bundle 位置。

- [ ] **步骤 4：安装候选版并做桌面层实机验收**

安装前先安全退出当前 AI Meter；复用项目既有安装脚本或将候选 App 复制到当前安装位置，保留现有 UserDefaults。逐项记录：

1. Finder 桌面可见且三枚 Logo 可点击；
2. 普通 Edge 窗口覆盖浮动条；
3. Edge 全屏完全看不到浮动条和详情；
4. 返回桌面后仍是用户原有 Antonio、右侧、97% 垂直位置；
5. 打开详情后切换 Space，返回时详情已关闭；
6. Mission Control 没有悬浮在 Space 缩略图之上的异常窗口；
7. 左右贴边的上下肩部都有连续蓝色波纹；
8. Logo、圆环、点击、拖动、设置菜单和自动隐藏均正常；
9. 有第二显示器则验收跨屏；没有则明确写“当前环境未覆盖多显示器”。

实机失败时回到对应任务增加失败测试，不允许只在日志里标注已知问题后继续合并。

- [ ] **步骤 5：补全用户文档和开发日志**

文档至少包含：

- README：浮动条属于桌面层，普通/全屏应用会覆盖；
- Settings：左右贴边与垂直位置仍保留，桌面层行为不是可切换偏好；
- Troubleshooting：为何全屏应用中看不到浮动条，以及如何返回桌面确认；
- CHANGELOG：桌面层修复、Space 详情关闭、背景肩部连续性；
- 开发日志：根因、设计链接、TDD 红绿结果、最终测试数、资产哈希、构建签名、安装指纹、实机矩阵；
- 下一阶段需求：R2/R6 标记完成，R1/R3/R4/R5 保持未完成；
- 规格与本计划：状态更新为已实现，并附提交与验收摘要。

- [ ] **步骤 6：提交文档与验收证据**

```bash
git add README.md CHANGELOG.md docs/user-guide/settings.md docs/user-guide/troubleshooting.md docs/development/2026-09-01-floating-strip-desktop-layer-and-background-crop.md docs/development/README.md docs/next-phase-requirements.md docs/superpowers/specs/2026-09-01-floating-strip-desktop-layer-and-background-crop-design.md docs/superpowers/plans/2026-09-01-floating-strip-desktop-layer-and-background-crop.md
git commit -m "docs: record desktop-only strip behavior"
```

- [ ] **步骤 7：执行完成前验证和独立代码审查**

使用 `verification-before-completion` 重新运行全量测试、Release 构建、签名、资产哈希、工作区状态与安装指纹检查。然后使用 `requesting-code-review` 审查：

- 策略是否确实低于普通窗口且不含 `.fullScreenAuxiliary`；
- Space 观察器是否注销、详情是否关闭、位置是否不被改写；
- 背景是否真正等比而非纵向拉伸；
- 原始 PNG 是否无变化；
- strip/detail/DeepSeek 焦点和拖动是否无回归；
- 文档是否与实际验收一致。

审查发现问题时先修复并重新验证；只有 0 个阻断问题时才能进入合并步骤。

- [ ] **步骤 8：按完成分支流程合并到 `main`**

使用 `finishing-a-development-branch` 检查提交历史和分支差异。合并前再次确认 `main` 没有用户新增改动；若有，先安全整合并重跑验证。合并后在 `main` 上运行至少一次完整测试，并确认已安装 App 与最终候选构建一致。

---

## 完成判定

- 窗口策略自动化测试证明桌面层低于普通窗口，且没有全屏辅助行为；
- Space 通知自动化测试证明可传递、可注销，实机证明详情在 Space 切换后关闭；
- 合成图片测试证明上下肩部均取得蓝色内容，左右仅水平镜像，X/Y 绝对缩放一致；
- 170 项基线测试与新增测试全绿，Release 构建和签名通过；
- 原始深海 PNG 哈希未变、Git 无二进制 diff；
- Finder、普通 Edge、全屏 Edge、Mission Control、左右 Space 和左右贴边均完成实机验收；
- 用户的 Antonio、右侧、97% 垂直位置及现有交互无回归；
- README、用户指南、故障排除、CHANGELOG、开发日志和需求状态全部同步；
- 独立审查无阻断问题，最终变更安全合并到 `main`。
