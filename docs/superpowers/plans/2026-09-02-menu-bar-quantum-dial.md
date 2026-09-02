# 菜单栏 Quantum Dial 图标实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 用可动态表达最高已用比例的 18×18pt 自绘 Quantum Dial 替换菜单栏顶部通用仪表图标，同时保留百分比文字、辅助功能和既有菜单行为。

**架构：** `AIMeterCore.MenuBarSummary` 增加一个与 `valueText` 同源的可选规范化比例；`AIMeterApp` 新增独立的几何模型和 SwiftUI 绘图视图，菜单栏只负责把该比例传入。几何计算与渲染分离，先用纯测试锁定比例、弧长和指针角度，再验证 18×18pt 像素边界和真实菜单栏表现。

**技术栈：** Swift 6、SwiftUI、Swift Testing、`ImageRenderer`、Swift Package Manager、macOS 14+

---

## 文件结构

- 修改：`Sources/AIMeterCore/Presentation/AppPresentation.swift` — 为 `MenuBarSummary` 输出最高有效已用比例，保持文字、语义与图标共享同一来源。
- 修改：`Tests/AIMeterCoreTests/AppPresentationTests.swift` — 覆盖比例选择、不可用数据排除、夹紧和无数据行为。
- 创建：`Sources/AIMeterApp/Views/MenuBarMeterIcon.swift` — 定义 Quantum Dial 几何与 18×18pt 单色 SwiftUI 视图。
- 创建：`Tests/AIMeterAppTests/MenuBarMeterIconTests.swift` — 覆盖几何端点、代表比例、无数据和像素边界。
- 修改：`Sources/AIMeterApp/Views/MenuBarPanel.swift` — 将菜单栏顶部图标替换为 `MenuBarMeterIcon`，保留弹出面板内部无数据图标。
- 修改：`Tests/AIMeterAppTests/TypographyTests.swift` — 更新图标源码契约，避免把 Quantum Dial 当成语义字体符号。
- 修改：`README.md`、`CHANGELOG.md`、`docs/user-guide/getting-started.md` — 说明菜单栏动态图标行为。
- 创建：`docs/development/2026-09-02-menu-bar-quantum-dial.md` — 记录 TDD、构建、安装、真实菜单栏验收和 Git 证据。
- 修改：`docs/development/README.md`、`docs/README.md`、`docs/development/commit-history.md`、`docs/requirements-backlog.md` — 建立文档入口并完成需求状态。

### 任务 1：让菜单栏摘要输出与文字同源的规范化比例

**文件：**
- 修改：`Sources/AIMeterCore/Presentation/AppPresentation.swift:148-180`
- 修改：`Tests/AIMeterCoreTests/AppPresentationTests.swift:145-180`

- [x] **步骤 1：编写失败的展示模型测试**

在 `AppPresentationTests` 的菜单栏摘要测试中加入比例断言，并新增边界用例：

```swift
@Test("Menu bar summary exposes the same highest fraction as its text")
func summarizesHighestFraction() {
    let summary = MenuBarSummary(snapshots: [
        usageSnapshot(provider: .claude, fraction: 0.23),
        usageSnapshot(provider: .codex, fraction: 0.90),
        usageSnapshot(provider: .deepSeek, fraction: 0.70),
    ])

    #expect(summary.usageFraction == 0.90)
    #expect(summary.valueText == "90%")
}

@Test("Menu bar summary clamps fractions and excludes unavailable providers")
func normalizesMenuBarFraction() {
    let unavailable = UsageSnapshot(
        provider: .claude,
        primaryMetric: UsageMetric(
            label: "Usage",
            current: 100,
            limit: 100,
            unit: .percent
        ),
        availability: .unavailable
    )
    let overLimit = UsageSnapshot(
        provider: .codex,
        primaryMetric: UsageMetric(
            label: "Usage",
            current: 140,
            limit: 100,
            unit: .percent
        )
    )

    #expect(MenuBarSummary(snapshots: [unavailable, overLimit]).usageFraction == 1)
    #expect(MenuBarSummary(snapshots: []).usageFraction == nil)
}
```

- [x] **步骤 2：运行定向测试验证失败**

运行：

```bash
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-quantum-core \
  bash scripts/test.sh --filter AppPresentationTests
```

预期：FAIL，编译器报告 `MenuBarSummary` 没有 `usageFraction`。

- [x] **步骤 3：实现最小展示模型改动**

在 `MenuBarSummary` 增加公开只读属性，并确保所有分支赋值：

```swift
public struct MenuBarSummary: Equatable, Sendable {
    public let semantic: UsageSemantic
    public let usageFraction: Double?
    public let valueText: String
    public let accessibilityLabel: String

    public init(snapshots: [UsageSnapshot]) {
        let presentations = snapshots.map(ProviderPresentation.init(snapshot:))
        let availablePresentations = zip(snapshots, presentations)
            .filter { snapshot, _ in snapshot.availability == .available }
            .map(\.1)
        if let highest = availablePresentations
            .compactMap(\.fraction)
            .filter({ $0.isFinite })
            .max() {
            let normalizedHighest = min(max(highest, 0), 1)
            usageFraction = normalizedHighest
            let percent = Int((normalizedHighest * 100).rounded())
            valueText = "\(percent)%"
            accessibilityLabel = "\(AppBrand.displayName), highest usage \(percent) percent"
            if normalizedHighest >= 0.90 {
                semantic = .critical
            } else if normalizedHighest >= 0.70 {
                semantic = .warning
            } else {
                semantic = .normal
            }
        } else {
            usageFraction = nil
            valueText = "—"
            accessibilityLabel = "\(AppBrand.displayName), usage unavailable"
            semantic = presentations.contains(where: { $0.semantic == .stale })
                ? .stale
                : .unavailable
        }
    }
}
```

- [x] **步骤 4：运行定向测试验证通过**

运行任务 1 步骤 2 的同一命令。

预期：`AppPresentationTests` 全部 PASS；现有最高风险、品牌文案和无数据测试不变。

- [x] **步骤 5：提交展示模型变更**

```bash
git add Sources/AIMeterCore/Presentation/AppPresentation.swift \
  Tests/AIMeterCoreTests/AppPresentationTests.swift
git commit -m "feat: expose menu bar usage fraction"
```

### 任务 2：实现并验证 18×18pt Quantum Dial

**文件：**
- 创建：`Sources/AIMeterApp/Views/MenuBarMeterIcon.swift`
- 创建：`Tests/AIMeterAppTests/MenuBarMeterIconTests.swift`

- [x] **步骤 1：编写失败的几何与渲染测试**

创建测试套件，先锁定几何常量和代表状态：

```swift
import AppKit
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("Menu bar Quantum Dial")
struct MenuBarMeterIconTests {
    @Test("Geometry maps usage to the approved 270 degree sweep")
    func mapsUsageToGeometry() {
        let zero = MenuBarMeterGeometry(fraction: 0)
        let twentyThree = MenuBarMeterGeometry(fraction: 0.23)
        let seventy = MenuBarMeterGeometry(fraction: 0.70)
        let ninety = MenuBarMeterGeometry(fraction: 0.90)
        let full = MenuBarMeterGeometry(fraction: 1)

        #expect(zero.progressTrim == 0)
        #expect(abs(twentyThree.progressTrim! - 0.1725) < 0.0001)
        #expect(abs(twentyThree.pointerDegrees - 197.1) < 0.0001)
        #expect(abs(seventy.progressTrim! - 0.525) < 0.0001)
        #expect(abs(ninety.progressTrim! - 0.675) < 0.0001)
        #expect(full.progressTrim == 0.75)
        #expect(full.pointerDegrees == 405)
    }

    @Test("Geometry clamps invalid bounds and gives unavailable a neutral pointer")
    func normalizesGeometry() {
        #expect(MenuBarMeterGeometry(fraction: -1).progressTrim == 0)
        #expect(MenuBarMeterGeometry(fraction: 2).progressTrim == 0.75)
        #expect(MenuBarMeterGeometry(fraction: nil).progressTrim == nil)
        #expect(MenuBarMeterGeometry(fraction: nil).pointerDegrees == 0)
        #expect(MenuBarMeterGeometry(fraction: .nan).progressTrim == nil)
    }
}
```

再增加 `@MainActor` 渲染测试，用 `ImageRenderer` 渲染 `.frame(width: 18, height: 18)`，断言：

```swift
let renderer = ImageRenderer(content:
    MenuBarMeterIcon(fraction: 0.23)
        .foregroundStyle(.white)
        .frame(width: 18, height: 18)
)
renderer.scale = 2
let image = try #require(renderer.cgImage)
#expect(image.width == 36)
#expect(image.height == 36)
```

通过 `NSBitmapImageRep` 检查四角 alpha 为零、中心轴和轨道采样点可见；分别渲染 `.white` 和 `.black` 前景，确认不依赖固定颜色或背景底板。

- [x] **步骤 2：运行图标测试验证失败**

运行：

```bash
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-quantum-icon \
  bash scripts/test.sh --filter MenuBarMeterIconTests
```

预期：FAIL，编译器报告 `MenuBarMeterGeometry` 和 `MenuBarMeterIcon` 尚不存在。

- [x] **步骤 3：实现独立几何模型**

在新文件中定义：

```swift
import SwiftUI

struct MenuBarMeterGeometry: Equatable {
    static let startDegrees = 135.0
    static let sweepDegrees = 270.0
    static let trackTrim = sweepDegrees / 360.0

    let normalizedFraction: Double?

    init(fraction: Double?) {
        guard let fraction, fraction.isFinite else {
            normalizedFraction = nil
            return
        }
        normalizedFraction = min(max(fraction, 0), 1)
    }

    var progressTrim: Double? {
        normalizedFraction.map { $0 * Self.trackTrim }
    }

    var pointerDegrees: Double {
        guard let normalizedFraction else { return 0 }
        return Self.startDegrees + normalizedFraction * Self.sweepDegrees
    }
}
```

- [x] **步骤 4：实现 Quantum Dial SwiftUI 视图**

在同一文件新增 `MenuBarMeterIcon`：

```swift
struct MenuBarMeterIcon: View {
    let fraction: Double?

    private let tickFractions = [0.0, 0.5, 1.0]

    var body: some View {
        let geometry = MenuBarMeterGeometry(fraction: fraction)
        ZStack {
            Circle()
                .trim(from: 0, to: MenuBarMeterGeometry.trackTrim)
                .stroke(
                    .primary.opacity(0.30),
                    style: StrokeStyle(lineWidth: 1.45, lineCap: .round)
                )
                .rotationEffect(.degrees(MenuBarMeterGeometry.startDegrees))

            if let progressTrim = geometry.progressTrim, progressTrim > 0 {
                Circle()
                    .trim(from: 0, to: progressTrim)
                    .stroke(
                        .primary,
                        style: StrokeStyle(lineWidth: 1.55, lineCap: .round)
                    )
                    .rotationEffect(.degrees(MenuBarMeterGeometry.startDegrees))
            }

            dialTicks

            Capsule()
                .fill(.primary)
                .frame(width: 5.7, height: 1.35)
                .offset(x: 2.55)
                .rotationEffect(.degrees(geometry.pointerDegrees))

            Circle()
                .fill(.primary)
                .frame(width: 2.5, height: 2.5)
        }
        .padding(1.4)
        .frame(width: 18, height: 18)
        .accessibilityHidden(true)
    }

    private var dialTicks: some View {
        ForEach(tickFractions, id: \.self) { tickFraction in
            Capsule()
                .fill(.primary)
                .frame(width: 1.15, height: 2.3)
                .offset(y: -6.1)
                .rotationEffect(.degrees(
                    MenuBarMeterGeometry.startDegrees
                        + tickFraction * MenuBarMeterGeometry.sweepDegrees
                        + 90
                ))
        }
    }
}
```

三枚短 `Capsule` 固定在起点、中点和终点；所有尺寸保持在本文件内。不要添加渐变、背景、阴影、定时器或动画。

- [x] **步骤 5：运行定向测试并按像素证据微调**

运行任务 2 步骤 2 的同一命令。

预期：全部 PASS；若采样点显示笔画裁切，只调整本文件中的 padding、线宽或刻度长度，不改变 135° 起点、270° 扫掠和 18×18pt 外框。

- [x] **步骤 6：提交 Quantum Dial 组件**

```bash
git add Sources/AIMeterApp/Views/MenuBarMeterIcon.swift \
  Tests/AIMeterAppTests/MenuBarMeterIconTests.swift
git commit -m "feat: add dynamic Quantum Dial menu icon"
```

### 任务 3：接入菜单栏、完成文档与发布验收

**文件：**
- 修改：`Sources/AIMeterApp/Views/MenuBarPanel.swift:5-18`
- 修改：`Tests/AIMeterAppTests/TypographyTests.swift:325-365`
- 修改：`README.md`
- 修改：`CHANGELOG.md`
- 修改：`docs/user-guide/getting-started.md`
- 创建：`docs/development/2026-09-02-menu-bar-quantum-dial.md`
- 修改：`docs/development/README.md`
- 修改：`docs/README.md`
- 修改：`docs/development/commit-history.md`
- 修改：`docs/requirements-backlog.md`

- [ ] **步骤 1：替换菜单栏顶部图标**

把 `MenuBarLabel` 的图标分支改为：

```swift
} icon: {
    MenuBarMeterIcon(fraction: model.menuBarSummary.usageFraction)
}
```

保留整个 `Label`、`Text(model.menuBarSummary.valueText)`、`.accessibilityLabel(...)` 和 `.aiMeterFontScope(...)`。不要修改 `MenuBarPanel` 中 `ContentUnavailableView` 的 SF Symbol。

- [ ] **步骤 2：移除已经失效的 SF Symbol 源码探测断言**

`TypographyTests.symbolFontMappings()` 当前对 `MenuBarPanel.swift` 中两次旧 SF Symbol 的源码片段和字体修饰符做字符串探测。顶部图标改成自绘组件后，这两条断言不再描述字体行为；同时按照测试规范，源码搜索也不能证明真实渲染。删除这两项 `SymbolSourceExpectation`，不要为新组件添加替代字符串断言。Quantum Dial 的真实行为由任务 1 的数据测试、任务 2 的几何/渲染测试、编译检查和本任务的真实菜单栏验收共同保护。

- [ ] **步骤 3：运行组件与排版测试验证接入**

运行：

```bash
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-quantum-wiring \
  bash scripts/test.sh --filter MenuBarMeterIconTests
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-quantum-typography \
  bash scripts/test.sh --filter TypographyTests
```

预期：两个套件全部 PASS；菜单栏字体作用域与弹出面板内部图标契约保持正常。

- [ ] **步骤 4：提交菜单栏接入**

```bash
git add Sources/AIMeterApp/Views/MenuBarPanel.swift \
  Tests/AIMeterAppTests/TypographyTests.swift
git commit -m "feat: use Quantum Dial in the menu bar"
```

- [ ] **步骤 5：更新用户与维护文档**

文档必须明确：

- Quantum Dial 的弧长和指针显示最高有效已用比例；
- 旁边百分比仍是精确值；
- 无数据时显示中性图标和 `—`；
- 图标为单色、无动画，并自动适配菜单栏；
- 不影响应用 Icon、Provider Logo 和额度算法。

开发日志记录 Red/Green 测试输出、完整测试计数、Release 路径、签名、候选与安装哈希、真实菜单栏验收结果和提交 ID。需求 `REQ-20260902-010` 仅在全部验收完成后改为 `已完成`。

- [ ] **步骤 6：运行完整自动化与静态检查**

```bash
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-quantum-full bash scripts/test.sh
git diff --check
```

预期：现有 295 项测试加新增测试全部 PASS，0 失败；`git diff --check` 无输出。

- [ ] **步骤 7：构建 Release 并验证候选应用**

```bash
AI_METER_INCLUDE_WIDGET=0 bash scripts/build-app.sh
codesign --verify --deep --strict --verbose=2 "dist/AI Token Meter.app"
file "dist/AI Token Meter.app/Contents/MacOS/AIMeterApp"
shasum -a 256 "dist/AI Token Meter.app/Contents/MacOS/AIMeterApp"
```

预期：构建成功、严格签名验证通过、可执行文件为 Apple Silicon Mach-O，并记录候选哈希。

- [ ] **步骤 8：安装并执行真实菜单栏验收**

先退出当前 AI Token Meter，把 `/Applications/AI Token Meter.app` 移到带时间戳的 `/private/tmp` 可恢复备份，再安装 `dist/AI Token Meter.app`。验证：

1. 菜单栏显示 Quantum Dial 与精确百分比，没有背景方块、裁切或模糊线条；
2. 图标点击区域正常，点击后仍打开原菜单面板；
3. 手动刷新后图标和百分比同步变化；
4. 深色或浅色菜单栏下前景对比正常；
5. 候选与安装版 `AIMeterApp` SHA-256 完全一致；
6. 安装版 `codesign --verify --deep --strict` 通过。

如果无法安全构造无数据或指定百分比状态，自动化几何/渲染测试作为这些状态的证据，真实验收不得伪造数据。

- [ ] **步骤 9：提交文档与验收证据**

```bash
git add README.md CHANGELOG.md docs
git commit -m "docs: record Quantum Dial menu icon delivery"
```

- [ ] **步骤 10：独立审查、修复问题并合并**

对功能分支执行代码审查，重点检查：

- 文字和图标是否使用同一个最高比例；
- 比例为空、非有限、越界时是否安全；
- 18×18pt 绘图是否裁切；
- 菜单栏辅助功能是否重复；
- 是否误改弹出面板、Provider Logo 或额度算法。

修复所有 Critical、Important 和确认成立的 Minor 问题，再重新运行完整测试。最终快进合并到 `main`，在合并后的主分支再次运行完整测试，并清理本功能工作树与分支。
