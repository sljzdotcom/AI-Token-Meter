# DeepSeek 登录输入焦点修复实现计划

> **面向 AI 代理的工作者：** 按任务顺序逐项执行；每个行为变更先写失败测试，再实现最小修复。完成前必须运行完整测试、构建、签名与本机界面验收。

**目标：** 让 AI Meter 内嵌的 DeepSeek 官方登录页稳定接收手机号与验证码输入，同时保持 Claude、Codex 详情不抢焦点。

**架构：** 在核心层用纯策略描述各提供商是否激活应用、是否请求网页焦点；在 AppKit 层用专用 `NSPanel` 子类实现可成为 Key Window 的交互式详情面板；`FloatingPanelController` 只执行策略并在详情关闭或切换时清理响应链。

**技术栈：** Swift 6、SwiftUI、AppKit、WebKit、Swift Testing、Swift Package Manager

---

## 任务 1：建立可测试的提供商交互策略（已完成）

**文件：**

- 新建：`Sources/AIMeterCore/UI/FloatingDetailInteractionPolicy.swift`
- 新建：`Tests/AIMeterCoreTests/FloatingDetailInteractionPolicyTests.swift`

- [x] **步骤 1：编写失败测试**

  新增 `FloatingDetailInteractionPolicyTests`，验证：

  - DeepSeek 的 `activatesApplication` 与 `requestsWebFirstResponder` 都为 `true`；
  - Claude、Codex 的两个属性都为 `false`。

- [x] **步骤 2：运行定向测试并确认失败**

  运行：`bash scripts/test.sh --filter FloatingDetailInteractionPolicyTests`

  预期：因为 `FloatingDetailInteractionPolicy` 尚不存在而编译失败。

- [x] **步骤 3：实现最小纯策略**

  新增 `public struct FloatingDetailInteractionPolicy: Equatable, Sendable`，初始化参数为 `UsageProvider`。只有 `.deepSeek` 返回交互式行为，其他提供商返回只读行为。

- [x] **步骤 4：运行定向测试并确认通过**

  运行：`bash scripts/test.sh --filter FloatingDetailInteractionPolicyTests`

  预期：新增测试全部通过。

- [x] **步骤 5：运行完整测试**

  运行：`bash scripts/test.sh`

  预期：现有 100 个测试加新增策略测试全部通过，环境依赖测试只允许保持既有跳过状态。

- [x] **步骤 6：保存版本**

  提交：`feat: define floating detail focus policy`

## 任务 2：让无边框详情面板具备键盘交互能力（已完成）

**文件：**

- 修改：`Package.swift`
- 新建：`Sources/AIMeterApp/System/InteractivePanel.swift`
- 新建：`Tests/AIMeterAppTests/InteractivePanelTests.swift`
- 修改：`Sources/AIMeterApp/System/FloatingPanelController.swift`

- [x] **步骤 1：为 AppKit 面板能力编写失败测试**

  在 `Package.swift` 增加依赖 `AIMeterApp` 的 `AIMeterAppTests` 测试目标。测试用 `.borderless` 创建 `InteractivePanel`，断言 `canBecomeKey == true` 且 `canBecomeMain == false`。

- [x] **步骤 2：运行定向测试并确认失败**

  运行：`bash scripts/test.sh --filter InteractivePanelTests`

  预期：因为 `InteractivePanel` 尚不存在而编译失败。

- [x] **步骤 3：实现专用面板类型**

  新增 `@MainActor final class InteractivePanel: NSPanel`：

  - 覆盖 `canBecomeKey`，固定返回 `true`；
  - 覆盖 `canBecomeMain`，固定返回 `false`。

- [x] **步骤 4：运行面板能力测试并确认通过**

  运行：`bash scripts/test.sh --filter InteractivePanelTests`

  预期：面板能力测试通过。

- [x] **步骤 5：接入详情控制器**

  修改 `FloatingPanelController`：

  - 悬浮条继续使用带 `.nonactivatingPanel` 的普通 `NSPanel`；
  - 详情使用 `InteractivePanel`，并设 `becomesKeyOnlyIfNeeded = false`；
  - DeepSeek 根据 `FloatingDetailInteractionPolicy` 激活 AI Meter、令详情成为 Key Window；
  - SwiftUI 宿主安装并进入下一轮主线程调度后，在当前选择仍为 DeepSeek且 WebView 仍属于详情窗口时，调用 `makeFirstResponder(webView)`；
  - Claude/Codex 只调用 `orderFrontRegardless`，不激活 App，不设置网页 First Responder；
  - 关闭详情或切换提供商之前调用 `makeFirstResponder(nil)`，避免已移除 WebView 留在响应链。

- [x] **步骤 6：运行定向与完整测试**

  运行：

  - `bash scripts/test.sh --filter FloatingDetailInteractionPolicyTests`
  - `bash scripts/test.sh --filter InteractivePanelTests`
  - `bash scripts/test.sh`

  预期：所有新增与既有测试通过。

- [x] **步骤 7：保存版本**

  提交：`fix: allow DeepSeek detail web input focus`

## 任务 3：补齐文档并完成静态验证（已完成）

**文件：**

- 修改：`CHANGELOG.md`
- 修改：`docs/user-guide/troubleshooting.md`
- 修改：`docs/architecture/overview.md`
- 新建：`docs/development/2026-08-31-deepseek-focus.md`
- 修改：`docs/development/README.md`

- [x] **步骤 1：记录用户行为与排障边界**

  文档说明：DeepSeek 详情会短暂激活 AI Meter 以接收网页输入；Claude/Codex 保持只读无抢焦点；出现 CAPTCHA 或额外安全确认时需用户处理。不得记录手机号或验证码。

- [x] **步骤 2：运行完整自动化验证**

  运行：

  - `bash scripts/test.sh`
  - `bash scripts/build-app.sh`
  - `plutil -lint "dist/AI Meter.app/Contents/Info.plist"`
  - `codesign --verify --deep --strict --verbose=2 "dist/AI Meter.app"`
  - `file "dist/AI Meter.app/Contents/MacOS/AIMeterApp"`
  - `git diff --check`

  预期：测试与构建退出码为 0；Info.plist 有效；严格签名通过；可执行文件为 Apple Silicon 原生 Mach-O；没有空白错误。

- [x] **步骤 3：保存版本**

  提交：`docs: document DeepSeek web focus repair`

## 任务 4：安装并完成真实登录验收

**文件：**

- 修改：`docs/development/2026-08-31-deepseek-focus.md`（仅记录非敏感验收结果）

- [ ] **步骤 1：备份当前安装版**

  退出 AI Meter，把 `/Applications/AI Meter.app` 复制到带时间戳的 `/private/tmp/AI Meter.app.pre-deepseek-focus-*`。备份路径写入本次运行记录，但不写入项目文档。

- [ ] **步骤 2：安装并校验构建产物**

  把 `dist/AI Meter.app` 安装到 `/Applications/AI Meter.app`，比较两处主可执行文件的 SHA-256，确认完全一致后启动。

- [ ] **步骤 3：验证输入焦点**

  点击 DeepSeek 圆环，确认详情窗口保持显示；点击手机号输入框，确认出现插入光标并能接受数字输入。

- [ ] **步骤 4：完成已授权的验证码登录**

  把用户已授权的手机号只输入 DeepSeek 官方页面，点击“获取验证码”；在“信息”中只读取最新且来源明确的 DeepSeek 验证码，填入同一官方页面并提交。若出现 CAPTCHA、法律协议、额外安全确认或验证码无法可靠区分，立即停止自动操作并交由用户确认。

- [ ] **步骤 5：验证登录后数据与回归行为**

  确认最近 30 天用量页能够加载或明确显示缓存状态；确认 Claude、Codex 详情不抢焦点；确认空白点击关闭、自动隐藏、悬停暂停与退出清理仍正常。

- [ ] **步骤 6：记录验收并保存版本**

  开发日志只记录焦点、登录和回归测试结论，不记录手机号、验证码、Cookie 或页面原始数据。提交：`test: record DeepSeek login acceptance`

## 任务 5：合并主分支并验证最终状态

- [ ] **步骤 1：检查分支状态与提交历史**

  运行 `git status --short --branch` 与 `git log --oneline --decorate -8`，确认工作树干净且关键节点均已提交。

- [ ] **步骤 2：合并到 `main`**

  在主工作区以非快进方式合并 `codex/deepseek-web-focus`。若主分支出现新提交，先审查差异并在功能分支合并 `main`、重新验证后再合入。

- [ ] **步骤 3：在 `main` 上复验**

  运行完整测试、构建、严格签名验证与 `git diff --check`。确认 `/Applications/AI Meter.app` 的主可执行文件与 `main` 最新构建一致。

- [ ] **步骤 4：报告结果**

  向用户报告：修复内容、自动化测试数量、真实登录结果、安装状态、关键提交与可恢复备份路径；不得包含手机号或验证码。
