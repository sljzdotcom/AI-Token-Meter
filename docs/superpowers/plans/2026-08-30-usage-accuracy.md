# AI Meter 用量准确性修复实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 准确读取 Claude 与 Codex 通用额度，并让圆环和中央数字始终表达同一个真实用量窗口。

**架构：** Claude 文本解析器先验证百分比所在行具备用量语义；Codex 采集器以顶层通用 `rateLimits` 为权威，不再由模型专用额度替换。展示层选择最高已用的有界指标作为唯一摘要指标，并把零值转换为“不绘制前景弧”。

**技术栈：** Swift 6、Swift Testing、SwiftUI、Codex app-server JSON-RPC、Claude Code PTY 文本解析。

---

## 文件结构

- 创建 `Tests/AIMeterCoreTests/Fixtures/claude-usage-promo-en.txt`：保存脱敏后的 Claude 2.1.251 用量页结构，覆盖促销百分比。
- 修改 `Tests/AIMeterCoreTests/ClaudeUsageParserTests.swift`：锁定促销百分比不会成为额度。
- 修改 `Sources/AIMeterCore/Collectors/TerminalUsageParser.swift`：仅接受具备用量语义的百分比行。
- 修改 `Tests/AIMeterCoreTests/CLICollectorTests.swift`：锁定 Codex 顶层通用额度优先于模型专用额度。
- 创建 `Tests/AIMeterCoreTests/Fixtures/fake-codex-general-and-model.sh`：返回通用 5% 与 Spark 0%/0% 的真实结构化响应。
- 修改 `Sources/AIMeterCore/Collectors/CodexAppServerClient.swift`：保留顶层通用额度，不做模型专用整体替换。
- 修改 `Tests/AIMeterCoreTests/AppPresentationTests.swift`：锁定摘要数字、圆环比例和零弧语义。
- 修改 `Sources/AIMeterCore/Presentation/AppPresentation.swift`：选择唯一摘要指标并提供 `ringFraction`。
- 修改 `Sources/AIMeterApp/Views/UsageRing.swift`：只在 `ringFraction` 大于零时绘制前景弧。
- 修改 `docs/development/2026-08-28-development-log.md`：记录根因、红绿证据、真实接口和安装验收。

### 任务 1：Claude 促销百分比过滤

**文件：**
- 创建：`Tests/AIMeterCoreTests/Fixtures/claude-usage-promo-en.txt`
- 修改：`Tests/AIMeterCoreTests/ClaudeUsageParserTests.swift`
- 修改：`Sources/AIMeterCore/Collectors/TerminalUsageParser.swift:10-52`

- [ ] **步骤 1：创建真实形状 fixture 并编写失败测试**

Fixture 包含：

```text
Current session
0% 0% used
Resets 8:10pm (Asia/Singapore)
Current week (all models)
0% 0% used
Resets Sep 5 at 3:59pm (Asia/Singapore)
+50% weekly limits promo through Aug 31
```

测试断言：

```swift
@Test("Ignores Claude promotional percentages")
func ignoresPromotionalPercentages() throws {
    let snapshot = try ClaudeUsageParser().parse(fixture("claude-usage-promo-en"))
    #expect(snapshot.primaryMetric?.usedFraction == 0)
    #expect(snapshot.secondaryMetric?.usedFraction == 0)
}
```

- [ ] **步骤 2：运行测试并确认红灯**

运行：

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-usage-accuracy --filter 'ClaudeUsageParserTests/ignoresPromotionalPercentages'
```

预期：FAIL，secondary 为 0.50，证明促销行覆盖了真实周额度。

- [ ] **步骤 3：实现最小用量语义过滤**

在百分比转换前增加：

```swift
guard isUsagePercentLine(line) else { continue }
```

`isUsagePercentLine` 只接受英文 `used/remaining/left` 或中文 `已用/使用/剩余/可用`；不接受仅含 `promo`、`higher` 或普通说明的百分比行。

- [ ] **步骤 4：运行 Claude 解析专项并确认绿灯**

运行：

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-usage-accuracy --filter 'ClaudeUsageParserTests'
```

预期：4 个 Claude 解析测试全部 PASS。

- [ ] **步骤 5：保存任务提交**

```bash
git add Sources/AIMeterCore/Collectors/TerminalUsageParser.swift Tests/AIMeterCoreTests/ClaudeUsageParserTests.swift Tests/AIMeterCoreTests/Fixtures/claude-usage-promo-en.txt
git commit -m "fix: ignore promotional usage percentages (task 1/3)"
```

### 任务 2：Codex 通用额度优先

**文件：**
- 创建：`Tests/AIMeterCoreTests/Fixtures/fake-codex-general-and-model.sh`
- 修改：`Tests/AIMeterCoreTests/CLICollectorTests.swift`
- 修改：`Sources/AIMeterCore/Collectors/CodexAppServerClient.swift:140-157`

- [ ] **步骤 1：编写失败的通用额度测试**

为 fixture 增加可选响应：顶层 `rateLimits.primary.usedPercent = 5`、窗口 10080 分钟、secondary 为 null；`rateLimitsByLimitId.codex_bengalfox` 返回 0% 的 300 与 10080 分钟窗口。测试通过专用 fixture URL 调用 `CodexAppServerClient.readRateLimits`：

```swift
@Test("Codex keeps the general limit instead of replacing it with a model limit")
func codexPrefersGeneralLimit() async throws {
    let snapshot = try await CodexAppServerClient().readRateLimits(
        executableURL: generalAndModelCodexExecutable
    )
    #expect(snapshot.primaryMetric?.label == "Weekly limit")
    #expect(snapshot.primaryMetric?.usedFraction == 0.05)
    #expect(snapshot.secondaryMetric == nil)
}
```

- [ ] **步骤 2：运行测试并确认红灯**

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-usage-accuracy --filter 'CLICollectorTests/codexPrefersGeneralLimit'
```

预期：FAIL，当前实现返回 Spark 的 0% 主、次窗口。

- [ ] **步骤 3：删除错误的整体替换**

`snapshot(from:)` 直接从 `result.rateLimits` 创建快照；不因 secondary 为 nil 搜索并替换为 `rateLimitsByLimitId`。保留现有窗口时长标签逻辑，因此 10080 分钟显示为 `Weekly limit`。

- [ ] **步骤 4：运行采集器专项并确认绿灯**

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-usage-accuracy --filter 'CLICollectorTests'
```

预期：新增通用额度测试与原有 8 个测试全部 PASS。

- [ ] **步骤 5：保存任务提交**

```bash
git add Sources/AIMeterCore/Collectors/CodexAppServerClient.swift Tests/AIMeterCoreTests/CLICollectorTests.swift Tests/AIMeterCoreTests/Fixtures/fake-codex-general-and-model.sh
git commit -m "fix: prefer Codex general rate limit (task 2/3)"
```

### 任务 3：摘要数字与圆环一致

**文件：**
- 修改：`Tests/AIMeterCoreTests/AppPresentationTests.swift`
- 修改：`Sources/AIMeterCore/Presentation/AppPresentation.swift:11-85`
- 修改：`Sources/AIMeterApp/Views/UsageRing.swift:8-18`
- 修改：`docs/development/2026-08-28-development-log.md`

- [ ] **步骤 1：编写最高窗口与零弧失败测试**

```swift
@Test("Uses the same highest window for summary text and ring")
func summarizesHighestWindowConsistently() {
    let snapshot = UsageSnapshot(
        provider: .claude,
        primaryMetric: UsageMetric(label: "Current session", current: 0, limit: 100, unit: .percent),
        secondaryMetric: UsageMetric(label: "Weekly limit", current: 10, limit: 100, unit: .percent)
    )
    let presentation = ProviderPresentation(snapshot: snapshot)
    #expect(presentation.valueText == "10%")
    #expect(presentation.fraction == 0.10)
    #expect(presentation.ringFraction == 0.10)
}

@Test("Zero and unbounded metrics do not draw progress")
func hidesFalseProgress() {
    let zero = ProviderPresentation(snapshot: usageSnapshot(provider: .codex, fraction: 0))
    let balance = ProviderPresentation(snapshot: UsageSnapshot(
        provider: .deepSeek,
        primaryMetric: UsageMetric(label: "Balance", current: 77.99, limit: nil, unit: .cny, kind: .balance)
    ))
    #expect(zero.ringFraction == nil)
    #expect(balance.ringFraction == nil)
}
```

- [ ] **步骤 2：运行测试并确认红灯**

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-usage-accuracy --filter 'AppPresentationTests'
```

预期：最高窗口测试得到文字 0%，并且 `ringFraction` 尚不存在。

- [ ] **步骤 3：实现唯一摘要指标与零弧**

在 `ProviderPresentation` 中从主、次指标选择 `usedFraction` 最大者作为 `summaryMetric`；`valueText` 和 `fraction` 均来自该指标。新增：

```swift
public var ringFraction: Double? {
    guard let fraction, fraction > 0 else { return nil }
    return min(fraction, 1)
}
```

`UsageRing` 用 `if let ringFraction` 包裹前景 `Circle`，删除 4% 下限。

- [ ] **步骤 4：运行展示与完整测试**

```bash
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-usage-accuracy --filter 'AppPresentationTests'
swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-usage-accuracy
```

预期：展示专项与完整套件 0 failures；环境相关测试按设计跳过。

- [ ] **步骤 5：运行真实 CLI、构建与安装验收**

```bash
AI_METER_RUN_CLI_SMOKE=1 swift test --disable-sandbox --scratch-path /private/tmp/ai-meter-usage-accuracy --filter 'CLIIntegrationSmokeTests'
./scripts/build-app.sh
codesign --verify --deep --strict --verbose=2 'dist/AI Meter.app'
plutil -lint 'dist/AI Meter.app/Contents/Info.plist'
```

安装前把当前 `/Applications/AI Meter.app` 移到唯一的 `/private/tmp` 恢复目录；安装新包并启动。验收 Claude 为 0% 且无前景弧，Codex 为 5% 且圆环约 5%，DeepSeek 显示余额但不伪造进度。

- [ ] **步骤 6：更新日志并保存任务提交**

```bash
git add Sources/AIMeterCore/Presentation/AppPresentation.swift Sources/AIMeterApp/Views/UsageRing.swift Tests/AIMeterCoreTests/AppPresentationTests.swift docs/development/2026-08-28-development-log.md
git commit -m "fix: align usage summaries and rings (task 3/3)"
```
