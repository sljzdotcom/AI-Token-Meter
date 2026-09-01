# AI Token Meter Claude 详情与本机活动实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 将 Claude 紧凑详情升级为官方额度优先的专用详情页，并安全展示当前 Mac 最近 30 天的 Claude Code 聚合活动。

**架构：** `ClaudeCollector` 并行获取官方 `/usage` 与可选的 `ClaudeLocalActivityReader` 聚合结果；本机失败不影响官方快照。新聚合模型随 `UsageSnapshot` 经过缓存和脱敏路径传递，`ClaudeDetailView` 明确分隔 Official quota 与 Last 30 days · This Mac。

**技术栈：** Swift 6、Foundation JSONL 流式解析、SwiftUI、Swift Charts、Swift Testing、Swift Package Manager

---

## 文件结构

- 创建 `Sources/AIMeterCore/Collectors/ClaudeLocalActivityReader.swift`：最小 JSON 解码、30 日聚合和只读目录扫描。
- 修改 `Sources/AIMeterCore/Collectors/ClaudeCollector.swift`：并行组合官方额度与可选本机活动。
- 修改 `Sources/AIMeterCore/Domain/ProviderSupplementalData.swift`：Claude 日、模型和汇总数据模型。
- 修改 `Sources/AIMeterCore/Domain/UsageModels.swift`：快照增加 `claudeLocalActivity` 及复制方法。
- 修改 `Sources/AIMeterCore/Security/SensitiveTextRedactor.swift`、`Sources/AIMeterCore/Coordination/RefreshCoordinator.swift`、`Sources/AIMeterApp/AppModel.swift`：保留安全聚合数据。
- 创建 `Sources/AIMeterApp/Views/ClaudeDetailView.swift`：专用详情页面。
- 创建 `Sources/AIMeterApp/System/ClaudeDetailPanelLayout.swift`：自适应详情高度策略。
- 修改 `Sources/AIMeterApp/Views/FloatingStripView.swift`、`Sources/AIMeterApp/System/FloatingPanelController.swift`：路由 Claude 专用页与尺寸。
- 创建 `Tests/AIMeterCoreTests/ClaudeLocalActivityTests.swift`：解析、汇总、隐私和降级测试。
- 修改 `Tests/AIMeterCoreTests/ClaudeCollectorTests.swift`：官方/本机并行与可选失败测试。
- 修改 `Tests/AIMeterCoreTests/SensitiveTextRedactorTests.swift`、`Tests/AIMeterCoreTests/RefreshCoordinatorTests.swift`：快照传递测试。
- 创建 `Tests/AIMeterAppTests/ClaudeDetailPanelLayoutTests.swift`：面板尺寸测试。
- 修改 `docs/architecture/data-sources.md`、`docs/user-guide/usage.md`、`docs/development/README.md`、`docs/requirements-backlog.md`；创建本阶段开发日志。

### 任务 1：Claude 本机活动领域模型与快照传递

**文件：**
- 修改：`Sources/AIMeterCore/Domain/ProviderSupplementalData.swift`
- 修改：`Sources/AIMeterCore/Domain/UsageModels.swift`
- 修改：`Tests/AIMeterCoreTests/SensitiveTextRedactorTests.swift`
- 修改：`Sources/AIMeterCore/Security/SensitiveTextRedactor.swift`
- 修改：`Tests/AIMeterCoreTests/RefreshCoordinatorTests.swift`
- 修改：`Sources/AIMeterCore/Coordination/RefreshCoordinator.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`

- [x] **步骤 1：编写失败的聚合模型与快照保留测试**

测试构造：

```swift
let activity = ClaudeLocalActivitySummary(
    days: [ClaudeDailyActivity(date: referenceDate, inputTokens: 10, outputTokens: 20, cacheTokens: 30)],
    sessionCount: 2,
    activeDayCount: 1,
    models: [ClaudeModelActivity(modelID: "claude-sonnet-4-6", tokenCount: 60)],
    updatedAt: referenceDate,
    dayCount: 30
)
let snapshot = UsageSnapshot(provider: .claude, claudeLocalActivity: activity)
#expect(snapshot.claudeLocalActivity == activity)
#expect(SensitiveTextRedactor.redact(snapshot).claudeLocalActivity == activity)
```

在刷新协调测试中，把 activity 放入成功快照，断言 stale/fresh 包装后仍相等。

- [x] **步骤 2：运行测试验证失败**

运行：`swift test --filter SensitiveTextRedactorTests`

预期：FAIL，Claude 聚合类型和快照属性不存在。

- [x] **步骤 3：实现不可含正文的领域模型**

新增 `ClaudeDailyActivity`、`ClaudeModelActivity`、`ClaudeLocalActivitySummary`；构造器把负数钳制为零、`dayCount` 至少为 1。`UsageSnapshot` 增加默认 `nil` 的属性和：

```swift
func withClaudeLocalActivity(_ activity: ClaudeLocalActivitySummary?) -> UsageSnapshot
```

更新快照重建位置，显式复制该字段。模型只允许 Date、Int/Int64 和 model ID，不新增文本内容字段。

- [x] **步骤 4：运行相关与完整测试**

运行：

```bash
swift test --filter SensitiveTextRedactorTests
swift test --filter RefreshCoordinatorTests
swift test
```

预期：全部 PASS。

- [x] **步骤 5：Commit**

```bash
git add Sources/AIMeterCore/Domain/ProviderSupplementalData.swift Sources/AIMeterCore/Domain/UsageModels.swift Sources/AIMeterCore/Security/SensitiveTextRedactor.swift Sources/AIMeterCore/Coordination/RefreshCoordinator.swift Sources/AIMeterApp/AppModel.swift Tests/AIMeterCoreTests/SensitiveTextRedactorTests.swift Tests/AIMeterCoreTests/RefreshCoordinatorTests.swift
git commit -m "feat: add Claude local activity snapshot data"
```

### 任务 2：隐私受限的 JSONL 读取与 30 日聚合

**文件：**
- 创建：`Tests/AIMeterCoreTests/ClaudeLocalActivityTests.swift`
- 创建：`Sources/AIMeterCore/Collectors/ClaudeLocalActivityReader.swift`

- [x] **步骤 1：编写失败的最小字段解析测试**

用临时目录写入主会话和 `subagents` JSONL。每行同时含有诱饵正文数字和真实 usage：

```swift
{"timestamp":"2026-09-01T01:00:00.000Z","sessionId":"main-1","message":{"model":"claude-sonnet-4-6","content":[{"type":"text","text":"ignore 999999 tokens"}],"usage":{"input_tokens":10,"output_tokens":20,"cache_creation_input_tokens":30,"cache_read_input_tokens":40}}}
```

断言总数为 100 而非正文数字；主会话计 1 session；同日 subagent 增加 token 但不增加 session；模型聚合正确。

- [x] **步骤 2：编写失败的 30 日边界与降级测试**

固定 Asia/Singapore 日历和 `now`，验证：今天与第 29 天纳入，第 30 天排除；无记录返回 30 个零日；损坏行跳过；负 token 跳过；超过单行上限的记录跳过；不可读目录抛出 `directoryUnavailable`。

- [x] **步骤 3：运行测试验证失败**

运行：`swift test --filter ClaudeLocalActivityTests`

预期：FAIL，reader、parser 和 summarizer 尚不存在。

- [x] **步骤 4：实现最小白名单解码与聚合**

创建只声明允许字段的 Codable 结构：

```swift
private struct ClaudeLogEntry: Decodable {
    let timestamp: String
    let sessionId: String?
    let message: Message?
    struct Message: Decodable {
        let model: String?
        let usage: Usage?
    }
    struct Usage: Decodable {
        let inputTokens: Int64?
        let outputTokens: Int64?
        let cacheCreationInputTokens: Int64?
        let cacheReadInputTokens: Int64?
    }
}
```

为 snake_case 字段提供 CodingKeys。解析器使用带小数秒和不带小数秒的 ISO8601 formatter；以带溢出保护的加法汇总四类 token。Reader 只枚举 `.jsonl`，跳过符号链接；使用 2 MiB 单行和 256 MiB 单文件上限，在 utility 任务中处理。通过路径是否包含 `/subagents/` 判断 session 计数资格。

- [x] **步骤 5：运行测试验证通过**

运行：`swift test --filter ClaudeLocalActivityTests`

预期：全部 PASS，测试日志不出现诱饵正文。

- [x] **步骤 6：Commit**

```bash
git add Sources/AIMeterCore/Collectors/ClaudeLocalActivityReader.swift Tests/AIMeterCoreTests/ClaudeLocalActivityTests.swift
git commit -m "feat: aggregate local Claude Code activity"
```

### 任务 3：官方额度与本机活动并行采集

**文件：**
- 修改：`Tests/AIMeterCoreTests/ClaudeCollectorTests.swift`
- 修改：`Sources/AIMeterCore/Collectors/ClaudeCollector.swift`

- [x] **步骤 1：编写失败的组合与降级测试**

增加可注入的 `ClaudeLocalActivityReading` stub：成功时返回固定 summary，断言官方 primary/secondary metric 不变且 `claudeLocalActivity` 被附加；失败时抛错，断言 `collect()` 仍成功且本机字段为 nil；官方认证失败时仍抛原有错误。

- [x] **步骤 2：运行测试验证失败**

运行：`swift test --filter ClaudeCollectorTests`

预期：FAIL，collector 尚未接收本机 reader。

- [x] **步骤 3：实现并行可选采集**

为 reader 定义 Sendable 协议并注入默认实现。在 `collect()` 中：

```swift
async let official = collectOfficialUsage()
async let local = optionalLocalActivity()
return try await official.withClaudeLocalActivity(local)
```

把现有官方逻辑原样移入 `collectOfficialUsage()`；`optionalLocalActivity()` 捕获全部本机错误并返回 nil，不吞掉官方错误。

- [x] **步骤 4：运行测试验证通过**

运行：

```bash
swift test --filter ClaudeCollectorTests
swift test --filter CLIIntegrationSmokeTests
```

预期：全部 PASS。

- [x] **步骤 5：Commit**

```bash
git add Sources/AIMeterCore/Collectors/ClaudeCollector.swift Tests/AIMeterCoreTests/ClaudeCollectorTests.swift
git commit -m "feat: attach optional Claude local activity"
```

### 任务 4：Claude 专用详情页与自适应窗口

**文件：**
- 创建：`Sources/AIMeterApp/Views/ClaudeDetailView.swift`
- 创建：`Sources/AIMeterApp/System/ClaudeDetailPanelLayout.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift`
- 修改：`Sources/AIMeterApp/System/FloatingPanelController.swift`
- 创建：`Tests/AIMeterAppTests/ClaudeDetailPanelLayoutTests.swift`
- 修改：`Tests/AIMeterAppTests/FloatingStripLayoutTests.swift`

- [x] **步骤 1：编写失败的面板尺寸测试**

验证标准屏幕返回 `CGSize(width: 390, height: 560)`；可用高度较小时高度不超过 `availableHeight - 24` 且不低于 500；`FloatingPanelController` 的 Claude 分支使用该策略而非旧 300×260。

- [x] **步骤 2：运行测试验证失败**

运行：`swift test --filter ClaudeDetailPanelLayoutTests`

预期：FAIL，布局类型不存在。

- [x] **步骤 3：实现详情页骨架与官方额度卡片**

创建 `ClaudeDetailView`，复用 Codex 的标题、quota card、glass card、progress bar 和主题语义，但 provider 固定 `.claude`。标题副标题为 `Official quota · Local activity`，两张额度卡来自 primary/secondary metric。

- [x] **步骤 4：实现本机统计、趋势和构成**

使用 Swift Charts 绘制固定 30 天柱形图；三张统计卡显示 Total token、Sessions、Active days；构成行显示 Input、Output、Cache；模型区按 token 降序最多显示三项。nil 显示 unavailable 卡，合法零数据显示空趋势和 0 值。所有标签包含 `official quota` 或 `local estimate` 可访问性描述。

- [x] **步骤 5：接入路由与窗口尺寸**

在 `FloatingDetailView` 增加 `.claude` 分支；`FloatingPanelController.preferredDetailSize` 调用 `ClaudeDetailPanelLayout.size(availableHeight:)`。保留 hover、置前、空白点击和自动隐藏逻辑。

- [x] **步骤 6：运行 UI 与完整测试**

运行：

```bash
swift test --filter ClaudeDetailPanelLayoutTests
swift test --filter FloatingStripLayoutTests
swift test
```

预期：全部 PASS。

- [x] **步骤 7：Commit**

```bash
git add Sources/AIMeterApp/Views/ClaudeDetailView.swift Sources/AIMeterApp/System/ClaudeDetailPanelLayout.swift Sources/AIMeterApp/Views/FloatingStripView.swift Sources/AIMeterApp/System/FloatingPanelController.swift Tests/AIMeterAppTests/ClaudeDetailPanelLayoutTests.swift Tests/AIMeterAppTests/FloatingStripLayoutTests.swift
git commit -m "feat: add rich Claude detail view"
```

### 任务 5：文档、隐私扫描、Release 安装与真实验收

**文件：**
- 修改：`docs/architecture/data-sources.md`
- 修改：`docs/user-guide/usage.md`
- 修改：`docs/development/README.md`
- 创建：`docs/development/2026-09-01-claude-detail-local-activity.md`
- 修改：`docs/requirements-backlog.md`

- [x] **步骤 1：补充数据口径与隐私文档**

明确 Official quota 来自 `/usage`；Last 30 days · This Mac 只包含本机 Claude Code 聚合元数据，排除 Web/Desktop/其他设备；说明本机不可用不会影响官方额度。

- [x] **步骤 2：执行隐私与占位符扫描**

运行：

```bash
rg -n "content|prompt|cwd|gitBranch|slug" Sources/AIMeterCore/Collectors/ClaudeLocalActivityReader.swift
rg -n "TO""DO|TB""D|PLACE""HOLDER" Sources Tests docs/superpowers/specs docs/superpowers/plans
git diff --check
```

预期：reader 不声明或输出正文、cwd、branch、slug 字段；无新增占位符；diff 无空白错误。

- [x] **步骤 3：执行完整测试与 Release 构建**

运行：

```bash
swift test
swift build -c release
```

预期：全部成功。

- [x] **步骤 4：安装并进行真实桌面验收**

使用项目现有安装流程安装签名候选版。点击 Claude，核对官方额度与 Claude 客户端 Usage；确认 This Mac 标识、30 日柱图、三项统计、token 构成和模型区出现；确认自动隐藏、点击空白关闭、置前、左右贴边和 Antonio 字体无回归。

- [x] **步骤 5：记录证据并关闭需求**

开发日志记录测试数量、构建签名、安装指纹、真实数据口径、隐私扫描和 UI 验收。将 `REQ-20260901-006` 更新为已完成并链接规格、计划、开发日志和关键提交。

- [x] **步骤 6：Commit**

```bash
git add docs/architecture/data-sources.md docs/user-guide/usage.md docs/development/README.md docs/development/2026-09-01-claude-detail-local-activity.md docs/requirements-backlog.md
git commit -m "docs: record Claude detail acceptance"
```
