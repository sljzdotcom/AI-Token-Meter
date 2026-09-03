# Sparkle 安装状态窗口置前实现计划

> **面向 AI 代理的工作者：** 使用当前会话内联执行；严格遵循测试先行、逐项验证和 Git 检查点。

**目标：** 让用户从 Settings 启动更新后始终看见 Sparkle 的标准安装窗口，并以 `v0.2.2` 完成公开真实升级验收。

**架构：** 新增一个只依赖 AppKit 窗口集合的 presenter，在 Sparkle 标准 UI 启动前隐藏 SwiftUI Settings 并激活应用。更新状态机、EdDSA 信任链和 Sparkle 标准双确认保持不变。

**技术栈：** Swift 6、SwiftUI Settings、AppKit `NSWindow`/`NSApplication`、Sparkle 2.9.4、Swift Testing、GitHub Actions。

---

## 文件结构

- 创建 `Sources/AIMeterApp/SoftwareUpdate/SoftwareUpdateWindowPresenter.swift`：封装 Settings 识别、隐藏和应用激活。
- 创建 `Tests/AIMeterAppTests/SoftwareUpdateWindowPresenterTests.swift`：验证实际 `NSWindow` 的选择与可见性行为。
- 修改 `Sources/AIMeterApp/SoftwareUpdate/SparkleUpdateEngine.swift`：在展示 Sparkle 更新 UI 前调用 presenter。
- 修改 `Tests/AIMeterAppTests/SoftwareUpdatePackagingTests.swift`：锁定集成调用顺序契约。
- 修改版本、Changelog、README、appcast、发布说明和需求/开发日志：形成可追溯 `v0.2.2` 发布。

### 任务 1：窗口让位行为

- [ ] 在 `SoftwareUpdateWindowPresenterTests.swift` 创建一个 Settings 标识窗口、一个普通窗口和一个无标识窗口，先断言调用不存在或 Settings 仍可见，运行 `scripts/test.sh --filter SoftwareUpdateWindowPresenterTests` 并看到预期失败。
- [ ] 实现 `SoftwareUpdateWindowPresenter.prepareForInstallation(windows:activate:)`：仅对标识为 `com_apple_SwiftUI_Settings_window` 的窗口调用 `orderOut(nil)`，然后调用一次 `activate`。
- [ ] 重新运行聚焦测试，确认 Settings 隐藏、其他窗口保持、激活次数为一。
- [ ] 提交测试和最小实现。

### 任务 2：Sparkle 集成顺序

- [ ] 在 `SoftwareUpdatePackagingTests.swift` 增加源码契约，要求 `presentAvailableUpdate()` 在 `controller?.checkForUpdates(nil)` 前调用 `SoftwareUpdateWindowPresenter.prepareForInstallation`；先运行并看到失败。
- [ ] 修改 `SparkleUpdateEngine.swift`，传入 `NSApp.windows` 和 `NSApp.activate(ignoringOtherApps: true)`，不改变其他 Sparkle 委托或失败映射。
- [ ] 运行两个更新聚焦测试组并确认通过。
- [ ] 提交集成节点。

### 任务 3：0.2.2 发布与真实验收

- [ ] 将主应用和 Widget 版本升级为 `0.2.2 (6)`，同步 README、CHANGELOG、发布说明和测试期望。
- [ ] 运行 `scripts/test.sh`、文档检查、公开安全检查和正式更新打包；验证 ZIP、SHA-256、EdDSA 和篡改拒绝。
- [ ] 推送分支并确认公开 CI，通过审查后快进合入 `main`。
- [ ] 创建并推送 `v0.2.2`，发布 ZIP 与 SHA-256，不修改 `v0.2.1`。
- [ ] 从 GitHub 重新下载资产并核验 SHA-256、`0.2.2 (6)` 与严格签名。
- [ ] 启动隔离 `0.2.1`，保持 Settings 打开，执行 `Check for Updates`、`Update Now`、`Install Update`、`Install and Relaunch`；确认状态窗口自动置前、原位升级与自动重启。
- [ ] 对比升级后二进制与正式 Release 哈希；更新需求台账、开发日志和提交历史，重新运行文档/安全门禁并提交收尾证据。
