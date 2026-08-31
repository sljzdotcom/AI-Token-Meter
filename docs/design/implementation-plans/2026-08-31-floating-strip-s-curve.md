# AI Meter 贴边浮岛 S 曲线实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 把浮岛上下的分段平台肩部替换为与用户手绘图一致、上下对称且左右镜像的连续 S 曲线。

**架构：** 保留 `108 × 356` 基准画布和现有水平镜像入口，只替换 `FloatingStripShape` 的右侧基准路径。每个肩部由两段三次贝塞尔组成，顶部定义一次、底部使用严格镜像坐标；拖动 Shape 继续由可见路径与 Logo 排除区相交得到，无需修改输入控制器。

**技术栈：** Swift 6、SwiftUI `Shape` / `Path`、Swift Testing、AppKit Release App Bundle。

---

## 文件结构

- 修改 `Sources/AIMeterApp/Views/FloatingStripShape.swift`：定义新的双贝塞尔 S 曲线基准路径。
- 修改 `Tests/AIMeterAppTests/VisualSystemTests.swift`：把旧“反向半圆”断言替换为尖点、S 曲线、上下对称和左右镜像契约。
- 修改 `Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift`：把旧肩部内的拖动夹具迁移到新 S 曲线内，继续保证 Logo 排除和透明肩部规则。
- 修改 `Tests/AIMeterAppTests/FloatingStripPointerDragStateTests.swift`：把指针起点迁移到新 S 曲线内，同时保持坐标转换与位移契约。
- 修改 `CHANGELOG.md`：记录用户可见轮廓修正。
- 修改 `docs/design/specifications/2026-08-31-floating-strip-s-curve-design.md`：把状态更新为已实施并链接验证日志。
- 创建 `docs/development/2026-08-31-floating-strip-s-curve.md`：记录红绿测试、构建、安装、真实截图和指针验收证据。
- 修改 `docs/development/commit-history.md`：加入规格、实现、验收和合并节点。
- 更新 `docs/assets/ai-meter-floating-strip.jpeg`：使用最终安装版的真实浮岛截图。

### 任务 1：用测试驱动替换 S 曲线轮廓

**文件：**
- 修改：`Tests/AIMeterAppTests/VisualSystemTests.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripShape.swift`
- 修改：`Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift`
- 修改：`Tests/AIMeterAppTests/FloatingStripPointerDragStateTests.swift`

- [x] **步骤 1：把旧肩部测试改成新的尖点和 S 曲线契约**

在 `VisualSystemTests` 中把 `floatingStripReverseSemicircleShoulders()` 替换为：

```swift
@Test("Floating island tapers from edge points through symmetric S curves")
func floatingStripSCurveShoulders() {
    let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
    let right = FloatingStripShape(edge: .right).path(in: rect)
    let left = FloatingStripShape(edge: .left).path(in: rect)

    let insideTop = [
        CGPoint(x: 107, y: 20),
        CGPoint(x: 100, y: 40),
        CGPoint(x: 75, y: 58),
    ]
    let outsideTop = [
        CGPoint(x: 90, y: 1),
        CGPoint(x: 90, y: 20),
        CGPoint(x: 70, y: 40),
        CGPoint(x: 40, y: 58),
    ]

    for point in insideTop {
        #expect(right.contains(point))
        #expect(right.contains(CGPoint(x: point.x, y: rect.maxY - point.y)))
        #expect(left.contains(CGPoint(x: rect.maxX - point.x, y: point.y)))
    }
    for point in outsideTop {
        #expect(!right.contains(point))
        #expect(!right.contains(CGPoint(x: point.x, y: rect.maxY - point.y)))
        #expect(!left.contains(CGPoint(x: rect.maxX - point.x, y: point.y)))
    }

    #expect(right.contains(CGPoint(x: 106, y: 178)))
    #expect(left.contains(CGPoint(x: 2, y: 178)))
}
```

同时把 `floatingSurfaceIsOpaqueAtWindowEdges()` 改名为 `floatingSurfacePaintsAttachedEdgeAndBody()`，只检查中央主体和贴边竖线：

```swift
for point in [(0, 178), (107, 1), (107, 178), (107, 354)] {
    #expect(try alpha(atX: point.0, y: point.1, in: image) > 0)
}
```

- [x] **步骤 2：运行专项测试并验证红灯**

运行：

```bash
bash scripts/test.sh --filter VisualSystemTests
```

预期：`Floating island tapers from edge points through symmetric S curves` 失败。旧路径仍包含 `(90, 1)`、`(90, 20)` 或 `(70, 40)` 等平台区域，证明测试能抓住当前错误轮廓；测试必须是断言失败而不是编译错误。

- [x] **步骤 3：用最少路径修改实现双贝塞尔 S 曲线**

保留 `point(_:_:)` 的缩放和左右镜像逻辑，把 `var path = Path()` 之后的路径替换为：

```swift
var path = Path()
path.move(to: point(108, 0))
path.addCurve(
    to: point(78, 52),
    control1: point(108, 28),
    control2: point(98, 45)
)
path.addCurve(
    to: point(0, 88),
    control1: point(53, 60),
    control2: point(0, 63)
)
path.addLine(to: point(0, 268))
path.addCurve(
    to: point(78, 304),
    control1: point(0, 293),
    control2: point(53, 296)
)
path.addCurve(
    to: point(108, 356),
    control1: point(98, 311),
    control2: point(108, 328)
)
path.closeSubpath()
```

这些控制点使两个尖点和主体连接点都具有近似竖直切线；`(78, 52)` / `(78, 304)` 是曲率过渡节点。不得保留原来的顶部水平线、`x = 58` 竖直短线、`y = 68` 水平短线或它们的底部镜像。

- [x] **步骤 4：运行视觉与拖动专项测试并验证绿灯**

运行：

```bash
bash scripts/test.sh --filter VisualSystemTests
bash scripts/test.sh --filter FloatingStripDragShapeTests
bash scripts/test.sh --filter FloatingStripPointerDragStateTests
```

预期：三个命令均通过；S 曲线几何、Logo 排除、透明肩部排除和真实指针坐标转换没有回归。

如果旧夹具使用的顶部点 `(86, 40)` 已落在新曲线外，则把右侧基准点改为经过手工验证、位于新曲线内的 `(75, 58)`：左侧镜像为 `(33, 58)`，两倍缩放且平移到 `(100, 200)` 的点为 `(250, 316)`，AppKit 底部原点窗口坐标为 `(75, 298)`。这些期望值必须以字面量写入对应测试，生产 Shape 不暴露测试专用控制点。

- [x] **步骤 5：运行完整测试**

运行：

```bash
bash scripts/test.sh
git diff --check
```

预期：完整套件至少 `148` 项测试、`33` 个测试组、`0` 失败；只有未显式授权的 Keychain/真实 CLI 检查按设计跳过；差异检查无输出。

- [x] **步骤 6：保存实现节点**

```bash
git add Sources/AIMeterApp/Views/FloatingStripShape.swift Tests/AIMeterAppTests/VisualSystemTests.swift Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift Tests/AIMeterAppTests/FloatingStripPointerDragStateTests.swift docs/design/implementation-plans/2026-08-31-floating-strip-s-curve.md
git commit -m "fix: reshape floating strip with S curves"
```

### 任务 2：完成文档、Release 与真实安装验收

**文件：**
- 修改：`CHANGELOG.md`
- 修改：`docs/design/specifications/2026-08-31-floating-strip-s-curve-design.md`
- 创建：`docs/development/2026-08-31-floating-strip-s-curve.md`
- 修改：`docs/development/commit-history.md`
- 更新：`docs/assets/ai-meter-floating-strip.jpeg`

- [x] **步骤 1：构建并验证 Release App Bundle**

运行：

```bash
bash scripts/build-app.sh
plutil -lint "dist/AI Meter.app/Contents/Info.plist"
codesign --verify --deep --strict --verbose=2 "dist/AI Meter.app"
file "dist/AI Meter.app/Contents/MacOS/AIMeterApp"
test -s "dist/AI Meter.app/Contents/Resources/AppIcon.icns"
```

预期：构建成功，Info.plist 合法，签名满足 Designated Requirement，可执行文件为 arm64，App Icon 存在。

- [x] **步骤 2：安全替换安装版并保留可恢复备份**

先退出正在运行的 `AIMeterApp`。确认 `/private/tmp/AI Meter.app.pre-s-curve-20260831-final` 不存在后，把 `/Applications/AI Meter.app` 移到这个明确路径，再把本次 `dist/AI Meter.app` 安装到 `/Applications/AI Meter.app`。安装后重新执行严格签名验证，并比较构建产物与安装版可执行文件的 SHA-256；两者必须一致。最后重新启动安装版。

- [x] **步骤 3：使用真实安装版完成视觉和交互验收**

使用 Computer Use 读取 `/Applications/AI Meter.app`，保存最终截图，并核对：

1. 右侧顶部和底部均从屏幕边缘尖点通过连续 S 曲线接入主体；
2. 没有水平平台、阶梯、边框或接缝；
3. 切换到左侧后轮廓严格镜像，再恢复用户原侧边偏好；
4. 三个 Logo 位置和视觉重量不变；
5. 从顶部、圆环间、底部玻璃拖动均改变垂直位置；
6. 点击任一 Logo 只打开详情，不改变浮岛位置；
7. 菜单栏 Settings 仍能打开既有设置窗口。

把安装版浮岛截图更新为 `docs/assets/ai-meter-floating-strip.jpeg`，图片只包含应用浮岛，不包含账户私密详情。

- [x] **步骤 4：更新当前文档与验证日志**

在 `CHANGELOG.md` 的 Unreleased / Fixed 中说明平台肩部已改为尖点 S 曲线；把规格状态更新为“已实施并验收”；创建开发日志，写明：

- 红灯失败的具体断言；
- 绿灯专项与完整测试数量；
- Release 签名结果；
- 安装备份位置和 SHA-256；
- 左右真实截图结果、三处拖动结果、Logo 点击和 Settings 结果；
- 实现提交和验收提交。

在 `docs/development/commit-history.md` 增加本阶段关键节点，不改写历史提交的含义。

- [x] **步骤 5：验证文档和最终差异**

运行：

```bash
git diff --check
bash scripts/test.sh
```

再检查所有 Markdown 相对链接存在，README 引用的截图能够读取。预期：至少 `148` 项测试、`33` 个测试组、`0` 失败，文档无断链。

- [x] **步骤 6：保存验收节点**

```bash
git add CHANGELOG.md docs/design/specifications/2026-08-31-floating-strip-s-curve-design.md docs/development/2026-08-31-floating-strip-s-curve.md docs/development/commit-history.md docs/assets/ai-meter-floating-strip.jpeg
git commit -m "docs: record S curve installation acceptance"
```

### 任务 3：审查并合入主分支

**文件：**
- 不新增生产文件；只核对任务 1–2 的提交和最终仓库状态。

- [x] **步骤 1：按规格逐项复核差异**

检查从规格提交到当前 HEAD 的差异，确认只涉及 Shape、对应测试、截图和文档；不得出现详情、采集器、额度算法、窗口尺寸或 Logo 资源修改。

- [x] **步骤 2：在功能分支运行最终验证**

```bash
bash scripts/test.sh
bash scripts/build-app.sh
codesign --verify --deep --strict --verbose=2 "dist/AI Meter.app"
git diff --check
git status --short --branch
```

预期：完整测试和构建通过，签名有效，工作区干净。

- [x] **步骤 3：按 finishing-a-development-branch 流程合并**

基础分支固定为 `main`。用户已经要求关键节点保存 Git；合并前仍要确认 `main` 没有用户未提交修改。使用非快进合并保留本阶段边界，在合并结果上重新运行完整测试和 Release 构建。通过后记录合并提交，清理本次 `.worktrees/floating-strip-s-curve` 与已合并功能分支；不得清理其他 worktree。
