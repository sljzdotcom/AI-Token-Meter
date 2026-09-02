# Claude 详情卡片精简实施计划

> **执行方式：** 在 `codex/claude-detail-cleanup` 隔离分支中按 TDD 顺序实施；每个关键节点单独提交。

**目标：** 从 Claude 详情页移除 `Token composition` 和 `Top models` 两张展示卡片，同时保留官方额度、本机三项统计、每日趋势、空状态、隐私提示及现有采集和缓存兼容性。

**架构边界：** 只改 SwiftUI 展示层及对应展示辅助代码。`AIMeterCore` 的 Claude 本机活动模型、采集器、缓存字段和历史数据读取保持不变，避免旧缓存失效或扩大改动范围。

**技术栈：** Swift 6、SwiftUI、Swift Charts、Swift Testing、macOS 原生 App。

---

## 任务 1：用回归测试锁定精简后的 Claude 详情结构

**文件：**

- 修改：`Tests/AIMeterAppTests/ClaudeDetailPresentationTests.swift`

### 步骤 1：先添加会失败的源码契约测试

新增测试，读取 `ClaudeDetailView.swift` 并断言：

```swift
#expect(!source.contains("tokenComposition(summary)"))
#expect(!source.contains("modelBreakdown("))
#expect(!source.contains("\"Token composition\""))
#expect(!source.contains("\"Top models\""))
#expect(source.contains("localStat(title: \"Sessions\""))
#expect(source.contains("activityChart(summary)"))
#expect(source.contains("Conversation content stays private."))
```

### 步骤 2：运行定向测试并确认红灯

运行：

```bash
AIMETER_TEST_SCRATCH_PATH=/private/tmp/ai-meter-card-cleanup-red \
  bash scripts/test.sh --filter ClaudeDetailPresentationTests
```

预期：新测试因为两个旧卡片仍在源码中而失败；其他既有测试不应出现无关错误。

## 任务 2：实施最小展示层改动并恢复绿灯

**文件：**

- 修改：`Sources/AIMeterApp/Views/ClaudeDetailView.swift`
- 修改：`Sources/AIMeterApp/Views/ClaudeDetailPresentation.swift`
- 修改：`Tests/AIMeterAppTests/ClaudeDetailPresentationTests.swift`

### 步骤 1：移除两张卡片的视图入口和私有视图函数

从 `localActivitySection` 移除：

```swift
tokenComposition(summary)
let modelRows = ClaudeDetailPresentation.topModelRows(summary)
if !modelRows.isEmpty {
    modelBreakdown(modelRows)
}
```

并删除仅服务于这两张卡片的 `tokenComposition`、`tokenRow` 与 `modelBreakdown` 私有函数。

### 步骤 2：清理不再使用的展示辅助类型

删除 `ClaudeModelRowPresentation` 与 `ClaudeDetailPresentation.topModelRows`。保留核心层的 `ClaudeModelActivity` 和本机活动数据字段，不改变采集、缓存或迁移行为。

### 步骤 3：删除已经失去产品意义的 Top models 单元测试

删除两项仅验证 `topModelRows` 百分比计算的测试；保留并运行新的视图结构回归测试、空状态、额度与无障碍测试。

### 步骤 4：运行定向测试并确认绿灯

运行：

```bash
AIMETER_TEST_SCRATCH_PATH=/private/tmp/ai-meter-card-cleanup-green \
  bash scripts/test.sh --filter ClaudeDetailPresentationTests
```

预期：Claude 详情展示测试全部通过。

### 步骤 5：提交功能节点

```bash
git add Sources/AIMeterApp/Views/ClaudeDetailView.swift \
  Sources/AIMeterApp/Views/ClaudeDetailPresentation.swift \
  Tests/AIMeterAppTests/ClaudeDetailPresentationTests.swift
git commit -m "feat: simplify Claude detail activity cards"
```

## 任务 3：同步用户文档、开发记录与需求台账

**文件：**

- 修改：`README.md`
- 修改：`CHANGELOG.md`
- 修改：`docs/user-guide/providers.md`
- 修改：`docs/design/specifications/2026-09-01-claude-detail-local-activity-design.md`
- 修改：`docs/development/2026-09-01-claude-detail-local-activity.md`
- 新增：`docs/development/2026-09-02-claude-detail-card-removal.md`
- 修改：`docs/development/README.md`
- 修改：`docs/README.md`
- 修改：`docs/requirements-backlog.md`

### 步骤 1：统一面向用户的功能描述

将 Claude 详情页说明调整为：官方额度、本机 Sessions、Active days、Tokens、30 天每日趋势、隐私提示。删除仍宣称显示 Token composition 或 Top models 的文字。

### 步骤 2：记录兼容边界和版本变更

说明展示卡片已精简，但历史缓存中的 Token 构成和模型标识仍保留并可被安全读取；记录测试、构建、安装与真实桌面验收结果。

### 步骤 3：在最终验收后关闭需求

将 `REQ-20260901-008` 标记为“已完成”，补充完成日期、实现证据、验证命令与提交记录。

### 步骤 4：提交文档节点

```bash
git add README.md CHANGELOG.md docs
git commit -m "docs: record Claude detail card cleanup"
```

## 任务 4：完整验证、安装与真实桌面验收

### 步骤 1：运行完整测试

```bash
AIMETER_TEST_SCRATCH_PATH=/private/tmp/ai-meter-card-cleanup-full \
  bash scripts/test.sh
```

预期：全部测试通过，测试数量相对基线减少一项（新增一项结构回归、删除两项废弃展示辅助测试）。

### 步骤 2：构建无 Widget 的正式候选包

```bash
AI_METER_INCLUDE_WIDGET=0 \
AI_METER_BUILD_DIR=/private/tmp/ai-meter-card-cleanup-release \
  bash scripts/build-app.sh
```

验证候选包签名、Bundle 标识与可执行文件。

### 步骤 3：备份并安装候选包

退出当前 AI Token Meter，将旧版 App 备份到 `/private/tmp`，安装候选包到 `/Applications/AI Token Meter.app`，再启动应用。

### 步骤 4：真实桌面验收

点击 Claude 浮动条图标，确认：

- 详情面板位于其他窗口上方；
- 官方额度卡片仍显示；
- Sessions、Active days、Tokens 与每日趋势仍显示；
- `Token composition` 与 `Top models` 不再显示；
- 隐私提示和更新时间仍显示；
- 自动隐藏与点击空白关闭行为未回归。

### 步骤 5：代码审查与合并

完成独立代码审查，修复阻断问题，再将 `codex/claude-detail-cleanup` 合并回 `main`，并确认主分支工作区干净。
