# AI Meter 首版实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法跟踪进度。

**目标：** 交付一个可在 macOS 14 及以上运行的原生菜单栏 App，本地汇总 Claude、Codex 与 DeepSeek 用量，并提供可关闭的右侧悬浮条、缓存和阈值通知。

**架构：** 使用 Swift Package 管理 `AIMeterCore` 领域库和 `AIMeterApp` SwiftUI 可执行程序。核心层通过统一的 `UsageCollector` 协议隔离 Claude/Codex CLI、DeepSeek HTTP、Keychain、缓存和通知；界面只观察统一的 `UsageSnapshot`。所有可观察行为先写失败测试，再添加最少实现。

**技术栈：** Swift 6.0、SwiftUI、Foundation、Security、UserNotifications、ServiceManagement、Swift Testing/XCTest、macOS 14+。

---

## 文件结构

- `Package.swift`：定义核心库、菜单栏 App、资源和测试目标。
- `Sources/AIMeterCore/Domain/UsageModels.swift`：提供商、指标、状态和统一快照模型。
- `Sources/AIMeterCore/Collectors/UsageCollector.swift`：采集器协议与采集错误。
- `Sources/AIMeterCore/Collectors/ANSITextSanitizer.swift`：清除终端控制字符并归一化文本。
- `Sources/AIMeterCore/Collectors/CommandRunner.swift`：受控命令请求、结果与执行协议。
- `Sources/AIMeterCore/Collectors/PTYCommandRunner.swift`：固定命令的伪终端执行与超时终止。
- `Sources/AIMeterCore/Collectors/ClaudeUsageParser.swift`：解析 Claude `/usage` 输出。
- `Sources/AIMeterCore/Collectors/CodexUsageParser.swift`：解析 Codex `/status` 输出。
- `Sources/AIMeterCore/Collectors/ClaudeCollector.swift`：发现并调用 Claude CLI。
- `Sources/AIMeterCore/Collectors/CodexCollector.swift`：发现并调用 Codex CLI。
- `Sources/AIMeterCore/DeepSeek/DeepSeekClient.swift`：余额 API 请求、响应解析和本地预算指标。
- `Sources/AIMeterCore/Security/KeychainStore.swift`：DeepSeek API Key 安全存取。
- `Sources/AIMeterCore/Persistence/SnapshotCache.swift`：非敏感快照 JSON 缓存。
- `Sources/AIMeterCore/Coordination/RefreshCoordinator.swift`：并发刷新、去重、缓存回退和状态发布。
- `Sources/AIMeterCore/Notifications/ThresholdEvaluator.swift`：70%/90% 阈值、去重和周期重置。
- `Sources/AIMeterApp/AIMeterApp.swift`：应用入口与菜单栏场景。
- `Sources/AIMeterApp/AppModel.swift`：连接核心协调器和 SwiftUI 状态。
- `Sources/AIMeterApp/Views/MenuBarPanel.swift`：菜单栏详情面板。
- `Sources/AIMeterApp/Views/ProviderCard.swift`：统一服务卡片和状态颜色。
- `Sources/AIMeterApp/Views/FloatingStripView.swift`：右侧三圆环悬浮条。
- `Sources/AIMeterApp/Views/SettingsView.swift`：悬浮条、通知、预算和 API Key 设置。
- `Sources/AIMeterApp/System/FloatingPanelController.swift`：多屏幕悬浮窗口定位。
- `Sources/AIMeterApp/System/NotificationService.swift`：macOS 通知授权、发送和点击路由。
- `Sources/AIMeterApp/System/LaunchAtLoginService.swift`：登录启动设置。
- `Sources/AIMeterApp/Resources/Info.plist`：应用标识、名称和最小系统版本。
- `Tests/AIMeterCoreTests/**`：领域、解析器、HTTP、缓存、协调和阈值测试。
- `Tests/AIMeterCoreTests/Fixtures/**`：人工核对的 CLI/API 样本。
- `scripts/build-app.sh`：把 release 二进制和资源封装成 `dist/AI Meter.app`。
- `docs/development/2026-08-28-development-log.md`：命令、红绿测试证据、决策、偏差和提交记录。
- `README.md`：安装、运行、授权、配置和故障排查。

## 任务 1：工程骨架与统一领域模型

**文件：**
- 创建：`Package.swift`
- 创建：`Sources/AIMeterCore/Domain/UsageModels.swift`
- 创建：`Sources/AIMeterCore/Collectors/UsageCollector.swift`
- 创建：`Sources/AIMeterApp/AIMeterApp.swift`
- 创建：`Sources/AIMeterApp/AppModel.swift`
- 创建：`Tests/AIMeterCoreTests/UsageModelsTests.swift`
- 创建：`docs/development/2026-08-28-development-log.md`

- [x] **步骤 1：写领域模型失败测试**

测试用字面值证明 73% 已用量、过期判断和无上限指标不会伪造比例：

```swift
@Test func metricClampsFractionAndSnapshotDetectsStaleData() {
    let metric = UsageMetric(label: "Session", current: 73, limit: 100, unit: .percent)
    #expect(metric.usedFraction == 0.73)

    let snapshot = UsageSnapshot(provider: .claude, primaryMetric: metric,
        fetchedAt: Date(timeIntervalSince1970: 100), staleAfter: 300)
    #expect(snapshot.isStale(at: Date(timeIntervalSince1970: 401)))
}
```

- [x] **步骤 2：运行测试并确认因模块或类型缺失失败**

运行：`swift test --filter UsageModelsTests`

预期：FAIL，提示 `no such module 'AIMeterCore'` 或领域类型不存在。

- [x] **步骤 3：创建最小包结构和领域模型**

`UsageMetric.usedFraction` 仅在 `limit > 0` 时返回 `min(max(current / limit, 0), 1)`；`UsageSnapshot.isStale(at:)` 使用 `fetchedAt + staleAfter` 判断。

- [x] **步骤 4：运行领域测试和完整构建**

运行：`swift test --filter UsageModelsTests && swift build`

预期：测试通过，debug 构建退出码 0。

- [x] **步骤 5：更新开发日志并提交**

记录工具版本、测试红灯/绿灯输出摘要和架构决定。

提交：`git commit -m "feat: scaffold AI Meter domain and app (task 1)"`

## 任务 2：终端文本净化与 Claude/Codex 解析器

**文件：**
- 创建：`Sources/AIMeterCore/Collectors/ANSITextSanitizer.swift`
- 创建：`Sources/AIMeterCore/Collectors/ClaudeUsageParser.swift`
- 创建：`Sources/AIMeterCore/Collectors/CodexUsageParser.swift`
- 创建：`Tests/AIMeterCoreTests/ANSITextSanitizerTests.swift`
- 创建：`Tests/AIMeterCoreTests/ClaudeUsageParserTests.swift`
- 创建：`Tests/AIMeterCoreTests/CodexUsageParserTests.swift`
- 创建：`Tests/AIMeterCoreTests/Fixtures/claude-usage-en.txt`
- 创建：`Tests/AIMeterCoreTests/Fixtures/claude-usage-zh.txt`
- 创建：`Tests/AIMeterCoreTests/Fixtures/codex-status-en.txt`
- 修改：`docs/development/2026-08-28-development-log.md`

- [ ] **步骤 1：写净化器和解析器失败测试**

测试必须抓住 ANSI 残留、`73% used`、`27% remaining` 换算、重置时间缺失和中文百分比行解析错误：

```swift
@Test func parsesRemainingAsUsedFraction() throws {
    let snapshot = try CodexUsageParser().parse("5h limit: 27% remaining\nResets 10:30 PM")
    #expect(snapshot.primaryMetric?.usedFraction == 0.73)
}
```

- [ ] **步骤 2：分别运行测试并确认功能缺失导致失败**

运行：`swift test --filter ANSITextSanitizerTests; swift test --filter ClaudeUsageParserTests; swift test --filter CodexUsageParserTests`

预期：三个测试组均 FAIL，原因是净化器或解析器类型缺失。

- [ ] **步骤 3：实现最小净化和面向标签的解析**

解析器先清除 CSI/OSC/回车覆盖，再按行识别百分比、`used/remaining/已使用/剩余` 方向、指标标签和重置描述。不能识别任何指标时返回 `.unrecognizedOutput`，不得返回伪造的 0%。

- [ ] **步骤 4：运行解析器测试**

运行：`swift test --filter ANSITextSanitizerTests && swift test --filter ClaudeUsageParserTests && swift test --filter CodexUsageParserTests`

预期：全部通过。

- [ ] **步骤 5：更新日志并提交**

提交：`git commit -m "feat: parse Claude and Codex usage output (task 2)"`

## 任务 3：受控伪终端执行器与 CLI 采集器

**文件：**
- 创建：`Sources/AIMeterCore/Collectors/CommandRunner.swift`
- 创建：`Sources/AIMeterCore/Collectors/PTYCommandRunner.swift`
- 创建：`Sources/AIMeterCore/Collectors/ExecutableLocator.swift`
- 创建：`Sources/AIMeterCore/Collectors/ClaudeCollector.swift`
- 创建：`Sources/AIMeterCore/Collectors/CodexCollector.swift`
- 创建：`Tests/AIMeterCoreTests/PTYCommandRunnerTests.swift`
- 创建：`Tests/AIMeterCoreTests/CLICollectorTests.swift`
- 创建：`Tests/AIMeterCoreTests/Fixtures/fake-interactive-cli.sh`
- 修改：`docs/development/2026-08-28-development-log.md`

- [ ] **步骤 1：写执行器超时和固定输入失败测试**

测试真实运行测试脚本，验证 `/usage` 被写入、输出被捕获、退出码保留，并验证挂起进程在设定时限后返回 `.timedOut`。

- [ ] **步骤 2：运行并确认失败**

运行：`swift test --filter PTYCommandRunnerTests`

预期：FAIL，提示执行器类型不存在。

- [ ] **步骤 3：实现受控执行器**

执行器只接受结构化 `CommandRequest(executableURL:arguments:inputLines:timeout:)`；使用 `/usr/bin/script -q /dev/null` 分配终端，环境只继承 `PATH`、`HOME`、`LANG`、`TERM`，超时先 `terminate()` 再在短暂宽限后强制终止。日志中不记录原始输出。

- [ ] **步骤 4：写 Claude/Codex 采集器失败测试并实现最小连接**

使用真实 fake CLI 和真实解析器。Claude 固定发送 `/usage` 与退出指令；Codex 固定发送 `/status` 与退出指令；执行文件不存在映射为 `.notInstalled`，未登录文本映射为 `.authenticationRequired`。

- [ ] **步骤 5：运行采集器测试与 CLI 本机只读烟雾测试**

运行：`swift test --filter PTYCommandRunnerTests && swift test --filter CLICollectorTests`

随后运行只读诊断命令验证本机 CLI 版本和可执行路径；若 CLI 输出格式与 fixture 不同，只新增去敏 fixture 和解析测试，不读取登录令牌。

- [ ] **步骤 6：更新日志并提交**

提交：`git commit -m "feat: collect usage through controlled CLI sessions (task 3)"`

## 任务 4：DeepSeek 余额与 Keychain

**文件：**
- 创建：`Sources/AIMeterCore/DeepSeek/DeepSeekClient.swift`
- 创建：`Sources/AIMeterCore/Security/SecretStore.swift`
- 创建：`Sources/AIMeterCore/Security/KeychainStore.swift`
- 创建：`Tests/AIMeterCoreTests/DeepSeekClientTests.swift`
- 创建：`Tests/AIMeterCoreTests/KeychainStoreTests.swift`
- 修改：`docs/development/2026-08-28-development-log.md`

- [ ] **步骤 1：写 HTTP 响应和预算计算失败测试**

使用自定义 `URLProtocol` 运行真实 `URLSession`，完整模拟官方余额 JSON。测试成功、多币种、空余额、401、429、500 和超时映射；100 CNY 月预算、余额变化不能伪装成官方已用量，预算指标必须标注 `.localBudget`。

- [ ] **步骤 2：运行并确认失败**

运行：`swift test --filter DeepSeekClientTests`

预期：FAIL，提示客户端类型不存在。

- [ ] **步骤 3：实现 DeepSeek 客户端**

固定请求 `GET https://api.deepseek.com/user/balance`，使用 `Authorization: Bearer <key>`，只解析官方字段；HTTP 错误映射为去敏错误，响应正文不进入日志。

- [ ] **步骤 4：写 Keychain 失败测试并实现**

以专用测试 service/account 写入临时 Keychain 项目，验证新增、替换、读取和删除。生产服务名固定为 `com.millerpan.AIMeter.deepseek`，API Key 不进入 UserDefaults 或快照。

- [ ] **步骤 5：运行 DeepSeek 与 Keychain 测试**

运行：`swift test --filter DeepSeekClientTests && swift test --filter KeychainStoreTests`

预期：全部通过，测试清理专用 Keychain 数据。

- [ ] **步骤 6：更新日志并提交**

提交：`git commit -m "feat: add DeepSeek balance and secure key storage (task 4)"`

## 任务 5：缓存、并发刷新与阈值判断

**文件：**
- 创建：`Sources/AIMeterCore/Persistence/SnapshotCache.swift`
- 创建：`Sources/AIMeterCore/Coordination/RefreshCoordinator.swift`
- 创建：`Sources/AIMeterCore/Notifications/ThresholdEvaluator.swift`
- 创建：`Tests/AIMeterCoreTests/SnapshotCacheTests.swift`
- 创建：`Tests/AIMeterCoreTests/RefreshCoordinatorTests.swift`
- 创建：`Tests/AIMeterCoreTests/ThresholdEvaluatorTests.swift`
- 修改：`docs/development/2026-08-28-development-log.md`

- [ ] **步骤 1：写缓存失败测试并实现原子 JSON 缓存**

在独立临时目录运行真实文件系统测试，验证编码/读取、损坏文件忽略和过期标记。缓存内容不得包含名为 `apiKey`、`token`、`authorization` 的字段。

- [ ] **步骤 2：写刷新协调失败测试并实现**

以轻量 fake collector 控制成功、失败和延迟，断言三个采集器并发、单项失败不阻塞、重复刷新被合并、失败时使用缓存并标记 stale。协调器对外发布按 `.claude/.codex/.deepSeek` 排序的快照。

- [ ] **步骤 3：写阈值失败测试并实现**

用字面量覆盖 69→70、89→90、65→93、重复刷新、重置时间变化和回落到 10% 以下。一次跨两级只产生 90% 事件。

- [ ] **步骤 4：运行相关测试和完整测试集**

运行：`swift test --filter SnapshotCacheTests && swift test --filter RefreshCoordinatorTests && swift test --filter ThresholdEvaluatorTests && swift test`

预期：全部通过。

- [ ] **步骤 5：更新日志并提交**

提交：`git commit -m "feat: coordinate refresh cache and usage alerts (task 5)"`

## 任务 6：菜单栏界面、悬浮条和系统服务

**文件：**
- 修改：`Sources/AIMeterApp/AIMeterApp.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 创建：`Sources/AIMeterApp/Views/MenuBarPanel.swift`
- 创建：`Sources/AIMeterApp/Views/ProviderCard.swift`
- 创建：`Sources/AIMeterApp/Views/UsageRing.swift`
- 创建：`Sources/AIMeterApp/Views/FloatingStripView.swift`
- 创建：`Sources/AIMeterApp/Views/SettingsView.swift`
- 创建：`Sources/AIMeterApp/System/FloatingPanelController.swift`
- 创建：`Sources/AIMeterApp/System/NotificationService.swift`
- 创建：`Sources/AIMeterApp/System/LaunchAtLoginService.swift`
- 创建：`Tests/AIMeterCoreTests/AppPresentationTests.swift`
- 修改：`docs/development/2026-08-28-development-log.md`

- [ ] **步骤 1：写展示模型失败测试**

测试每种状态的标题、辅助说明、颜色语义和菜单栏最高风险汇总。测试只断言展示模型行为，不断言 SwiftUI 内部层级。

- [ ] **步骤 2：运行并确认失败**

运行：`swift test --filter AppPresentationTests`

预期：FAIL，提示展示模型不存在。

- [ ] **步骤 3：实现菜单栏和统一服务卡片**

使用 `MenuBarExtra` 展示三项服务、刷新按钮、最后更新时间和设置入口；错误状态显示可行动文案，不显示原始错误或凭证。

- [ ] **步骤 4：实现右侧悬浮条**

使用无标题透明 `NSPanel`，固定在可见屏幕右侧，支持多屏变化；视觉采用黑色有机胶囊、三个细圆环和点击展开卡片。设置关闭时销毁面板但不停止菜单栏刷新。

- [ ] **步骤 5：实现通知、登录启动与设置**

通知服务消费 `ThresholdEvent`；设置通过 `AppStorage` 保存非敏感选项，API Key 通过 `SecretStore`；登录启动使用 `SMAppService.mainApp` 并把系统错误显示为非阻塞状态。

- [ ] **步骤 6：运行测试和构建**

运行：`swift test && swift build`

预期：全部通过且无 Swift 编译错误。

- [ ] **步骤 7：更新日志并提交**

提交：`git commit -m "feat: add menu bar floating meter and settings (task 6)"`

## 任务 7：App 打包、验收脚本与使用文档

**文件：**
- 创建：`Sources/AIMeterApp/Resources/Info.plist`
- 创建：`scripts/build-app.sh`
- 创建：`Tests/AIMeterCoreTests/PrivacyRegressionTests.swift`
- 创建：`README.md`
- 修改：`docs/development/2026-08-28-development-log.md`

- [ ] **步骤 1：写隐私回归失败测试**

以实际快照、缓存和去敏错误生成产物，断言已知测试密钥与授权头不会出现在缓存、展示文本或错误描述中。

- [ ] **步骤 2：运行并确认失败，再实现缺失的去敏边界**

运行：`swift test --filter PrivacyRegressionTests`

预期：首次因缺少隐私边界或测试目标失败；添加最少实现后通过。

- [ ] **步骤 3：创建可重复 App 打包脚本**

脚本执行 release 构建、建立 `dist/AI Meter.app/Contents/{MacOS,Resources}`、复制二进制和 `Info.plist`，然后使用 `codesign --sign - --force --deep` 做本机临时签名。脚本使用 `set -euo pipefail`，不得打印环境变量或凭证。

- [ ] **步骤 4：编写 README 和故障排查**

明确 Claude/Codex CLI 登录前置条件、DeepSeek Keychain 配置、5 分钟刷新、通知权限、登录启动、卸载和日志位置。

- [ ] **步骤 5：执行完整验收**

运行：

```bash
swift test
swift build -c release
bash scripts/build-app.sh
codesign --verify --deep --strict "dist/AI Meter.app"
```

预期：所有命令退出码 0，测试 0 失败，App 包签名验证成功。

- [ ] **步骤 6：人工本机烟雾验证**

启动 App，检查菜单栏出现、悬浮条可开关、设置可保存、手动刷新不会冻结 UI。未配置 DeepSeek Key 时显示“需要配置”，不弹出敏感数据。

- [ ] **步骤 7：更新日志并提交**

提交：`git commit -m "release: package and document AI Meter v0.1.0 (task 7)"`

## 完成审查

- [ ] 对照设计规格逐项标记已实现、部分实现或未实现。
- [ ] 扫描仓库和 Git diff，确认不存在测试 API Key、Bearer 头、Claude/Codex 登录令牌和原始用量输出。
- [ ] 重新运行完整测试、release 构建、App 打包与签名验证。
- [ ] 检查 `git status --short` 为空并记录最终提交列表。
- [ ] 使用 `finishing-a-development-branch` 技能完成分支收尾。
