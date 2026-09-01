# AI Token Meter Settings 字体隔离与内容字号提升实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 让 Settings 永久使用 macOS 系统默认字体，同时让浮动条、三个详情页和菜单栏点击面板的全部产品文字精确增加 `1pt`。

**架构：** 把现有单一显示字体环境扩展为明确的界面作用域配置：Settings 固定 `.system + 0pt`，内容界面使用用户所选字体 `+ 1pt`，菜单栏紧凑标签保留用户所选字体但不增加字号。字体解析器集中处理语义字号、固定字号、自定义字体、系统回退与安全下限；根视图负责安装作用域，子视图继续使用统一 `aiMeterFont`，不感知 Settings 特例。

**技术栈：** Swift 6、SwiftUI、AppKit、Swift Testing、SwiftPM、macOS Release App Bundle、ad-hoc codesign。

---

## 文件结构

- 修改：`Sources/AIMeterApp/Views/AIMeterTypography.swift` — 定义字体作用域配置、字号偏移环境、安全字号解析和新的根作用域 modifier。
- 修改：`Tests/AIMeterAppTests/TypographyTests.swift` — 覆盖 Settings 隔离、内容 `+1pt`、系统/自定义/回退/固定字号、安全下限和根视图接线契约。
- 修改：`Sources/AIMeterApp/Views/SettingsView.swift` — 固定 Settings 系统字体作用域，移除字体选项字形预览。
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift` — 为浮动条和浮动详情安装内容字号作用域。
- 修改：`Sources/AIMeterApp/Views/MenuBarPanel.swift` — 为菜单点击面板安装内容字号作用域；菜单栏紧凑标签保持原字号。
- 修改：`README.md` — 说明字体选择的作用范围和 Settings 固定字体。
- 修改：`CHANGELOG.md` — 记录 Settings 字体隔离与内容字号提升。
- 修改：`docs/user-guide/settings.md` — 更新字体选项、无预览和 Settings 不随选择变化的说明。
- 修改：`docs/superpowers/specs/2026-08-31-display-font-selection-design.md` — 标记被新规格覆盖的 Settings 条款。
- 修改：`docs/superpowers/specs/2026-09-01-settings-font-isolation-and-content-size-step-design.md` — 回填实施与验收状态。
- 创建：`docs/development/2026-09-01-settings-font-isolation-and-content-size-step.md` — 记录 TDD、构建、安装和实机验收证据。
- 修改：`docs/development/README.md` — 索引本次开发日志。

### 任务 1：建立可测试的字体作用域和精确字号偏移

**文件：**
- 修改：`Sources/AIMeterApp/Views/AIMeterTypography.swift`
- 修改：`Tests/AIMeterAppTests/TypographyTests.swift`

- [x] **步骤 1：为作用域配置和字号解析编写失败测试**

在 `TypographyTests` 增加：

```swift
@Test("Font surfaces isolate settings and apply one point only to content")
func fontSurfaceConfigurations() {
    #expect(AIMeterFontScopeConfiguration.settings.choice == .system)
    #expect(AIMeterFontScopeConfiguration.settings.pointOffset == 0)

    let content = AIMeterFontScopeConfiguration.content(.antonio)
    #expect(content.choice == .antonio)
    #expect(content.pointOffset == 1)

    let menuBarLabel = AIMeterFontScopeConfiguration.menuBarLabel(.dinCondensed)
    #expect(menuBarLabel.choice == .dinCondensed)
    #expect(menuBarLabel.pointOffset == 0)
}

@Test("Content offset adds exactly one point to system and custom semantic fonts")
func contentSemanticOffset() {
    let catalog = DisplayFontCatalog(availableFamilies: ["Antonio"])

    let system = AIMeterTypography.resolvedDescriptor(
        token: .semantic(.headline),
        choice: .system,
        catalog: catalog,
        pointOffset: 1,
        design: .default,
        weight: nil
    )
    guard case let .systemFixed(size, _, weight) = system else {
        Issue.record("Offset system headline did not resolve to an exact point size")
        return
    }
    #expect(size == 14)
    #expect(weight == .semibold)

    let custom = AIMeterTypography.resolvedDescriptor(
        token: .semantic(.body),
        choice: .antonio,
        catalog: catalog,
        pointOffset: 1,
        design: .default,
        weight: nil
    )
    guard case let .custom(family, size, relativeTo, _) = custom else {
        Issue.record("Offset Antonio body did not remain a custom font")
        return
    }
    #expect(family == "Antonio")
    #expect(size == 14)
    #expect(relativeTo == .body)
}

@Test("Fixed fonts and unavailable custom fallback preserve the content offset")
func fixedAndFallbackOffset() {
    let unavailable = DisplayFontCatalog(availableFamilies: [])
    let fallback = AIMeterTypography.resolvedDescriptor(
        token: .semantic(.caption),
        choice: .antonio,
        catalog: unavailable,
        pointOffset: 1,
        design: .default,
        weight: nil
    )
    guard case let .systemFixed(fallbackSize, _, _) = fallback else {
        Issue.record("Unavailable custom content font did not retain exact offset")
        return
    }
    #expect(fallbackSize == 11)

    let fixed = AIMeterTypography.resolvedDescriptor(
        token: .fixed(size: 15, relativeTo: .body),
        choice: .system,
        catalog: unavailable,
        pointOffset: 1,
        design: .default,
        weight: .semibold
    )
    guard case let .systemFixed(fixedSize, _, _) = fixed else {
        Issue.record("Fixed system font did not resolve with offset")
        return
    }
    #expect(fixedSize == 16)
}

@Test("Point offsets cannot produce a zero or negative font size")
func pointSizeLowerBound() {
    #expect(
        AIMeterTypography.resolvedPointSize(
            token: .fixed(size: 2, relativeTo: .caption2),
            pointOffset: -10
        ) == 1
    )
}
```

- [x] **步骤 2：运行字体测试确认红灯**

运行：

```bash
swift test --filter TypographyTests
```

预期：编译失败，指出 `AIMeterFontScopeConfiguration`、`pointOffset` 参数和 `resolvedPointSize` 尚不存在。

- [x] **步骤 3：实现字体作用域配置和偏移解析**

在 `AIMeterTypography.swift` 增加：

```swift
struct AIMeterFontScopeConfiguration: Equatable, Sendable {
    let choice: DisplayFontChoice
    let pointOffset: CGFloat

    static let settings = Self(choice: .system, pointOffset: 0)

    static func content(_ choice: DisplayFontChoice) -> Self {
        Self(choice: choice, pointOffset: 1)
    }

    static func menuBarLabel(_ choice: DisplayFontChoice) -> Self {
        Self(choice: choice, pointOffset: 0)
    }
}
```

给 `AIMeterTypography` 增加安全字号方法：

```swift
static func resolvedPointSize(
    token: AIMeterFontToken,
    pointOffset: CGFloat
) -> CGFloat {
    max(1, token.pointSize + pointOffset)
}
```

把 `resolvedDescriptor` 签名改为：

```swift
static func resolvedDescriptor(
    token: AIMeterFontToken,
    choice: DisplayFontChoice,
    catalog: DisplayFontCatalog,
    pointOffset: CGFloat = 0,
    design: Font.Design,
    weight: Font.Weight?
) -> AIMeterResolvedFontDescriptor
```

解析规则必须是：

```swift
let style = token.relativeStyle
let size = resolvedPointSize(token: token, pointOffset: pointOffset)

if let family = resolvedFamily(for: choice, catalog: catalog) {
    return .custom(
        family: family,
        size: size,
        relativeTo: style,
        weight: weight ?? style.customDefaultWeight
    )
}

return switch token {
case .semantic where pointOffset == 0:
    .systemSemantic(style: style, design: design, weight: weight)
case .semantic:
    .systemFixed(
        size: size,
        design: design,
        weight: weight ?? style.customDefaultWeight
    )
case .fixed:
    .systemFixed(size: size, design: design, weight: weight)
}
```

- [x] **步骤 4：把字号偏移接入 SwiftUI 环境和根作用域**

在显示字体环境键旁新增：

```swift
private struct AIMeterFontPointOffsetKey: EnvironmentKey {
    static let defaultValue: CGFloat = 0
}

extension EnvironmentValues {
    var aiMeterFontPointOffset: CGFloat {
        get { self[AIMeterFontPointOffsetKey.self] }
        set { self[AIMeterFontPointOffsetKey.self] = newValue }
    }
}
```

`AIMeterFontModifier` 读取 `aiMeterFontPointOffset` 并把它传给 `resolvedDescriptor`。把 `AIMeterFontScopeModifier` 的输入从单独 `choice` 改为 `AIMeterFontScopeConfiguration`，同时注入两个环境值，并使用同一解析器计算根级 `.body` 默认字体。

公开的根 modifier 改为：

```swift
func aiMeterFontScope(_ configuration: AIMeterFontScopeConfiguration) -> some View {
    modifier(AIMeterFontScopeModifier(configuration: configuration))
}
```

删除 `AIMeterFontPreviewModifier` 和 `aiMeterFontPreview(_:)`，避免 Settings 以后再次绕过隔离作用域。

- [x] **步骤 5：运行字体测试确认绿灯**

运行：

```bash
swift test --filter TypographyTests
```

预期：原字体测试与新增作用域、偏移、回退和下限测试全部通过。

- [x] **步骤 6：提交字体基础设施**

```bash
git add Sources/AIMeterApp/Views/AIMeterTypography.swift \
  Tests/AIMeterAppTests/TypographyTests.swift
git commit -m "feat: isolate typography by interface surface"
```

### 任务 2：接入 Settings、浮动窗口和菜单面板

**文件：**
- 修改：`Sources/AIMeterApp/Views/SettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift`
- 修改：`Sources/AIMeterApp/Views/MenuBarPanel.swift`
- 修改：`Tests/AIMeterAppTests/TypographyTests.swift`

- [x] **步骤 1：编写根视图接线失败测试**

在 `TypographyTests` 增加源码契约测试和读取助手：

```swift
@Test("Settings is system-only while content roots apply the content scale")
func rootScopeWiring() throws {
    let settings = try viewSource("SettingsView.swift")
    #expect(settings.contains(".aiMeterFontScope(.settings)"))
    #expect(!settings.contains("aiMeterFontPreview"))

    let floating = try viewSource("FloatingStripView.swift")
    #expect(
        floating.components(
            separatedBy: ".aiMeterFontScope(.content(model.displayFontChoice))"
        ).count - 1 == 2
    )

    let menu = try viewSource("MenuBarPanel.swift")
    #expect(menu.contains(".aiMeterFontScope(.content(model.displayFontChoice))"))
    #expect(menu.contains(".aiMeterFontScope(.menuBarLabel(model.displayFontChoice))"))
}

private func viewSource(_ fileName: String) throws -> String {
    let projectRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let fileURL = projectRoot
        .appending(path: "Sources/AIMeterApp/Views")
        .appending(path: fileName)
    return try String(contentsOf: fileURL, encoding: .utf8)
}
```

- [x] **步骤 2：运行接线测试确认红灯**

运行：

```bash
swift test --filter TypographyTests.rootScopeWiring
```

预期：测试失败，因为三个根视图仍使用旧 `aiMeterFontScope(model.displayFontChoice)`，Settings 仍包含 `aiMeterFontPreview`。

- [x] **步骤 3：隔离 Settings 并移除字体选项预览**

在 `SettingsView` 的字体 Picker 中删除：

```swift
.aiMeterFontPreview(option.choice)
```

把 Settings 根作用域改为：

```swift
.aiMeterFontScope(.settings)
```

保留选项名称、`Not installed`、禁用、选中、保存和 Restore Default Font 行为。不要删除 Picker 对 `model.displayFontChoice` 的读写，因为它仍控制其他内容界面。

- [x] **步骤 4：为浮动条与详情接入内容作用域**

在 `FloatingStripView.body` 和 `FloatingDetailView.body` 的根部把旧调用都替换为：

```swift
.aiMeterFontScope(.content(model.displayFontChoice))
```

这样 `ProviderCard`、`CodexDetailView`、`CodexResetCreditsView`、`DeepSeekAnalyticsView` 和 Claude 紧凑详情中的全部 `aiMeterFont` 会自动获得 `+1pt`，无需逐个增加字号。

保留以下直接 `.font(...)`，因为它们只设置 SF Symbol 图标而不是产品文字：

- `UsageRing` 的无颜色辅助状态图标；
- `CodexDetailView` 的本机统计图标；
- `CodexResetCreditsView` 的重置箭头图标；
- `DeepSeekAnalyticsView` 的不可用图表图标。

- [x] **步骤 5：为菜单点击面板接入内容作用域**

把 `MenuBarPanel` 根部改为：

```swift
.aiMeterFontScope(.content(model.displayFontChoice))
```

菜单栏中的紧凑 `MenuBarLabel` 不属于点击后面板，保持原字号并继续跟随所选字体：

```swift
.aiMeterFontScope(.menuBarLabel(model.displayFontChoice))
```

- [x] **步骤 6：运行接线与完整 App 视觉系统测试**

运行：

```bash
swift test --filter TypographyTests
swift test --filter VisualSystemTests
```

预期：根作用域契约、字体解析和既有背景、轮廓、图标、Provider 配色测试全部通过。

- [x] **步骤 7：提交界面接线**

```bash
git add Sources/AIMeterApp/Views/SettingsView.swift \
  Sources/AIMeterApp/Views/FloatingStripView.swift \
  Sources/AIMeterApp/Views/MenuBarPanel.swift \
  Tests/AIMeterAppTests/TypographyTests.swift
git commit -m "feat: enlarge content fonts outside settings"
```

### 任务 3：文档、全量回归、构建安装与实机验收

**文件：**
- 修改：`README.md`
- 修改：`CHANGELOG.md`
- 修改：`docs/user-guide/settings.md`
- 修改：`docs/superpowers/specs/2026-08-31-display-font-selection-design.md`
- 修改：`docs/superpowers/specs/2026-09-01-settings-font-isolation-and-content-size-step-design.md`
- 创建：`docs/development/2026-09-01-settings-font-isolation-and-content-size-step.md`
- 修改：`docs/development/README.md`
- 修改：`docs/superpowers/plans/2026-09-01-settings-font-isolation-and-content-size-step.md`

- [x] **步骤 1：更新用户文档和历史规格**

在 `README.md` 的字体功能说明中明确：

```markdown
- 浮动条、详情和菜单点击面板的显示字体可在 System Default、Antonio 和 DIN Condensed 之间即时切换；Settings 永远使用 macOS 系统字体，不随选择或内容字号变化。
```

在 `CHANGELOG.md` 的 `Changed` 中增加：

```markdown
- Settings 现在固定使用 macOS 系统字体，字体选项只显示名称；浮动条、三个详情页和菜单点击面板的产品文字统一增大 1pt。
```

更新 `docs/user-guide/settings.md` 的 Display font：删除“切换应用到当前 Settings”和字体名称预览含义，明确设置页固定系统字体。给旧字体规格增加“Settings 相关条款已由 2026-09-01 新规格覆盖”的说明，不改写历史实现记录。

- [x] **步骤 2：创建开发日志并记录 TDD 证据**

创建 `docs/development/2026-09-01-settings-font-isolation-and-content-size-step.md`，记录：

- 规格和计划路径；
- 任务 1、任务 2 的红灯命令与失败原因；
- 对应绿灯命令和通过结果；
- 完整测试套件数量与失败数；
- Release 构建、签名、候选包与安装包 SHA-256；
- Settings 三种选择不改变字体的观察结果；
- 浮动条、Claude、Codex、DeepSeek 详情和菜单面板的字号与截断检查；
- 未覆盖的硬件或人工验收项必须明确标记，不能写成已通过。

在 `docs/development/README.md` 加入该日志链接。

- [x] **步骤 3：运行全量自动化回归**

运行：

```bash
bash scripts/test.sh
```

预期：全部测试套件通过，0 失败；依赖本机 Keychain 或已安装 CLI 的环境门控测试可以跳过，但跳过原因必须写入开发日志。

- [x] **步骤 4：构建并验证 Release App Bundle**

运行：

```bash
bash scripts/build-app.sh
codesign --verify --deep --strict "dist/AI Meter.app"
shasum -a 256 "dist/AI Meter.app/Contents/MacOS/AIMeterApp"
```

预期：Release 构建完成，严格签名验证无输出且退出码为 0，并取得候选可执行文件 SHA-256。

- [x] **步骤 5：安装候选版并核对安装指纹**

先完全退出正在运行的 AI Meter，把当前 `/Applications/AI Meter.app` 备份到 `/private/tmp/AI-Meter-font-scope-backup-20260901/AI Meter.app`，再用 `ditto` 安装 `dist/AI Meter.app`。随后运行：

```bash
codesign --verify --deep --strict "/Applications/AI Meter.app"
shasum -a 256 "/Applications/AI Meter.app/Contents/MacOS/AIMeterApp"
```

预期：安装包签名通过，安装包可执行文件 SHA-256 与步骤 4 候选值完全一致。涉及关闭、备份、覆盖和启动 GUI App 时按系统要求请求授权，不删除备份。

- [ ] **步骤 6：执行 Settings 隔离实机验收**

启动安装候选版并依次检查：

1. 当前 Antonio 选择下打开 Settings，全部文字为 macOS 系统字体；
2. 依次选择 System Default、Antonio、DIN Condensed，Settings 字形与字号不变化；
3. 三个字体选项只显示系统字体名称，不展示对应字形；
4. `Not installed`、禁用、选择和 Restore Default Font 仍正常；
5. 最终恢复用户原有选择 Antonio；
6. 重启 App 后 Antonio 保持，Settings 仍为系统字体。

保存必要截图、辅助功能树和最终偏好状态到开发日志；视觉观察不能被单元测试替代。

- [ ] **步骤 7：执行内容字号与布局实机验收**

分别打开菜单点击面板、浮动条、Claude 详情、Codex 详情和 DeepSeek 详情，检查：

1. 所有产品文字相对当前基线增大 `1pt`；
2. Logo、圆环、SF Symbol、深海背景和浮动轮廓尺寸不变；
3. 百分比、金额、重置时间、充值券到期日、30 天统计和错误说明无截断；
4. 详情自适应高度、点击、拖动、自动隐藏和外部点击关闭不回归；
5. Settings 字号不随内容增大。

把直接观察结果和任何环境未覆盖项写入开发日志。

- [x] **步骤 8：回填规格、计划和最终验证结果**

将新规格状态更新为真实结果；在本计划完成的步骤改为 `[x]`；开发日志记录提交号、测试数、候选与安装指纹。运行：

```bash
git diff --check
git status --short
```

预期：文档无格式错误，状态只包含本任务预期文档变更。

- [x] **步骤 9：提交文档与验收记录**

```bash
git add README.md CHANGELOG.md docs/user-guide/settings.md \
  docs/superpowers/specs/2026-08-31-display-font-selection-design.md \
  docs/superpowers/specs/2026-09-01-settings-font-isolation-and-content-size-step-design.md \
  docs/superpowers/plans/2026-09-01-settings-font-isolation-and-content-size-step.md \
  docs/development/2026-09-01-settings-font-isolation-and-content-size-step.md \
  docs/development/README.md
git commit -m "docs: record font scope acceptance"
```

- [x] **步骤 10：执行提交后最终检查**

运行：

```bash
bash scripts/test.sh
git diff --check HEAD^ HEAD
git status --short --branch
```

预期：全量测试 0 失败、最终提交格式检查通过、工作树干净并停留在 `codex/desktop-only-floating-strip`。未经用户明确批准，不合并 `main`、不推送远端。
