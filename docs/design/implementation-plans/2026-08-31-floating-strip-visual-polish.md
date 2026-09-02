# 浮岛圆滑接边、无阴影与服务品牌色实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 消除贴边浮岛肩部的硬转角和突兀黑影，并让 Claude、Codex、DeepSeek 在浮岛、菜单卡片和详情页中使用统一且可辨的服务配色。

**架构：** `FloatingStripShape` 继续作为可见轮廓与拖动命中区域的唯一几何来源，以新增收直缓冲曲线实现二阶观感更平滑的肩部；`FloatingStripSurface` 只保留玻璃填充。服务颜色集中为 `UsageProvider` 的主题映射，所有正常状态通过同一解析器取得品牌渐变，异常状态则由现有 `UsageSemantic` 状态色覆盖。

**技术栈：** Swift 6、SwiftUI、AppKit、Swift Testing、Swift Package Manager、macOS 14+

---

## 文件结构

- 修改 `Sources/AIMeterApp/Views/FloatingStripShape.swift`：更新 S 曲线肩部并移除外投影。
- 创建 `Sources/AIMeterApp/Views/ProviderAccentPalette.swift`：保存可测试的品牌色 token、服务映射、正常/异常状态解析与 SwiftUI ShapeStyle。
- 修改 `Sources/AIMeterApp/Views/AIMeterVisualTheme.swift`：让通用进度条接收服务身份并使用统一颜色解析器。
- 修改 `Sources/AIMeterApp/Views/UsageRing.swift`：让正常圆环使用服务渐变、异常圆环使用语义状态色。
- 修改 `Sources/AIMeterApp/Views/ProviderCard.swift`：同步菜单卡片标题与关键数值颜色。
- 修改 `Sources/AIMeterApp/Views/FloatingStripView.swift`：同步 Claude 紧凑详情标题、关键数值与进度条。
- 修改 `Sources/AIMeterApp/Views/CodexDetailView.swift`：同步 Codex 标题、额度、进度条与本机统计强调色。
- 修改 `Sources/AIMeterApp/Views/CodexResetCreditsView.swift`：把剩余充值券及卡片强调色从全局薄荷色切换为 Codex 品牌色。
- 修改 `Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift`：同步 DeepSeek 标题、余额、统计卡片、图表与登录框强调色。
- 修改 `Tests/AIMeterAppTests/VisualSystemTests.swift`：覆盖新轮廓、无阴影、品牌色精确映射、服务差异和语义覆盖。
- 必要时修改 `Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift`：只更新受新轮廓影响的真实采样点，不放宽 Logo 排除契约。
- 创建 `docs/development/2026-08-31-floating-strip-visual-polish.md`：记录根因、设计决策、TDD 证据、测试、构建、安装和人工验收。
- 修改 `CHANGELOG.md`：在未发布版本中记录浮岛和品牌色变化。
- 修改 `docs/design/specifications/2026-08-31-floating-strip-corner-shadow-design.md`：验收后把状态更新为已实施。

### 任务 1：圆滑收直肩部与无阴影表面

**文件：**
- 修改：`Tests/AIMeterAppTests/VisualSystemTests.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripShape.swift`
- 必要时修改：`Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift`

- [x] **步骤 1：编写失败的轮廓与无阴影测试**

把 `floatingStripSCurveShoulders()` 的肩部断言补充为新收直区域的内外点，并新增透明像素测试：

```swift
let settlingOutside = [
    CGPoint(x: 0, y: 94),
    CGPoint(x: 0, y: 100),
]
for point in settlingOutside {
    #expect(!right.contains(point))
    #expect(!right.contains(CGPoint(x: point.x, y: rect.maxY - point.y)))
    #expect(!left.contains(CGPoint(x: rect.maxX - point.x, y: point.y)))
}

#expect(right.contains(CGPoint(x: 8, y: 100)))
#expect(right.contains(CGPoint(x: 1, y: 110)))
```

新增：

```swift
@Test("Floating island has no shadow outside its visible shoulder")
@MainActor
func floatingSurfaceHasNoExteriorShadow() throws {
    let renderer = ImageRenderer(content:
        FloatingStripSurface(edge: .right)
            .frame(width: 108, height: 356)
    )
    renderer.scale = 1
    let image = try #require(renderer.cgImage)

    #expect(try alpha(atX: 0, y: 94, in: image) == 0)
}
```

- [x] **步骤 2：运行专项测试并验证红灯**

运行：

```bash
bash scripts/test.sh --filter VisualSystemTests
```

预期：旧路径在 `y = 94/100` 已进入竖直主体，且旧实现会让 `(0, 94)` 产生 alpha，因此新增断言失败。`(0, 100)` 只用于几何测试，避免把曲线边缘的正常抗锯齿误判成阴影。

- [x] **步骤 3：实现三段肩部并移除外投影**

把顶部第二段替换为两段曲线：

```swift
path.addCurve(
    to: point(16, 82),
    control1: point(54, 60),
    control2: point(30, 68)
)
path.addCurve(
    to: point(0, 104),
    control1: point(8, 89),
    control2: point(0, 96)
)
path.addLine(to: point(0, 252))
path.addCurve(
    to: point(16, 274),
    control1: point(0, 260),
    control2: point(8, 267)
)
path.addCurve(
    to: point(78, 304),
    control1: point(30, 288),
    control2: point(54, 296)
)
```

保留尖点两端曲线，并让 `FloatingStripSurface` 只绘制：

```swift
FloatingStripShape(edge: edge)
    .fill(AIMeterVisualTheme.floatingGlass)
```

- [x] **步骤 4：运行轮廓与拖动专项测试**

运行：

```bash
bash scripts/test.sh --filter VisualSystemTests
bash scripts/test.sh --filter FloatingStripDragShapeTests
bash scripts/test.sh --filter FloatingStripPointerDragStateTests
```

预期：新轮廓、透明外肩、左右镜像、Logo 排除和玻璃拖动全部通过。若旧拖动夹具落在新的透明肩部，只把该采样点移动到新路径内，禁止改变三个 Logo 的排除框尺寸。

- [x] **步骤 5：提交几何检查点**

```bash
git add Sources/AIMeterApp/Views/FloatingStripShape.swift Tests/AIMeterAppTests/VisualSystemTests.swift Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift
git commit -m "fix: smooth floating strip shoulders"
```

### 任务 2：集中式服务品牌色与语义覆盖

**文件：**
- 创建：`Sources/AIMeterApp/Views/ProviderAccentPalette.swift`
- 修改：`Tests/AIMeterAppTests/VisualSystemTests.swift`

- [x] **步骤 1：编写失败的品牌色映射测试**

在 `VisualSystemTests` 中新增：

```swift
@Test("Each provider owns the approved unique brand palette")
func providerBrandPalettes() {
    #expect(UsageProvider.claude.accentPalette == .init(startHex: 0xE8B96D, endHex: 0xD97757))
    #expect(UsageProvider.codex.accentPalette == .init(startHex: 0xFF6FAE, endHex: 0xA96DFF))
    #expect(UsageProvider.deepSeek.accentPalette == .init(startHex: 0x54EDC6, endHex: 0x7769FF))
    #expect(Set(UsageProvider.allCases.map(\.accentPalette)).count == 3)
}

@Test("Semantic states override provider identity colors")
func semanticAccentPrecedence() {
    #expect(UsageSemantic.normal.accentRole(for: .codex) == .provider(.codex))
    for semantic in [UsageSemantic.warning, .critical, .stale, .unavailable] {
        for provider in UsageProvider.allCases {
            #expect(semantic.accentRole(for: provider) == .semantic(semantic))
        }
    }
}
```

- [x] **步骤 2：运行测试并验证编译红灯**

运行：

```bash
bash scripts/test.sh --filter VisualSystemTests/providerBrandPalettes
```

预期：编译失败，报告 `accentPalette`、`AIMeterProviderPalette` 或 `accentRole` 尚不存在。

- [x] **步骤 3：实现可测试的主题 token 和统一解析器**

创建 `ProviderAccentPalette.swift`：

```swift
import AIMeterCore
import SwiftUI

struct AIMeterProviderPalette: Hashable {
    let startHex: UInt32
    let endHex: UInt32

    var startColor: Color { Color(hex: startHex) }
    var endColor: Color { Color(hex: endHex) }
    var gradient: LinearGradient {
        LinearGradient(
            colors: [startColor, endColor],
            startPoint: .topLeading,
            endPoint: .bottomTrailing
        )
    }
}

enum AIMeterAccentRole: Equatable {
    case provider(UsageProvider)
    case semantic(UsageSemantic)
}

extension UsageProvider {
    var accentPalette: AIMeterProviderPalette {
        switch self {
        case .claude: .init(startHex: 0xE8B96D, endHex: 0xD97757)
        case .codex: .init(startHex: 0xFF6FAE, endHex: 0xA96DFF)
        case .deepSeek: .init(startHex: 0x54EDC6, endHex: 0x7769FF)
        }
    }
}

extension UsageSemantic {
    func accentRole(for provider: UsageProvider) -> AIMeterAccentRole {
        self == .normal ? .provider(provider) : .semantic(self)
    }

    func accentStyle(for provider: UsageProvider) -> AnyShapeStyle {
        switch accentRole(for: provider) {
        case .provider(let provider): AnyShapeStyle(provider.accentPalette.gradient)
        case .semantic(let semantic): AnyShapeStyle(semantic.color)
        }
    }
}

private extension Color {
    init(hex: UInt32) {
        self.init(
            red: Double((hex >> 16) & 0xFF) / 255,
            green: Double((hex >> 8) & 0xFF) / 255,
            blue: Double(hex & 0xFF) / 255
        )
    }
}
```

- [x] **步骤 4：运行映射和语义专项测试**

运行：

```bash
bash scripts/test.sh --filter VisualSystemTests/providerBrandPalettes
bash scripts/test.sh --filter VisualSystemTests/semanticAccentPrecedence
```

预期：精确色值、三个唯一配色以及四种异常状态优先级全部通过。

- [x] **步骤 5：提交主题基础检查点**

```bash
git add Sources/AIMeterApp/Views/ProviderAccentPalette.swift Tests/AIMeterAppTests/VisualSystemTests.swift
git commit -m "feat: add provider accent palettes"
```

### 任务 3：把品牌色同步到浮岛、菜单与详情

**文件：**
- 修改：`Sources/AIMeterApp/Views/AIMeterVisualTheme.swift`
- 修改：`Sources/AIMeterApp/Views/UsageRing.swift`
- 修改：`Sources/AIMeterApp/Views/ProviderCard.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift`
- 修改：`Sources/AIMeterApp/Views/CodexDetailView.swift`
- 修改：`Sources/AIMeterApp/Views/CodexResetCreditsView.swift`
- 修改：`Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift`
- 修改：`Tests/AIMeterAppTests/VisualSystemTests.swift`

- [x] **步骤 1：编写失败的进度条渲染差异与语义覆盖测试**

新增一个返回进度条中心像素 RGB 的测试辅助方法，并新增：

```swift
@Test("Normal progress bars use provider colors while warnings stay semantic")
@MainActor
func providerProgressBarColors() throws {
    let normalColors = try UsageProvider.allCases.map {
        try progressBarPixel(provider: $0, semantic: .normal)
    }
    #expect(Set(normalColors).count == 3)

    let warningColors = try UsageProvider.allCases.map {
        try progressBarPixel(provider: $0, semantic: .warning)
    }
    #expect(Set(warningColors).count == 1)
}
```

`progressBarPixel` 使用 `ImageRenderer` 绘制 `AIMeterProgressBar(provider:fraction:semantic:)` 的 `120 × 5` 视图，并读取 `(60, 2)` 的 RGB，不包含 alpha。

- [x] **步骤 2：运行测试并验证编译红灯**

运行：

```bash
bash scripts/test.sh --filter VisualSystemTests/providerProgressBarColors
```

预期：编译失败，因为 `AIMeterProgressBar` 还没有 `provider` 参数。

- [x] **步骤 3：让圆环和通用进度条使用统一解析器**

把进度条签名改为：

```swift
struct AIMeterProgressBar: View {
    let provider: UsageProvider
    let fraction: Double
    let semantic: UsageSemantic

    private var fillStyle: AnyShapeStyle {
        semantic.accentStyle(for: provider)
    }
}
```

把 `UsageRing.ringStyle` 改为：

```swift
private var ringStyle: AnyShapeStyle {
    presentation.semantic.accentStyle(for: presentation.provider)
}
```

- [x] **步骤 4：同步菜单卡片和 Claude 紧凑详情**

在 `ProviderCard` 中让标题使用 `snapshot.provider.accentPalette.gradient`，让主值使用 `presentation.semantic.accentStyle(for: snapshot.provider)`。在 `FloatingDetailView.compactDetail` 中同样处理标题与主值，并给 `AIMeterProgressBar` 传入 `snapshot.provider`；次级百分比也使用相同解析后的样式。

- [x] **步骤 5：同步 Codex 详情和充值券卡片**

在 `CodexDetailView` 中：

- `Codex` 标题使用 `.codex.accentPalette.gradient`；
- 主百分比、两个额度卡百分比、三个本机统计图标和数值使用 `presentation.semantic.accentStyle(for: .codex)`；
- 两个 `AIMeterProgressBar` 传入 `.codex`。

在 `CodexResetCreditsView` 中用 `.codex.accentPalette.startColor/endColor` 替换正常状态的 `mintAccent/violetAccent`；`today`、`expired` 和 `unavailable` 仍保留橙、红、灰状态色。

- [x] **步骤 6：同步 DeepSeek 详情**

在 `DeepSeekAnalyticsView` 中让标题、余额、统计卡关键数值和柱状图使用 `presentation.semantic.accentStyle(for: .deepSeek)`；登录 WebView 描边使用 `.deepSeek.accentPalette.startColor.opacity(0.42)`。不得更改图表数据、登录会话或自动隐藏逻辑。

- [x] **步骤 7：运行品牌视觉专项测试**

运行：

```bash
bash scripts/test.sh --filter VisualSystemTests
```

预期：三个正常进度条颜色不同，三种 warning 进度条颜色相同；精确 palette、语义符号、几何、无阴影和 App Icon 测试全部通过。

- [x] **步骤 8：提交界面同步检查点**

```bash
git add Sources/AIMeterApp/Views/AIMeterVisualTheme.swift Sources/AIMeterApp/Views/UsageRing.swift Sources/AIMeterApp/Views/ProviderCard.swift Sources/AIMeterApp/Views/FloatingStripView.swift Sources/AIMeterApp/Views/CodexDetailView.swift Sources/AIMeterApp/Views/CodexResetCreditsView.swift Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift Tests/AIMeterAppTests/VisualSystemTests.swift
git commit -m "feat: sync provider colors across the interface"
```

### 任务 4：文档、完整验证、安装与主分支集成

**文件：**
- 创建：`docs/development/2026-08-31-floating-strip-visual-polish.md`
- 修改：`CHANGELOG.md`
- 修改：`docs/design/specifications/2026-08-31-floating-strip-corner-shadow-design.md`

- [x] **步骤 1：记录开发与用户可见变更**

开发记录必须包含：旧曲率突然归零与 34% 黑色偏移阴影的根因、方案 A 轮廓坐标、方案 C Codex 配色、三个 palette、红灯和绿灯测试命令、最终测试数量、Release 签名、备份目录、安装哈希和人工验收结果。不得写入 API Key、OAuth URL、账户原始响应或短信内容。

在 `CHANGELOG.md` 未发布部分添加：

```markdown
- Refined the edge island with a longer tangent transition and removed the abrupt exterior shadow.
- Added distinct provider accents across rings, menu cards, detail progress, titles, and key values while preserving semantic warning colors.
```

把规格状态改为 `已实施并验收`。

- [x] **步骤 2：运行完整自动化测试**

运行：

```bash
bash scripts/test.sh
```

预期：基线 148 项加本次新增测试全部通过，0 个失败；环境门控的真实 CLI smoke 测试可保持 skip。

- [x] **步骤 3：构建并验证 Release App**

运行：

```bash
bash scripts/build-app.sh
codesign --verify --deep --strict "dist/AI Meter.app"
plutil -lint "dist/AI Meter.app/Contents/Info.plist"
```

预期：`dist/AI Meter.app` 构建成功、签名有效、Info.plist 有效。

- [x] **步骤 4：提交文档与发布候选检查点**

```bash
git add CHANGELOG.md docs/development/2026-08-31-floating-strip-visual-polish.md docs/design/specifications/2026-08-31-floating-strip-corner-shadow-design.md docs/design/implementation-plans/2026-08-31-floating-strip-visual-polish.md
git commit -m "docs: record floating strip visual polish"
```

- [x] **步骤 5：安全替换已安装 App**

退出当前 AI Meter，把 `/Applications/AI Meter.app` 移动到带时间戳的 `/private/tmp/AI Meter.app.pre-visual-polish-*` 目录，再把 `dist/AI Meter.app` 复制到 `/Applications`。验证新安装可执行文件与 `dist` 可执行文件的 SHA-256 完全相同，然后启动安装版。

- [x] **步骤 6：完成真实 UI 验收**

在浅色桌面背景上逐项确认：

1. 右贴边上、下肩部圆滑进入中央主体，无黑色外影；
2. 左贴边是严格镜像且仍与屏幕零间距；
3. Claude 黄橙、Codex 玫红紫、DeepSeek 薄荷紫在三个圆环中明显区分；
4. 依次打开三个详情，标题、关键数据和进度条与各自圆环一致；
5. 异常状态仍使用语义状态色和非颜色状态符号；
6. 上部、圆环间和下部可拖动，Logo 点击只打开详情；
7. 菜单栏 Settings 可打开设置，Automatic/Left/Right 仍正常。

- [x] **步骤 7：合并回 `main` 并做合并后验证**

确认功能分支干净后，在主工作区快进或无冲突合并 `codex/visual-polish`。随后在 `main` 运行 `bash scripts/test.sh`，确认测试数量与功能分支一致，再记录最终合并提交和安装哈希。
