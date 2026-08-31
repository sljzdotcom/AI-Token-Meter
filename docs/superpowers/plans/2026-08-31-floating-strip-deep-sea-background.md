# 浮动条「深海波纹」背景实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 在现有浮动条内部加入方案 C 的静态黑蓝深海波纹图片，左右贴边时仅背景同步镜像，并完整保留短肩轮廓、Logo、进度环和全部交互。

**架构：** 新增独立的背景资源加载与合成组件，继续以 `FloatingStripShape` 作为唯一可见裁切边界。现有玻璃渐变始终作为底层回退，背景 PNG 和固定深色遮罩位于其上，Provider 圆环仍由 `FloatingStripView` 在最前层绘制，因此图片镜像不会影响 Logo 或进度方向。

**技术栈：** Swift 6、SwiftUI、AppKit `NSImage`、Swift Package Manager 资源、Swift Testing、ImageRenderer、macOS App Bundle 构建与 ad-hoc 签名。

---

## 文件结构

- 创建：`Sources/AIMeterApp/Resources/Backgrounds/floating-strip-deep-sea.png` — 只包含深海波纹纹理的 3× 静态背景资源。
- 创建：`Sources/AIMeterApp/Views/FloatingStripBackground.swift` — 资源定位、加载、镜像策略、遮罩参数和背景合成。
- 修改：`Package.swift` — 将 `Resources/Backgrounds` 明确复制进 App Bundle。
- 修改：`Sources/AIMeterApp/Views/FloatingStripShape.swift` — 保留纯几何 `Shape`，将 `FloatingStripSurface` 移到背景组件文件。
- 修改：`Tests/AIMeterAppTests/VisualSystemTests.swift` — 覆盖资源、回退、裁切、镜像与品牌颜色保护。
- 修改：`README.md` — 补充浮动条深海波纹背景说明。
- 修改：`CHANGELOG.md` — 记录视觉改进。
- 创建：`docs/development/2026-08-31-floating-strip-deep-sea-background.md` — 记录实现、测试、构建、安装与实机验收。
- 修改：`docs/superpowers/specs/2026-08-31-floating-strip-deep-sea-background-design.md` — 实施后更新状态和验收摘要。

### 任务 1：生成并可靠打包深海波纹资源

**文件：**
- 创建：`Sources/AIMeterApp/Resources/Backgrounds/floating-strip-deep-sea.png`
- 修改：`Package.swift`
- 创建：`Sources/AIMeterApp/Views/FloatingStripBackground.swift`
- 测试：`Tests/AIMeterAppTests/VisualSystemTests.swift`

- [ ] **步骤 1：先编写资源缺失时失败的测试**

在 `VisualSystemTests` 中引入 AppKit，并添加：

```swift
@Test("Deep sea background is bundled at retina resolution")
func floatingBackgroundResource() throws {
    let url = try #require(FloatingStripBackgroundAsset.resourceURL())
    let image = try #require(NSImage(contentsOf: url))

    #expect(image.size.width >= 324)
    #expect(image.size.height >= 1068)
}
```

- [ ] **步骤 2：运行专项测试并确认红灯**

运行：

```bash
swift test --filter VisualSystemTests.floatingBackgroundResource
```

预期：编译失败，提示 `FloatingStripBackgroundAsset` 不存在；不得通过跳过或删除断言绕开失败。

- [ ] **步骤 3：加入最小资源定位器和 SwiftPM 资源声明**

在 `Package.swift` 的 `AIMeterApp` resources 中同时复制 Logo 和背景目录：

```swift
resources: [
    .copy("Resources/Logos"),
    .copy("Resources/Backgrounds"),
]
```

创建 `FloatingStripBackground.swift`，先只实现资源定位：

```swift
import AppKit
import SwiftUI

enum FloatingStripBackgroundAsset {
    static let filename = "floating-strip-deep-sea"

    static func resourceURL(in bundle: Bundle = .module) -> URL? {
        bundle.url(
            forResource: filename,
            withExtension: "png",
            subdirectory: "Backgrounds"
        )
    }

    static func load(in bundle: Bundle = .module) -> NSImage? {
        resourceURL(in: bundle).flatMap(NSImage.init(contentsOf:))
    }
}
```

- [ ] **步骤 4：生成不含控件的最终背景资源**

使用已确认的 C 方案作为视觉依据，通过内置图像生成工具制作纯背景：

```text
Use case: stylized-concept
Asset type: exact-aspect macOS floating widget background texture
Primary request: a static abstract deep-sea wave background made of two or three oversized flowing curved bands; near-black and ink navy base with restrained luminous azure edges
Composition: portrait 108:356 aspect ratio; broad sparse curves; keep three low-contrast calm zones centered near y=86, y=178, and y=270 for circular controls
Constraints: background texture only; no widget silhouette, no logos, no rings, no text, no particles, no bubbles, no orange, no outer shadow
```

将选定输出复制到项目资源目录，并机械裁切/缩放为至少 `324 × 1068` 像素，保持 `108:356` 纵横比。不得覆盖生成工具中的原始输出；项目只保留最终选定版本。

- [ ] **步骤 5：检查资源尺寸、色彩和内容**

运行：

```bash
sips -g pixelWidth -g pixelHeight -g format \
  "Sources/AIMeterApp/Resources/Backgrounds/floating-strip-deep-sea.png"
```

预期：PNG，宽度至少 324、高度至少 1068；肉眼检查图片只含黑蓝大幅波纹，没有 Logo、圆环、文字和细碎装饰。

- [ ] **步骤 6：运行资源测试确认绿灯**

运行：

```bash
swift test --filter VisualSystemTests.floatingBackgroundResource
```

预期：测试通过，SwiftPM 测试资源包能找到并解码背景图片。

- [ ] **步骤 7：提交资源交付物**

```bash
git add Package.swift \
  Sources/AIMeterApp/Resources/Backgrounds/floating-strip-deep-sea.png \
  Sources/AIMeterApp/Views/FloatingStripBackground.swift \
  Tests/AIMeterAppTests/VisualSystemTests.swift
git commit -m "feat: add deep sea floating strip background asset"
```

### 任务 2：合成、裁切、镜像并保护可读性

**文件：**
- 修改：`Sources/AIMeterApp/Views/FloatingStripBackground.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripShape.swift`
- 测试：`Tests/AIMeterAppTests/VisualSystemTests.swift`

- [ ] **步骤 1：编写镜像、回退和外部透明度测试**

在 `VisualSystemTests` 中增加：

```swift
@Test("Only the background mirrors with the attached edge")
func floatingBackgroundOrientation() {
    #expect(FloatingStripBackgroundPresentation.horizontalScale(for: .right) == 1)
    #expect(FloatingStripBackgroundPresentation.horizontalScale(for: .left) == -1)
}

@Test("Missing background keeps the glass fallback and exact shoulder mask")
@MainActor
func floatingBackgroundFallback() throws {
    let renderer = ImageRenderer(content:
        FloatingStripSurface(edge: .right, backgroundImage: nil)
            .frame(width: 108, height: 356)
    )
    renderer.scale = 1
    let image = try #require(renderer.cgImage)

    #expect(try alpha(atX: 0, y: 178, in: image) > 0)
    #expect(try alpha(atX: 107, y: 8, in: image) == 0)
    #expect(try alpha(atX: 107, y: 348, in: image) == 0)
}
```

将现有 `floatingSurfacePaintsAttachedEdgeAndBody` 和 `floatingSurfaceHasNoExteriorShadow` 保留为使用真实资源的集成渲染检查。

- [ ] **步骤 2：运行专项测试并确认红灯**

运行：

```bash
swift test --filter VisualSystemTests
```

预期：新增测试因缺少 `FloatingStripBackgroundPresentation` 和可注入的 `backgroundImage` 初始化器而失败；既有短肩测试保持通过。

- [ ] **步骤 3：实现最小镜像策略与背景合成**

在 `FloatingStripBackground.swift` 中增加：

```swift
enum FloatingStripBackgroundPresentation {
    static let scrimOpacity = 0.38

    static func horizontalScale(for edge: FloatingStripEdge) -> CGFloat {
        edge == .left ? -1 : 1
    }
}

struct FloatingStripSurface: View {
    let edge: FloatingStripEdge
    private let backgroundImage: NSImage?

    init(
        edge: FloatingStripEdge,
        backgroundImage: NSImage? = FloatingStripBackgroundAsset.load()
    ) {
        self.edge = edge
        self.backgroundImage = backgroundImage
    }

    var body: some View {
        FloatingStripShape(edge: edge)
            .fill(AIMeterVisualTheme.floatingGlass)
            .overlay {
                if let backgroundImage {
                    Image(nsImage: backgroundImage)
                        .resizable()
                        .scaledToFill()
                        .scaleEffect(
                            x: FloatingStripBackgroundPresentation
                                .horizontalScale(for: edge),
                            y: 1
                        )
                        .overlay(
                            Color.black.opacity(
                                FloatingStripBackgroundPresentation.scrimOpacity
                            )
                        )
                        .clipShape(FloatingStripShape(edge: edge))
                        .accessibilityHidden(true)
                }
            }
    }
}
```

从 `FloatingStripShape.swift` 删除旧的 `FloatingStripSurface`，该文件只保留几何路径。不得将镜像应用到 `FloatingStripView` 的外层 `ZStack`，否则会错误镜像 Logo 和圆环。

- [ ] **步骤 4：运行视觉专项测试确认绿灯**

运行：

```bash
swift test --filter VisualSystemTests
swift test --filter FloatingStripDragShapeTests
swift test --filter FloatingStripPointerDragStateTests
```

预期：资源、回退、透明肩部、左右策略、短肩轮廓、品牌配色和拖动相关测试全部通过。

- [ ] **步骤 5：渲染左右静态快照并人工检查**

在测试或临时预览中分别渲染 `.right` 和 `.left` 的 `108 × 356` 表面，检查：

- 背景方向严格水平镜像；
- 外部肩部保持透明；
- 图片未拉伸到改变波纹比例；
- 三个圆环区域的背景亮度不会削弱 Logo 和进度色；
- 无黑色外投影、亮边溢出或贴边接缝。

若可读性不足，只允许微调 `scrimOpacity`，每次改动后重新运行 `VisualSystemTests`；不得改 Provider 颜色补偿背景。

- [ ] **步骤 6：提交合成与镜像交付物**

```bash
git add Sources/AIMeterApp/Views/FloatingStripBackground.swift \
  Sources/AIMeterApp/Views/FloatingStripShape.swift \
  Tests/AIMeterAppTests/VisualSystemTests.swift
git commit -m "feat: render mirrored deep sea strip background"
```

### 任务 3：完整验证、安装验收和项目文档

**文件：**
- 修改：`README.md`
- 修改：`CHANGELOG.md`
- 创建：`docs/development/2026-08-31-floating-strip-deep-sea-background.md`
- 修改：`docs/superpowers/specs/2026-08-31-floating-strip-deep-sea-background-design.md`

- [ ] **步骤 1：运行完整自动化测试**

运行：

```bash
bash scripts/test.sh
```

预期：全部非环境门控测试通过，0 失败；记录测试数、测试组数和按设计跳过的真实 CLI/钥匙串集成检查数。

- [ ] **步骤 2：构建并验证发布包**

运行：

```bash
bash scripts/build-app.sh
codesign --verify --deep --strict "dist/AI Meter.app"
plutil -lint "dist/AI Meter.app/Contents/Info.plist"
shasum -a 256 "dist/AI Meter.app/Contents/MacOS/AIMeterApp"
```

预期：构建退出码 0，签名无错误，Info.plist 为 `OK`；记录发布包可执行文件 SHA-256。

- [ ] **步骤 3：安全安装候选版并启动**

完整退出当前 AI Meter，将 `/Applications/AI Meter.app` 移到带时间戳的 `/private/tmp` 备份目录，再使用 `ditto` 安装候选包并启动。安装后再次核对签名、Info.plist，以及候选包与安装包的 SHA-256 完全相同。

- [ ] **步骤 4：完成真实 UI 验收**

使用 macOS Computer Use 与截图依次验证：

1. 右贴边显示黑蓝深海波纹，短肩和透明区域正确；
2. 切换左贴边后波纹同步镜像，三枚 Logo 与进度环不镜像；
3. Claude、Codex、DeepSeek 的进度色与当前值保持原样；
4. 点击三个 Logo 可打开对应详情，详情可按现有规则自动隐藏；
5. 点击桌面空白处可关闭详情；
6. 浮动条玻璃空白区域可拖动，Provider 圆环仍只响应点击；
7. AI Meter 菜单中的 Settings 可正常打开；
8. 浅色与深色桌面壁纸上均无背景溢出、黑色阴影或贴边接缝；
9. 验收后恢复用户原有贴边方向和垂直位置。

- [ ] **步骤 5：补全文档和变更日志**

文档必须记录：

- C 方案最终颜色、构图和静态资源路径；
- 合成层级、镜像边界、遮罩值和安全回退；
- 自动化测试结果；
- Release 构建、签名、Info.plist、SHA-256 与备份位置；
- 左右贴边、Logo、详情、拖动、Settings 和配色实机验收结果。

将设计规格状态更新为“已实施并验收”，README 只补充用户可见功能，CHANGELOG 使用一条清晰的 Added/Changed 记录。

- [ ] **步骤 6：提交文档检查点**

```bash
git add README.md CHANGELOG.md \
  docs/development/2026-08-31-floating-strip-deep-sea-background.md \
  docs/superpowers/specs/2026-08-31-floating-strip-deep-sea-background-design.md \
  docs/superpowers/plans/2026-08-31-floating-strip-deep-sea-background.md
git commit -m "docs: record deep sea strip background acceptance"
```

- [ ] **步骤 7：完成前最终验证**

运行：

```bash
bash scripts/test.sh
git diff --check
git status --short --branch
```

预期：完整测试仍为 0 失败，`git diff --check` 无输出，工作区除计划中明确需要提交的内容外保持干净。随后按 `finishing-a-development-branch` 流程选择合并方式；未经用户选择不得擅自推送远端或删除未合并分支。
