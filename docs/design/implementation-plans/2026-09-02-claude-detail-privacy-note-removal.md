# Claude 详情隐私说明移除实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 从 Claude 详情本机活动区域完整移除隐私说明文字与锁形图标，同时保持所有数据保护逻辑和其他详情内容不变。

**架构：** 改动限制在 `ClaudeDetailView` 展示层及其源码契约测试。采集、领域模型、缓存和隐私白名单不修改；用户文档只更新可见界面说明，不删除长期隐私边界文档。

**技术栈：** Swift 6、SwiftUI、Swift Testing、macOS 原生 App。

---

## 文件与职责

- `Sources/AIMeterApp/Views/ClaudeDetailView.swift`：删除本机活动区域底部的隐私 `Label`。
- `Tests/AIMeterAppTests/ClaudeDetailPresentationTests.swift`：先证明旧说明仍存在，再锁定说明文字和锁形图标不得回归。
- `CHANGELOG.md`、`docs/user-guide/providers.md`：同步当前可见行为。
- `docs/design/specifications/2026-09-02-claude-detail-card-removal-design.md`、`docs/development/2026-09-02-claude-detail-card-removal.md`：记录后续展示变更，不重写原始验收事实。
- `docs/development/2026-09-02-claude-detail-privacy-note-removal.md`、文档索引、提交历史与需求台账：记录本阶段证据。

### 任务 1：测试驱动移除隐私说明

**文件：**

- 修改：`Tests/AIMeterAppTests/ClaudeDetailPresentationTests.swift`
- 修改：`Sources/AIMeterApp/Views/ClaudeDetailView.swift`

- [ ] **步骤 1：编写失败的源码契约测试**

从原有“保留重要内容”测试中删除对隐私说明必须存在的正向断言，新增独立测试：

```swift
@Test("Claude detail omits the local privacy note")
func privacyNoteStaysAbsent() throws {
    let projectRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let source = try String(contentsOf: projectRoot.appending(
        path: "Sources/AIMeterApp/Views/ClaudeDetailView.swift"
    ))

    #expect(!source.contains("Only aggregate timestamps, token counts, session IDs and model IDs are read."))
    #expect(!source.contains("systemImage: \"lock.shield\""))
    #expect(source.contains("activityChart(summary)"))
    #expect(source.contains("Text(\"Updated "))
}
```

- [ ] **步骤 2：运行定向测试并确认红灯**

运行：

```bash
AIMETER_TEST_SCRATCH_PATH=/private/tmp/ai-meter-privacy-note-red \
  bash scripts/test.sh --filter ClaudeDetailPresentationTests
```

预期：新测试因旧文字与 `lock.shield` 仍存在而产生 2 个失败断言；不是编译错误。

- [ ] **步骤 3：编写最少生产代码**

从 `localActivitySection` 中删除：

```swift
Label(
    "Only aggregate timestamps, token counts, session IDs and model IDs are read. Conversation content stays private.",
    systemImage: "lock.shield"
)
.aiMeterFont(.caption2)
.foregroundStyle(AIMeterVisualTheme.tertiaryText)
```

不修改相邻统计、趋势、空状态或页脚。

- [ ] **步骤 4：运行定向测试并确认绿灯**

运行：

```bash
AIMETER_TEST_SCRATCH_PATH=/private/tmp/ai-meter-privacy-note-green \
  bash scripts/test.sh --filter ClaudeDetailPresentationTests
```

预期：Claude 详情展示测试 5/5 通过。

- [ ] **步骤 5：提交功能节点**

```bash
git add Sources/AIMeterApp/Views/ClaudeDetailView.swift \
  Tests/AIMeterAppTests/ClaudeDetailPresentationTests.swift
git commit -m "feat: remove Claude detail privacy note"
```

### 任务 2：同步文档并完成交付验证

**文件：**

- 修改：`CHANGELOG.md`
- 修改：`docs/user-guide/providers.md`
- 修改：`docs/design/specifications/2026-09-02-claude-detail-card-removal-design.md`
- 修改：`docs/development/2026-09-02-claude-detail-card-removal.md`
- 创建：`docs/development/2026-09-02-claude-detail-privacy-note-removal.md`
- 修改：`docs/development/README.md`
- 修改：`docs/development/commit-history.md`
- 修改：`docs/README.md`
- 修改：`docs/requirements-backlog.md`

- [ ] **步骤 1：更新当前行为与历史说明**

在变更日志和用户指南中写明 Claude 本机活动区域不再显示单独隐私提示；在上一阶段规格与验收日志顶部增加后续变更说明，强调底层隐私边界不变。

- [ ] **步骤 2：创建本阶段开发验收日志并关闭需求**

记录红灯、绿灯、全量测试数量、Release 签名、安装哈希、旧版备份路径、真实详情打开/自动隐藏状态和非激活面板截图限制。将 `REQ-20260902-009` 标记为“已完成”。

- [ ] **步骤 3：运行完整测试**

运行：

```bash
AIMETER_TEST_SCRATCH_PATH=/private/tmp/ai-meter-privacy-note-full \
  bash scripts/test.sh
```

预期：295 项测试、58 个测试组通过，0 失败。

- [ ] **步骤 4：构建和验证 Release 候选包**

运行：

```bash
AI_METER_INCLUDE_WIDGET=0 bash scripts/build-app.sh
codesign --verify --deep --strict --verbose=2 "dist/AI Token Meter.app"
```

预期：构建完成、签名有效、主程序为 arm64。

- [ ] **步骤 5：安装并验收真实桌面行为**

退出当前 AI Token Meter，将旧安装版备份到 `/private/tmp`，安装候选包到 `/Applications/AI Token Meter.app` 并启动。点击 Claude 后确认详情状态打开，等待当前 8 秒自动隐藏后确认关闭；安装版主程序哈希必须与候选包一致。

- [ ] **步骤 6：提交文档节点**

```bash
git add CHANGELOG.md docs
git commit -m "docs: record Claude privacy note cleanup"
```

- [ ] **步骤 7：独立审查、最终验证并合并**

请求独立代码审查，修复全部 Critical/Important 问题；在最终分支和合并后的 `main` 各运行一次完整测试，随后清理本次 worktree 与分支。
