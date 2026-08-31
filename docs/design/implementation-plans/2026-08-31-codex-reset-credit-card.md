# Codex 重置券卡片实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 将 Codex 重置券区域改成已确认的分层卡片布局，明确显示数量、券类型、到期时间与剩余天数，并让详情面板按券数量自适应高度。

**架构：** 在 `AIMeterCore` 增加纯展示模型，集中处理排序和自然日状态，SwiftUI 视图只负责渲染。详情和菜单栏入口通过显示密度参数复用同一组件；独立的面板布局辅助类型根据券数量计算并约束高度。

**技术栈：** Swift 6、SwiftUI、AppKit、Swift Testing、Swift Package Manager。

---

## 文件结构

- 修改 `Sources/AIMeterCore/Presentation/AppPresentation.swift`：增加重置券排序、到期状态和展示文案的纯模型。
- 修改 `Tests/AIMeterCoreTests/AppPresentationTests.swift`：覆盖未来、当天、过期、缺失日期、稳定排序和不完整明细。
- 修改 `Sources/AIMeterApp/Views/CodexResetCreditsView.swift`：实现分层卡片与紧凑模式。
- 修改 `Sources/AIMeterApp/Views/CodexDetailView.swift`：明确使用详情卡片模式。
- 修改 `Sources/AIMeterApp/Views/ProviderCard.swift`：菜单栏继续使用紧凑模式，避免卡片撑大概览。
- 创建 `Sources/AIMeterApp/System/CodexDetailPanelLayout.swift`：纯函数计算 Codex 详情面板高度。
- 修改 `Sources/AIMeterApp/System/FloatingPanelController.swift`：按券数量和屏幕可用空间采用计算后的高度。
- 创建 `Tests/AIMeterAppTests/CodexDetailPanelLayoutTests.swift`：验证基础高度、增量和屏幕上限。
- 修改 `CHANGELOG.md`、`docs/user-guide/providers.md`、`docs/development/testing.md`：同步用户可见变化与测试基线。
- 创建 `docs/development/2026-08-31-codex-reset-credit-card.md`：保存视觉与安装验收记录。
- 修改 `docs/development/commit-history.md`：记录设计、实现、验证与合并节点。

### 任务 1：重置券展示模型

**文件：**
- 修改：`Sources/AIMeterCore/Presentation/AppPresentation.swift`
- 测试：`Tests/AIMeterCoreTests/AppPresentationTests.swift`

- [ ] **步骤 1：编写失败的日期状态和排序测试**

在 `AppPresentationTests` 增加使用固定 Gregorian 日历与时区的测试：

```swift
@Test("Orders reset credits and explains expiration by local calendar day")
func presentsResetCredits() {
  var calendar = Calendar(identifier: .gregorian)
  calendar.timeZone = TimeZone(secondsFromGMT: 0)!
  let now = Date(timeIntervalSince1970: 1_788_076_800) // 2026-08-30 00:00 UTC
  let summary = CodexResetCreditsSummary(
    availableCount: 4,
    credits: [
      CodexResetCreditDisplay(title: "Unknown", expiresAt: nil),
      CodexResetCreditDisplay(title: "Later", expiresAt: now.addingTimeInterval(3 * 86_400)),
      CodexResetCreditDisplay(title: "Today", expiresAt: now.addingTimeInterval(3_600)),
      CodexResetCreditDisplay(title: "Expired", expiresAt: now.addingTimeInterval(-86_400)),
    ],
    hasCompleteDetails: true
  )

  let value = CodexResetCreditsPresentation(summary: summary, now: now, calendar: calendar)

  #expect(value.availableText == "4 available")
  #expect(value.rows.map(\.title) == ["Expired", "Today", "Later", "Unknown"])
  #expect(value.rows.map(\.statusText) == ["Expired", "Expires today", "3 days remaining", "Expiration unavailable"])
}
```

再增加一项不完整数据测试，断言 `availableCount` 大于明细数量或 `hasCompleteDetails == false` 时，`showsIncompleteDetails` 为 `true`。

- [ ] **步骤 2：运行定向测试，确认类型尚不存在**

运行：

```bash
bash scripts/test.sh --filter AppPresentationTests
```

预期：编译失败，提示找不到 `CodexResetCreditsPresentation`。

- [ ] **步骤 3：实现最小纯展示模型**

在 `AppPresentation.swift` 增加：

```swift
public enum CodexResetCreditExpirationState: Equatable, Sendable {
  case remaining(days: Int)
  case today
  case expired
  case unavailable
}

public struct CodexResetCreditRowPresentation: Equatable, Sendable {
  public let title: String
  public let expiresAt: Date?
  public let statusText: String
  public let expirationState: CodexResetCreditExpirationState
}

public struct CodexResetCreditsPresentation: Equatable, Sendable {
  public let availableText: String
  public let rows: [CodexResetCreditRowPresentation]
  public let showsIncompleteDetails: Bool

  public init(
    summary: CodexResetCreditsSummary,
    now: Date = Date(),
    calendar: Calendar = .current
  ) {
    // 使用 calendar.startOfDay(for:) 计算自然日差；
    // 有日期的券按 expiresAt 升序稳定排列，无日期券置后；
    // 生成规格定义的四种 statusText。
  }
}
```

排序时保留原始索引作为第二排序键，避免相同日期或缺失日期的顺序漂移。`showsIncompleteDetails` 同时检查 `hasCompleteDetails` 与 `availableCount > credits.count`。

- [ ] **步骤 4：运行定向测试并确认通过**

运行：

```bash
bash scripts/test.sh --filter AppPresentationTests
```

预期：新增和既有展示测试全部通过。

- [ ] **步骤 5：提交展示模型检查点**

```bash
git add Sources/AIMeterCore/Presentation/AppPresentation.swift Tests/AIMeterCoreTests/AppPresentationTests.swift
git commit -m "feat: present Codex reset credit status"
```

### 任务 2：分层卡片与自适应详情面板

**文件：**
- 修改：`Sources/AIMeterApp/Views/CodexResetCreditsView.swift`
- 修改：`Sources/AIMeterApp/Views/CodexDetailView.swift`
- 修改：`Sources/AIMeterApp/Views/ProviderCard.swift`
- 创建：`Sources/AIMeterApp/System/CodexDetailPanelLayout.swift`
- 修改：`Sources/AIMeterApp/System/FloatingPanelController.swift`
- 测试：`Tests/AIMeterAppTests/CodexDetailPanelLayoutTests.swift`

- [ ] **步骤 1：编写失败的面板尺寸测试**

创建 `CodexDetailPanelLayoutTests.swift`：

```swift
import Testing
@testable import AIMeterApp

@Suite("Codex detail panel layout")
struct CodexDetailPanelLayoutTests {
  @Test("Grows for additional credits and respects the screen")
  func adaptiveHeight() {
    #expect(CodexDetailPanelLayout.height(creditCount: 0, availableHeight: 900) == 470)
    #expect(CodexDetailPanelLayout.height(creditCount: 1, availableHeight: 900) == 520)
    #expect(CodexDetailPanelLayout.height(creditCount: 3, availableHeight: 900) == 704)
    #expect(CodexDetailPanelLayout.height(creditCount: 8, availableHeight: 640) == 624)
  }
}
```

- [ ] **步骤 2：运行测试确认布局类型尚不存在**

运行：

```bash
bash scripts/test.sh --filter CodexDetailPanelLayoutTests
```

预期：编译失败，提示找不到 `CodexDetailPanelLayout`。

- [ ] **步骤 3：实现可测试的高度计算**

创建：

```swift
import Foundation

enum CodexDetailPanelLayout {
  static func height(creditCount: Int, availableHeight: CGFloat) -> CGFloat {
    let count = max(creditCount, 0)
    let contentHeight = count == 0 ? 470 : 520 + CGFloat(max(count - 1, 0)) * 92
    return min(contentHeight, max(availableHeight - 16, 0))
  }
}
```

在 `FloatingPanelController.positionPanels()` 中读取当前 Codex snapshot 的 `credits.count`，把 `.codex` 的固定 `470` 替换为上述计算结果。

- [ ] **步骤 4：运行尺寸测试确认通过**

运行：

```bash
bash scripts/test.sh --filter CodexDetailPanelLayoutTests
```

预期：4 个断言全部通过。

- [ ] **步骤 5：实现 A 方案 SwiftUI 布局**

为 `CodexResetCreditsView` 增加显示密度：

```swift
enum CodexResetCreditsDisplayMode {
  case detail
  case compact
}

struct CodexResetCreditsView: View {
  let summary: CodexResetCreditsSummary
  let mode: CodexResetCreditsDisplayMode
  // detail：标题 + 数量胶囊 + 每张独立券卡片；
  // compact：保留菜单栏所需的紧凑信息，不渲染大卡片。
}
```

详情模式必须包含：

- 薄荷色数量胶囊；
- 每券一个圆角内卡片与重置图标；
- 券类型、完整日期时间、`Expiration` 和状态文案；
- 缺失明细提示；
- 每张券独立无障碍标签。

在 `CodexDetailView` 传入 `.detail`，在 `ProviderCard` 传入 `.compact`。不得增加任何 `Button`。

- [ ] **步骤 6：构建 App 并检查 SwiftUI 编译**

运行：

```bash
swift build --product AIMeterApp
```

预期：构建成功，无 Swift 并发或访问级别错误。

- [ ] **步骤 7：提交界面检查点**

```bash
git add Sources/AIMeterApp/Views/CodexResetCreditsView.swift Sources/AIMeterApp/Views/CodexDetailView.swift Sources/AIMeterApp/Views/ProviderCard.swift Sources/AIMeterApp/System/CodexDetailPanelLayout.swift Sources/AIMeterApp/System/FloatingPanelController.swift Tests/AIMeterAppTests/CodexDetailPanelLayoutTests.swift
git commit -m "feat: redesign Codex reset credit cards"
```

### 任务 3：文档、完整验证与安装验收

**文件：**
- 修改：`CHANGELOG.md`
- 修改：`docs/user-guide/providers.md`
- 修改：`docs/development/testing.md`
- 创建：`docs/development/2026-08-31-codex-reset-credit-card.md`
- 修改：`docs/development/commit-history.md`

- [ ] **步骤 1：更新用户和开发文档**

记录以下事实：

- Codex 重置券现在以分层卡片显示；
- 数量、完整到期日期、自然日剩余状态和不完整明细提示的含义；
- 菜单栏继续采用紧凑模式；
- 详情面板高度按券数量增长并受屏幕高度约束；
- 功能仍为只读，不消耗或兑换重置券。

- [ ] **步骤 2：运行完整测试**

运行：

```bash
bash scripts/test.sh
```

预期：在 110 个既有测试基础上增加展示模型与面板尺寸测试，全部通过；仅环境依赖型 Keychain/真实 CLI 检查按设计跳过。

- [ ] **步骤 3：检查差异和文档链接**

运行：

```bash
git diff --check
rg -n 'TO[D]O|TB[D]|[待]定|FIXM[E]' docs/design/implementation-plans/2026-08-31-codex-reset-credit-card.md docs/design/specifications/2026-08-31-codex-reset-credit-card-design.md
```

预期：没有尾随空格、占位符或未完成描述。

- [ ] **步骤 4：构建并校验正式 App**

运行：

```bash
bash scripts/build-app.sh
plutil -lint "dist/AI Meter.app/Contents/Info.plist"
codesign --verify --deep --strict --verbose=2 "dist/AI Meter.app"
file "dist/AI Meter.app/Contents/MacOS/AIMeterApp"
```

预期：Release 构建成功、plist 合法、签名有效、可执行文件为 arm64 Mach-O。

- [ ] **步骤 5：安装并完成真实视觉验收**

先精确停止运行中的 AI Meter，把 `/Applications/AI Meter.app` 移到 `/private/tmp` 的本次备份路径，再用 `ditto` 安装新构建并启动。打开 Codex 详情验证：

- 单张券卡片清晰、无文字截断；
- 数量胶囊、完整日期与剩余天数同时可读；
- 官方额度和三项本机统计仍正常；
- 点击空白处与自动隐藏仍正常；
- 安装版可执行文件校验值与构建版一致。

- [ ] **步骤 6：提交验证与文档节点**

```bash
git add CHANGELOG.md docs/user-guide/providers.md docs/development/testing.md docs/development/2026-08-31-codex-reset-credit-card.md docs/development/commit-history.md
git commit -m "docs: verify Codex reset credit card redesign"
```

- [ ] **步骤 7：合并前最终审查**

运行完整测试、`git diff main...HEAD --check` 和 `git status --short --branch`。只有测试全绿、安装验收通过、分支工作区干净时，才按项目流程合入 `main`，记录 merge hash，并移除已合并 worktree。
