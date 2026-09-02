# 浮岛轮廓、拖动与设置入口回归修复实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 恢复已批准的反向半圆贴边轮廓，移除短横并让玻璃背景可靠拖动，同时修复菜单栏设置入口。

**架构：** 用单条可镜像的基准路径定义浮岛轮廓，用独立 Shape 定义排除服务按钮后的拖动命中区；现有面板控制器继续负责移动和吸附。设置按钮使用可测试的顺序命令，先激活 `LSUIElement` 应用，再调用 SwiftUI 官方 `OpenSettingsAction`。

**技术栈：** Swift 6、SwiftUI、AppKit、Swift Testing、Swift Package Manager、macOS 14+

---

## 文件结构

- 修改 `Sources/AIMeterApp/Views/FloatingStripShape.swift`：实现批准原型的反向半圆连续路径。
- 创建 `Sources/AIMeterApp/Views/FloatingStripDragShape.swift`：定义玻璃背景拖动命中区并排除三个服务圆环。
- 修改 `Sources/AIMeterApp/Views/FloatingStripView.swift`：移除短横，把拖动、光标、键盘和 VoiceOver 移到背景表面。
- 创建 `Sources/AIMeterApp/System/SettingsPresentationCommand.swift`：封装“激活应用 → 打开设置”的确定顺序。
- 修改 `Sources/AIMeterApp/Views/MenuBarPanel.swift`：用普通按钮调用官方 `openSettings` 环境动作。
- 修改 `Tests/AIMeterAppTests/VisualSystemTests.swift`：保护反向半圆关键内外点和左右镜像。
- 创建 `Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift`：保护空白背景可拖动、Logo 区域不拖动。
- 创建 `Tests/AIMeterAppTests/SettingsPresentationCommandTests.swift`：保护设置动作顺序。
- 修改 `docs/development/2026-08-31-visual-system-edge-docking.md`、`docs/development/commit-history.md`、`CHANGELOG.md`：记录根因、修复、测试、构建和安装证据。

### 任务 1：恢复反向半圆连续轮廓

**文件：**
- 修改：`Tests/AIMeterAppTests/VisualSystemTests.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripShape.swift`

- [ ] **步骤 1：编写会抓住“肩部再次变方”的失败测试**

在 `floatingStripBounds()` 之后加入独立测试。它用手工选定的批准路径关键点断言，不复用生产路径计算：

```swift
@Test("Floating island uses the approved reverse semicircle shoulders")
func floatingStripReverseSemicircleShoulders() {
    let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
    let right = FloatingStripShape(edge: .right).path(in: rect)
    let left = FloatingStripShape(edge: .left).path(in: rect)

    #expect(right.contains(CGPoint(x: 70, y: 8)))
    #expect(!right.contains(CGPoint(x: 45, y: 45)))
    #expect(right.contains(CGPoint(x: 70, y: 348)))
    #expect(!right.contains(CGPoint(x: 45, y: 311)))

    #expect(left.contains(CGPoint(x: 38, y: 8)))
    #expect(!left.contains(CGPoint(x: 63, y: 45)))
    #expect(left.contains(CGPoint(x: 38, y: 348)))
    #expect(!left.contains(CGPoint(x: 63, y: 311)))
}
```

这条测试要抓的生产破坏是：把批准路径重新近似成顶部/底部矩形肩部。

- [ ] **步骤 2：运行测试并确认正确失败**

运行：

```bash
swift test --filter VisualSystemTests/floatingStripReverseSemicircleShoulders
```

预期：测试断言失败；至少一个批准路径内点被当前近似路径排除，或一个应在缺口中的点被包含。

- [ ] **步骤 3：用基准坐标实现最少连续路径**

把 `FloatingStripShape.path(in:)` 改为按 `108 × 356` 基准缩放的一条路径。保留现有 `point` 水平镜像函数，右侧路径依次使用以下节点：

```swift
path.move(to: point(108, 0))
path.addLine(to: point(89, 0))
path.addCurve(to: point(58, 31), control1: point(67, 0), control2: point(58, 11))
path.addLine(to: point(58, 41))
path.addCurve(to: point(28, 68), control1: point(58, 59), control2: point(48, 68))
path.addLine(to: point(18, 68))
path.addCurve(to: point(0, 89), control1: point(7, 68), control2: point(0, 77))
path.addLine(to: point(0, 267))
path.addCurve(to: point(18, 288), control1: point(0, 279), control2: point(7, 288))
path.addLine(to: point(28, 288))
path.addCurve(to: point(58, 315), control1: point(48, 288), control2: point(58, 297))
path.addLine(to: point(58, 325))
path.addCurve(to: point(89, 356), control1: point(58, 345), control2: point(67, 356))
path.addLine(to: point(108, 356))
path.closeSubpath()
```

`point` 先把基准坐标分别乘以 `rect.width / 108` 和 `rect.height / 356`，再按 `.left` 水平镜像。

- [ ] **步骤 4：运行轮廓测试确认转绿**

运行：

```bash
swift test --filter VisualSystemTests
```

预期：`VisualSystemTests` 全部通过，包括完整边界、透明边缘和反向半圆关键点。

- [ ] **步骤 5：提交轮廓修复**

```bash
git add Sources/AIMeterApp/Views/FloatingStripShape.swift Tests/AIMeterAppTests/VisualSystemTests.swift
git commit -m "fix: restore reverse semicircle island shape"
```

### 任务 2：移除短横并启用玻璃背景拖动

**文件：**
- 创建：`Sources/AIMeterApp/Views/FloatingStripDragShape.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift`
- 创建：`Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift`

- [ ] **步骤 1：编写拖动命中区失败测试**

创建测试，手工断言空白玻璃点可拖、三个 Logo 中心不可拖、透明缺口不可拖：

```swift
import AIMeterCore
import CoreGraphics
import Testing
@testable import AIMeterApp

@Suite("Floating strip drag region")
struct FloatingStripDragShapeTests {
    @Test("Glass background drags while provider buttons remain click-only")
    func dragRegionExcludesProviderButtons() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
        let right = FloatingStripDragShape(edge: .right).path(in: rect)

        #expect(right.contains(CGPoint(x: 86, y: 40), eoFill: true))
        #expect(right.contains(CGPoint(x: 54, y: 142), eoFill: true))
        #expect(!right.contains(CGPoint(x: 54, y: 106), eoFill: true))
        #expect(!right.contains(CGPoint(x: 54, y: 178), eoFill: true))
        #expect(!right.contains(CGPoint(x: 54, y: 250), eoFill: true))
        #expect(!right.contains(CGPoint(x: 20, y: 30), eoFill: true))
    }

    @Test("Left drag region mirrors the right region")
    func dragRegionMirrors() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
        let left = FloatingStripDragShape(edge: .left).path(in: rect)

        #expect(left.contains(CGPoint(x: 22, y: 40), eoFill: true))
        #expect(!left.contains(CGPoint(x: 54, y: 178), eoFill: true))
        #expect(!left.contains(CGPoint(x: 88, y: 30), eoFill: true))
    }
}
```

这两条测试要抓的生产破坏是：拖动再次只绑定到小手柄，或拖动层覆盖 Logo 按钮。

- [ ] **步骤 2：运行测试并确认类型缺失失败**

运行：

```bash
swift test --filter FloatingStripDragShapeTests
```

预期：编译失败，提示找不到 `FloatingStripDragShape`。

- [ ] **步骤 3：实现最小拖动 Shape**

创建 `FloatingStripDragShape`：

```swift
struct FloatingStripDragShape: Shape {
    let edge: FloatingStripEdge

    func path(in rect: CGRect) -> Path {
        var path = FloatingStripShape(edge: edge).path(in: rect)
        let scaleX = rect.width / 108
        let scaleY = rect.height / 356
        for centerY in [106.0, 178.0, 250.0] {
            path.addEllipse(in: CGRect(
                x: 24 * scaleX,
                y: (centerY - 30) * scaleY,
                width: 60 * scaleX,
                height: 60 * scaleY
            ))
        }
        return path
    }
}
```

Logo 均在中央轴线上，左右镜像后的排除圆仍相同；调用方必须使用 even-odd 命中规则。

- [ ] **步骤 4：运行命中区测试确认转绿**

运行：

```bash
swift test --filter FloatingStripDragShapeTests
```

预期：2 个测试全部通过。

- [ ] **步骤 5：把手势和无障碍动作移到背景**

在 `FloatingStripView` 中删除 `Capsule`。给 `FloatingStripSurface` 依次添加：

```swift
.contentShape(FloatingStripDragShape(edge: displayState.resolvedEdge), eoFill: true)
.gesture(stripDragGesture)
.focusable()
.accessibilityLabel("Move floating meter")
.accessibilityValue(accessibilityPositionValue)
```

把现有方向键、adjustable action 和左右自定义 action 原样迁移到背景。提取：

```swift
private var stripDragGesture: some Gesture {
    DragGesture(minimumDistance: 1, coordinateSpace: .global)
        .onChanged { value in
            displayState.isDragging = true
            NSCursor.closedHand.set()
            onStripDragChanged(value.translation)
        }
        .onEnded { value in
            displayState.isDragging = false
            NSCursor.openHand.set()
            onStripDragEnded(value.translation)
        }
}
```

使用 `onContinuousHover` 在 Shape 区域进入/离开时设置 `openHand`/`arrow`。`VStack` 只保留三个服务按钮和现有 12 点间距。

- [ ] **步骤 6：运行相关与完整测试**

运行：

```bash
swift test --filter FloatingStripDragShapeTests
swift test --filter InteractivePanelTests
bash scripts/test.sh
```

预期：新测试通过；完整测试比基线增加 2 个，0 个失败。

- [ ] **步骤 7：提交拖动修复**

```bash
git add Sources/AIMeterApp/Views/FloatingStripDragShape.swift Sources/AIMeterApp/Views/FloatingStripView.swift Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift
git commit -m "fix: drag the floating meter from its glass surface"
```

### 任务 3：可靠打开设置窗口

**文件：**
- 创建：`Sources/AIMeterApp/System/SettingsPresentationCommand.swift`
- 修改：`Sources/AIMeterApp/Views/MenuBarPanel.swift`
- 创建：`Tests/AIMeterAppTests/SettingsPresentationCommandTests.swift`

- [ ] **步骤 1：编写设置动作顺序失败测试**

```swift
import Testing
@testable import AIMeterApp

@Suite("Settings presentation command")
struct SettingsPresentationCommandTests {
    @Test("Activates the menu bar app before opening settings")
    @MainActor
    func activationPrecedesOpening() {
        var events: [String] = []
        let command = SettingsPresentationCommand(
            activateApplication: { events.append("activate") },
            openSettings: { events.append("open") }
        )

        command.perform()

        #expect(events == ["activate", "open"])
    }
}
```

这条测试要抓的生产破坏是：菜单动作只请求设置场景，却没有先让无 Dock 图标应用获得前台展示机会。

- [ ] **步骤 2：运行测试并确认类型缺失失败**

运行：

```bash
swift test --filter SettingsPresentationCommandTests
```

预期：编译失败，提示找不到 `SettingsPresentationCommand`。

- [ ] **步骤 3：实现最小顺序命令**

```swift
@MainActor
struct SettingsPresentationCommand {
    let activateApplication: () -> Void
    let openSettings: () -> Void

    func perform() {
        activateApplication()
        openSettings()
    }
}
```

- [ ] **步骤 4：运行设置命令测试确认转绿**

运行：

```bash
swift test --filter SettingsPresentationCommandTests
```

预期：1 个测试通过。

- [ ] **步骤 5：接通菜单按钮与官方环境动作**

在 `MenuBarPanel` 添加：

```swift
@Environment(\.openSettings) private var openSettings
```

把 `SettingsLink` 替换为普通按钮：

```swift
Button {
    SettingsPresentationCommand(
        activateApplication: {
            NSApplication.shared.activate(ignoringOtherApps: true)
        },
        openSettings: { openSettings() }
    ).perform()
} label: {
    Image(systemName: "gearshape")
}
```

保持现有 `.buttonStyle(.borderless)` 和帮助文本。

- [ ] **步骤 6：运行设置与完整测试**

运行：

```bash
swift test --filter SettingsPresentationCommandTests
bash scripts/test.sh
```

预期：新测试通过；完整测试比上一任务增加 1 个，0 个失败。

- [ ] **步骤 7：提交设置入口修复**

```bash
git add Sources/AIMeterApp/System/SettingsPresentationCommand.swift Sources/AIMeterApp/Views/MenuBarPanel.swift Tests/AIMeterAppTests/SettingsPresentationCommandTests.swift
git commit -m "fix: reliably present settings from the menu bar"
```

### 任务 4：文档、发布构建、安装与真实验收

**文件：**
- 修改：`docs/development/2026-08-31-visual-system-edge-docking.md`
- 修改：`docs/development/commit-history.md`
- 修改：`CHANGELOG.md`

- [ ] **步骤 1：补充开发记录**

记录四项根因、三个修复提交、测试总数、Release 构建、签名、构建/安装哈希和左右实机验收结果。`CHANGELOG.md` 在 `Unreleased / Fixed` 下加入用户可见变化。

- [ ] **步骤 2：运行文档与敏感信息检查**

运行：

```bash
git diff --check
ruby -e 'ok=true; Dir.glob("**/*.md", File::FNM_DOTMATCH).each do |file|; File.read(file).scan(/\[[^\]]*\]\(([^)]+)\)/).flatten.each do |target|; path=target.split("#",2).first; next if path.empty? || path =~ /\A(?:https?:|mailto:)/; resolved=File.expand_path(path.gsub("%20", " "), File.dirname(file)); unless File.exist?(resolved); warn "Missing link: #{file} -> #{target}"; ok=false; end; end; end; abort "Markdown link validation failed" unless ok; puts "Markdown relative links: OK"'
```

预期：无空白错误，所有相对链接存在。

- [ ] **步骤 3：运行完整测试和正式构建**

```bash
bash scripts/test.sh
bash scripts/build-app.sh
codesign --verify --deep --strict --verbose=2 "dist/AI Meter.app"
plutil -lint "dist/AI Meter.app/Contents/Info.plist"
file "dist/AI Meter.app/Contents/MacOS/AIMeterApp" "dist/AI Meter.app/Contents/Resources/AppIcon.icns"
```

预期：141 个测试、31 个测试组、0 个失败；Release 构建、签名、plist、arm64 可执行文件和 ICNS 全部通过。

- [ ] **步骤 4：安装并核对产物**

先退出当前 `AIMeterApp`，把 `/Applications/AI Meter.app` 移到带时间戳的 `/private/tmp` 可恢复备份，再使用 `ditto` 安装新构建。运行：

```bash
codesign --verify --deep --strict --verbose=2 "/Applications/AI Meter.app"
shasum -a 256 "dist/AI Meter.app/Contents/MacOS/AIMeterApp" "/Applications/AI Meter.app/Contents/MacOS/AIMeterApp"
shasum -a 256 "dist/AI Meter.app/Contents/Resources/AppIcon.icns" "/Applications/AI Meter.app/Contents/Resources/AppIcon.icns"
```

预期：严格签名通过，构建和安装的可执行文件及图标哈希分别一致。

- [ ] **步骤 5：真实界面逐项验收**

启动已安装应用并验证：

1. 右侧和左侧都是反向半圆连续轮廓，没有方块、框线、接缝或透明贴边空隙；
2. Claude 上方没有短横；
3. 从顶部、环间和底部玻璃空白处都能拖动；
4. 三个 Logo 点击只打开详情，不启动拖动；
5. Automatic 可跨边，Left/Right 只垂直移动，重启后位置保留；
6. 菜单栏齿轮打开设置，重复点击置前同一设置场景。

- [ ] **步骤 6：请求独立代码审查并处理反馈**

以 `96ad67b` 为基线、当前 HEAD 为终点，审查规格覆盖、形状几何、SwiftUI 命中优先级、设置窗口生命周期、可访问性、测试诚实性和文档。修复所有 Critical/Important 问题并重新运行相关测试。

- [ ] **步骤 7：提交收尾记录**

```bash
git add CHANGELOG.md docs/development/2026-08-31-visual-system-edge-docking.md docs/development/commit-history.md
git commit -m "docs: record floating strip regression verification"
```

- [ ] **步骤 8：合并并做主分支回归**

把 `codex/fix-floating-strip-regressions` 合并到 `main`，在 `main` 再运行 `bash scripts/test.sh`、`bash scripts/build-app.sh` 和严格签名验证。确认主分支干净后移除本 worktree 和已合并分支。
