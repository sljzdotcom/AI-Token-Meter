# 详情窗口置前实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 让浮动条点击产生的详情窗口显示在所有普通应用窗口上方，同时保留浮动条的桌面层级和现有焦点行为。

**架构：** 将现有单一窗口展示策略拆分为 `strip` 与 `detail` 两种角色；控制器在创建窗口时分配角色。详情使用标准 `.floating` 层级，浮动条继续使用桌面层级，两者共享现有 Space 行为。

**技术栈：** Swift 6、AppKit `NSPanel` / `NSWindow.Level`、Swift Testing、现有 macOS Release 构建与验收脚本。

---

## 文件结构

### 修改

- `Sources/AIMeterApp/System/FloatingPanelPresentationPolicy.swift`：定义面板角色并按角色返回、应用窗口层级。
- `Sources/AIMeterApp/System/FloatingPanelController.swift`：创建浮动条和详情面板时传入对应角色。
- `Tests/AIMeterAppTests/FloatingPanelPresentationPolicyTests.swift`：验证两个角色的窗口层级和共享 Space 行为。
- `docs/requirements-backlog.md`：完成需求状态和证据。
- `docs/development/2026-09-01-detail-panel-frontmost.md`：记录测试、构建、安装和真实桌面验收。
- `README.md`、`CHANGELOG.md`、`docs/development/README.md`、`docs/development/commit-history.md`：同步用户功能和版本记录。

## 任务 1：角色化窗口展示策略

**文件：**
- 修改：`Tests/AIMeterAppTests/FloatingPanelPresentationPolicyTests.swift`
- 修改：`Sources/AIMeterApp/System/FloatingPanelPresentationPolicy.swift`
- 修改：`Sources/AIMeterApp/System/FloatingPanelController.swift`

- [ ] **步骤 1：编写失败的角色层级测试**

```swift
@Test("Detail panels float above ordinary application windows")
func detailLevel() {
    #expect(FloatingPanelPresentationPolicy.level(for: .detail) == .floating)
    #expect(FloatingPanelPresentationPolicy.level(for: .detail).rawValue > NSWindow.Level.normal.rawValue)
}
```

同时把原来“两个面板使用相同策略”的测试改为分别应用 `.strip` 和 `.detail`，并断言两个层级不同、collection behavior 相同。

- [ ] **步骤 2：运行定向测试确认失败**

运行：

```bash
swift test --filter FloatingPanelPresentationPolicyTests
```

预期：编译失败，报告 `FloatingPanelPresentationRole` 或 `level(for:)` 不存在。

- [ ] **步骤 3：实现最小角色策略**

```swift
enum FloatingPanelPresentationRole {
    case strip
    case detail
}

static func level(for role: FloatingPanelPresentationRole) -> NSWindow.Level {
    switch role {
    case .strip: desktopLevel
    case .detail: .floating
    }
}
```

将 `apply(to:)` 改为 `apply(to:role:)`，控制器的 `makePanel` 接收角色，分别以 `.strip` 和 `.detail` 创建两个面板。保留 Provider 原有的激活和置前调用。

- [ ] **步骤 4：运行定向测试确认通过**

运行：

```bash
swift test --filter FloatingPanelPresentationPolicyTests
```

预期：角色层级和 Space 行为测试全部通过。

- [ ] **步骤 5：运行完整自动化测试**

运行：

```bash
bash scripts/test.sh
```

预期：所有测试通过且没有既有行为回归。

- [ ] **步骤 6：提交功能节点**

```bash
git add Sources/AIMeterApp/System/FloatingPanelPresentationPolicy.swift Sources/AIMeterApp/System/FloatingPanelController.swift Tests/AIMeterAppTests/FloatingPanelPresentationPolicyTests.swift docs/superpowers
git commit -m "fix: keep detail panels above app windows"
```

## 任务 2：Release 与真实桌面验收

**文件：**
- 创建：`docs/development/2026-09-01-detail-panel-frontmost.md`
- 修改：`README.md`
- 修改：`CHANGELOG.md`
- 修改：`docs/development/README.md`
- 修改：`docs/development/commit-history.md`
- 修改：`docs/requirements-backlog.md`

- [ ] **步骤 1：构建并校验 Release**

运行：

```bash
bash scripts/build-app.sh
codesign --verify --deep --strict build/AI\ Token\ Meter.app
```

预期：Release 构建成功且签名验证通过；Widget 因已记录的开发证书限制继续跳过。

- [ ] **步骤 2：安全安装并启动新版本**

先退出当前应用，把旧 `/Applications/AI Token Meter.app` 移入独立临时备份目录，再复制新 Release。校验安装包签名和可执行文件哈希与构建产物一致后启动。

- [ ] **步骤 3：完成真实窗口栈验收**

在桌面打开一个覆盖详情区域的普通窗口，依次点击 Claude、Codex、DeepSeek 图标。确认三个详情页都位于普通窗口上方；确认 DeepSeek 可输入、Claude/Codex 不抢焦点、点击空白可关闭，浮动条本身仍低于普通窗口。

- [ ] **步骤 4：更新文档和需求状态**

在开发日志写入自动化测试数量、Release 签名、构建/安装哈希和真实桌面验收结果；把 `REQ-20260901-005` 标记为已完成并附 Git 与文档证据，同时同步 README、CHANGELOG 和开发索引。

- [ ] **步骤 5：最终验证并提交文档节点**

再次运行：

```bash
bash scripts/test.sh
git diff --check
git status --short
```

预期：完整测试通过、无空白错误；提交文档后工作区干净。

