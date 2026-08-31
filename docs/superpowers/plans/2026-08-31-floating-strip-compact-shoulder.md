# 浮岛紧凑短肩实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 将浮岛上下长 S 肩部替换为用户选定的紧凑短肩方案 A，同时保持尺寸、圆环、拖动、贴边和详情行为不变。

**架构：** 只修改 `FloatingStripShape` 的右侧基准路径；顶部用两段三次 Bézier 从贴边短平台进入主体，底部按 `y = 178` 严格镜像，左贴边继续由既有坐标解析器水平镜像。轮廓测试直接采样真实 `Path`，拖动测试继续通过真实 `FloatingStripDragShape` 和 `FloatingStripPointerDragState` 验证命中行为。

**技术栈：** Swift 6、SwiftUI `Shape`/`Path`、Swift Testing、SwiftPM、原生 macOS App Bundle、Git。

---

## 文件结构

- 修改：`Sources/AIMeterApp/Views/FloatingStripShape.swift` — 实现方案 A 的唯一基准轮廓。
- 修改：`Tests/AIMeterAppTests/VisualSystemTests.swift` — 锁定新边界、肩部高度、内外采样、上下对称和左右镜像。
- 修改：`Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift` — 证明新增可见短肩玻璃可拖动且透明区仍排除。
- 修改：`Tests/AIMeterAppTests/FloatingStripPointerDragStateTests.swift` — 从真实指针入口验证短肩命中。
- 修改：`CHANGELOG.md` — 记录用户可见的轮廓修正。
- 修改：`docs/design/specifications/2026-08-31-floating-strip-compact-shoulder-design.md` — 实施后更新状态和验证结果。
- 创建：`docs/development/2026-08-31-floating-strip-compact-shoulder.md` — 保存红绿证据、构建、安装、实机验收和 Git 检查点。

### 任务 1：用 TDD 实现紧凑短肩轮廓

**文件：**
- 修改：`Tests/AIMeterAppTests/VisualSystemTests.swift:15-77`
- 修改：`Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift:7-30`
- 修改：`Tests/AIMeterAppTests/FloatingStripPointerDragStateTests.swift:7-56`
- 修改：`Sources/AIMeterApp/Views/FloatingStripShape.swift:4-57`

- [x] **步骤 1：编写真实路径失败测试**

把 `floatingStripBounds` 改为断言方案 A 的真实可见边界，并把肩部测试改成以下独立推导的采样：

```swift
@Test("Both compact shoulders use the approved inset bounds")
func floatingStripBounds() {
    let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
    let expected = CGRect(x: 0, y: 16, width: 108, height: 324)

    #expect(FloatingStripShape(edge: .right).path(in: rect).boundingRect == expected)
    #expect(FloatingStripShape(edge: .left).path(in: rect).boundingRect == expected)
}

@Test("Compact shoulders match the approved short shelf and mirrored arc")
func floatingStripCompactShoulders() {
    let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
    let right = FloatingStripShape(edge: .right).path(in: rect)
    let left = FloatingStripShape(edge: .left).path(in: rect)

    for point in [CGPoint(x: 40, y: 50), CGPoint(x: 8, y: 82), CGPoint(x: 1, y: 94)] {
        #expect(right.contains(point))
        #expect(right.contains(CGPoint(x: point.x, y: rect.maxY - point.y)))
        #expect(left.contains(CGPoint(x: rect.maxX - point.x, y: point.y)))
    }
    for point in [CGPoint(x: 107, y: 8), CGPoint(x: 40, y: 30), CGPoint(x: 0, y: 82)] {
        #expect(!right.contains(point))
        #expect(!right.contains(CGPoint(x: point.x, y: rect.maxY - point.y)))
        #expect(!left.contains(CGPoint(x: rect.maxX - point.x, y: point.y)))
    }
}
```

测试抓住的破坏是：恢复旧 `(108, 0) → (0, 104)` 长肩、让上下不对称，或让左贴边不再镜像。期望坐标来自已确认规格，不调用被测路径计算。

- [x] **步骤 2：增加真实拖动入口失败测试**

在拖动 Shape 测试中增加短肩可见点：

```swift
#expect(right.contains(CGPoint(x: 40, y: 50), eoFill: true))
```

在指针状态测试中增加：

```swift
@Test("Visible compact shoulder begins a pointer drag")
func compactShoulderBeginsDrag() {
    var state = FloatingStripPointerDragState()

    let began = state.begin(
        windowPoint: CGPoint(x: 40, y: 50),
        screenPoint: CGPoint(x: 1900, y: 700),
        panelSize: CGSize(width: 108, height: 356),
        edge: .right
    )

    #expect(began)
    #expect(state.isActive)
}
```

测试抓住的破坏是：视觉 Shape 已改变但拖动仍使用旧轮廓，或者短肩的真实玻璃区域不能拖动。

- [x] **步骤 3：运行专项测试，确认旧路径正确失败**

运行：

```bash
bash scripts/test.sh --filter VisualSystemTests
bash scripts/test.sh --filter FloatingStripDragShapeTests
bash scripts/test.sh --filter FloatingStripPointerDragStateTests
```

预期：旧路径在新边界、`(40, 50)` 与 `(1, 94)` 内点、`(107, 8)` 外点及短肩拖动入口上失败；失败来自旧几何，不是编译错误。

- [x] **步骤 4：实现方案 A 的最小路径修改**

将 `FloatingStripShape.path(in:)` 的路径构造替换为：

```swift
var path = Path()
path.move(to: point(108, 16))
path.addCurve(
    to: point(66, 28),
    control1: point(98, 23),
    control2: point(88, 27)
)
path.addCurve(
    to: point(0, 88),
    control1: point(29, 29),
    control2: point(0, 54)
)
path.addLine(to: point(0, 268))
path.addCurve(
    to: point(66, 328),
    control1: point(0, 302),
    control2: point(29, 327)
)
path.addCurve(
    to: point(108, 340),
    control1: point(88, 329),
    control2: point(98, 333)
)
path.closeSubpath()
```

不要修改 `point(_:_:)` 镜像逻辑、窗口尺寸、圆环布局、表面填充或配色。

- [x] **步骤 5：运行专项测试，确认全部转绿**

运行：

```bash
bash scripts/test.sh --filter VisualSystemTests
bash scripts/test.sh --filter FloatingStripDragShapeTests
bash scripts/test.sh --filter FloatingStripPointerDragStateTests
```

预期：三个测试组全部通过，无意外跳过或警告导致失败。

- [x] **步骤 6：检查差异并保存功能检查点**

运行：

```bash
git diff --check
git diff -- Sources/AIMeterApp/Views/FloatingStripShape.swift Tests/AIMeterAppTests/VisualSystemTests.swift Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift Tests/AIMeterAppTests/FloatingStripPointerDragStateTests.swift
git add Sources/AIMeterApp/Views/FloatingStripShape.swift Tests/AIMeterAppTests/VisualSystemTests.swift Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift Tests/AIMeterAppTests/FloatingStripPointerDragStateTests.swift
git commit -m "fix: tighten floating strip shoulders"
```

预期：只包含方案 A 路径和相应回归测试。

### 任务 2：完整验证、文档、安装与实机验收

**文件：**
- 修改：`CHANGELOG.md`
- 修改：`docs/design/specifications/2026-08-31-floating-strip-compact-shoulder-design.md`
- 创建：`docs/development/2026-08-31-floating-strip-compact-shoulder.md`

- [x] **步骤 1：运行完整测试**

运行：

```bash
bash scripts/test.sh
```

预期：全部 Swift 测试通过；只允许既有真实钥匙串/已安装 CLI 门控测试按设计跳过。记录测试数、测试组数、失败数和跳过数。

- [x] **步骤 2：完成 Release 构建和静态验证**

运行：

```bash
bash scripts/build-app.sh
codesign --verify --deep --strict "dist/AI Meter.app"
plutil -lint "dist/AI Meter.app/Contents/Info.plist"
shasum -a 256 "dist/AI Meter.app/Contents/MacOS/AIMeterApp"
```

预期：构建退出码 0，签名验证无错误，Info.plist 为 `OK`，记录候选可执行文件 SHA-256。

- [x] **步骤 3：安全安装并启动候选版**

先确认 `/private/tmp/AI Meter.app.pre-compact-shoulder-20260831-1600` 不存在，再把现有 `/Applications/AI Meter.app` 移动到该路径；随后用 `ditto` 安装 `dist/AI Meter.app` 并启动。不得删除备份；如果该备份路径已经存在，停止安装并选用另一个写入计划和开发记录的明确路径。

- [x] **步骤 4：真实界面验收**

通过本机 UI 验收以下行为：

- 右贴边上、下肩部与已批准方案 A 一致，明显比旧版收窄；
- 左贴边是精确镜像，验收后恢复用户原来的边缘设置；
- 浮岛主体空白可以拖动，三个 Logo 点击只打开对应详情；
- Settings 菜单仍能打开；
- Claude、Codex、DeepSeek 颜色和余额/额度显示没有回归；
- 浅色或复杂壁纸上没有黑色外阴影和透明接缝。

- [x] **步骤 5：更新变更记录和开发证据**

在 `CHANGELOG.md` 的 `Unreleased / Fixed` 记录肩部外鼓修正。把规格状态改为“已实施并验收”。创建开发记录，写入：

- 根因和选定方案 A；
- 精确路径坐标；
- 红灯失败点和绿灯结果；
- 完整测试统计；
- Release、签名、Info.plist 和哈希；
- 安装备份路径；
- 左右贴边、拖动、Logo、Settings 和配色实机验收；
- Git 检查点。

- [x] **步骤 6：验证安装版与构建产物一致**

运行：

```bash
codesign --verify --deep --strict "/Applications/AI Meter.app"
plutil -lint "/Applications/AI Meter.app/Contents/Info.plist"
shasum -a 256 "dist/AI Meter.app/Contents/MacOS/AIMeterApp" "/Applications/AI Meter.app/Contents/MacOS/AIMeterApp"
git diff --check
```

预期：两个 SHA-256 完全相同，签名和 plist 验证通过，差异无空白错误。

- [x] **步骤 7：保存文档检查点并确认工作区**

运行：

```bash
git add CHANGELOG.md docs/design/specifications/2026-08-31-floating-strip-compact-shoulder-design.md docs/development/2026-08-31-floating-strip-compact-shoulder.md docs/superpowers/plans/2026-08-31-floating-strip-compact-shoulder.md
git commit -m "docs: record compact shoulder acceptance"
git status --short --branch
```

预期：计划复选框全部为 `[x]`，工作区干净，安装版正在运行，旧安装版备份仍可恢复。
