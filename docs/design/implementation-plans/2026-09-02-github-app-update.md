# AI Token Meter GitHub 应用内更新实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 在 Settings → About 中提供由用户主动触发的检查更新与安全安装能力，并通过 GitHub Release、Sparkle appcast 和 EdDSA 签名发布首个更新器版本 `0.2.0 (4)`。

**架构：** 应用层新增纯状态模型、可替换更新引擎和主线程 `SoftwareUpdateCoordinator`；生产引擎固定使用 Sparkle `2.9.4` 的标准安装 UI，Settings 负责展示状态和触发两个动作。构建脚本完整嵌入并验证 Sparkle framework/helper，发布脚本只使用 Keychain 中的生产私钥生成签名 ZIP 与 appcast，测试使用独立夹具且不接触生产密钥。

**技术栈：** Swift 6、SwiftUI、Observation、Sparkle 2.9.4、SwiftPM binary target、Swift Testing、POSIX shell、GitHub Releases、Sparkle appcast、EdDSA、SHA-256。

---

## 文件结构

### 新建

- `Sources/AIMeterApp/SoftwareUpdate/SoftwareUpdateState.swift`：更新版本摘要、阶段、按钮规则和用户可见文案的纯状态模型。
- `Sources/AIMeterApp/SoftwareUpdate/SoftwareUpdateEngine.swift`：应用与第三方更新框架之间的窄协议及事件类型。
- `Sources/AIMeterApp/SoftwareUpdate/SoftwareUpdateCoordinator.swift`：串行化检查/安装动作，将引擎事件归一化为 Observable 状态。
- `Sources/AIMeterApp/SoftwareUpdate/SparkleUpdateEngine.swift`：Sparkle 2.9.4 的唯一适配层。
- `Sources/AIMeterApp/Views/SoftwareUpdateSettingsView.swift`：About 内的软件更新卡片。
- `Tests/AIMeterAppTests/SoftwareUpdateStateTests.swift`：状态、文案和按钮规则测试。
- `Tests/AIMeterAppTests/SoftwareUpdateCoordinatorTests.swift`：检查、发现、无更新、失败、重复点击和安装转发测试。
- `Tests/AIMeterAppTests/SoftwareUpdatePackagingTests.swift`：依赖锁定、Info.plist、framework/helper、签名和发布脚本合同测试。
- `scripts/verify-update-bundle.sh`：验证构建 App 的 Sparkle 嵌入、配置和嵌套签名。
- `scripts/package-update-release.sh`：构建、ZIP、SHA-256、EdDSA 签名和 appcast 生成的失败即停止入口。
- `appcast.xml`：正式稳定更新 feed；首个条目指向 GitHub `v0.2.0` arm64 ZIP。
- `docs/development/2026-09-02-github-app-update.md`：实现、密钥边界、验证和发布证据。

### 修改

- `Package.swift`：以官方 Release URL 和官方 SHA-256 直接声明 Sparkle `2.9.4` binary target，只让主 App target 依赖它；避免 Git 协议不可用时无法解析仅含二进制产物的上游包。
- `Sources/AIMeterApp/AIMeterApp.swift`、`AppDelegate.swift`：创建并向 Settings 注入唯一 coordinator；应用终止时结束更新状态。
- `Sources/AIMeterApp/Views/SettingsView.swift`、`AboutSettingsView.swift`：接入 Software Update 卡片。
- `Sources/AIMeterApp/Resources/Info.plist`：升级 `0.2.0 (4)`，写入 feed、公钥并明确禁用自动检查/自动安装。
- `scripts/build-app.sh`：把完整 Sparkle framework 放入 `Contents/Frameworks`，签名顺序从最内层 helper 到主 App。
- `scripts/verify-app-resources.sh`、`scripts/check-public-release.sh`、`scripts/check-docs.sh`：增加更新框架、密钥泄露和版本/feed 文档门禁。
- `README.md`、`CHANGELOG.md`、`docs/README.md`、`docs/project-status.md`、`docs/user-guide/settings.md`、`docs/user-guide/getting-started.md`、`docs/user-guide/troubleshooting.md`、`docs/architecture/overview.md`、`docs/architecture/decisions.md`、`docs/architecture/repository-structure.md`、`docs/security-and-privacy.md`、`docs/development/testing.md`、`docs/development/release-process.md`、`docs/development/maintenance-playbook.md`、`docs/development/commit-history.md`、`docs/development/README.md`、`docs/requirements-backlog.md`：同步功能、首次手动升级、发布与验收事实。

## 任务 1：锁定 Sparkle 并建立纯更新状态模型

**文件：**
- 修改：`Package.swift`
- 创建：`Sources/AIMeterApp/SoftwareUpdate/SoftwareUpdateState.swift`
- 创建：`Tests/AIMeterAppTests/SoftwareUpdateStateTests.swift`

- [ ] **步骤 1：编写失败的状态测试**

覆盖初始、检查中、最新、发现新版、交给安装器和失败状态，以及两个按钮的启用规则：

```swift
@Test("Only an available release enables Update Now")
func updateNowAvailability() {
    let release = SoftwareUpdateRelease(
        version: "0.2.1",
        build: "5",
        publishedAt: nil,
        summary: "Reliability fixes"
    )
    #expect(!SoftwareUpdateState.idle.canInstall)
    #expect(SoftwareUpdateState.available(release).canInstall)
    #expect(!SoftwareUpdateState.installing(release).canInstall)
}

@Test("Busy phases reject another check")
func busyCheckPolicy() {
    #expect(!SoftwareUpdateState.checking.canCheck)
    #expect(SoftwareUpdateState.upToDate.canCheck)
}
```

- [ ] **步骤 2：运行测试确认失败**

运行：`bash scripts/test.sh --filter SoftwareUpdateStateTests`

预期：编译失败，提示 `SoftwareUpdateState` 和 `SoftwareUpdateRelease` 尚未定义。

- [ ] **步骤 3：实现最小纯状态模型**

实现统一且可测试的值类型：

```swift
struct SoftwareUpdateRelease: Equatable, Sendable {
    let version: String
    let build: String
    let publishedAt: Date?
    let summary: String?
}

enum SoftwareUpdateState: Equatable, Sendable {
    case idle
    case checking
    case upToDate
    case available(SoftwareUpdateRelease)
    case installing(SoftwareUpdateRelease)
    case failed(String)

    var canCheck: Bool {
        switch self {
        case .checking, .installing: false
        default: true
        }
    }

    var canInstall: Bool {
        if case .available = self { true } else { false }
    }
}
```

状态文件同时提供 `statusText`、`availableRelease` 和经过长度限制的纯文本摘要，不在 View 中复制 switch。

- [ ] **步骤 4：精确锁定 Sparkle 2.9.4**

在 `Package.swift` 增加官方二进制 target：

```swift
.binaryTarget(
    name: "Sparkle",
    url: "https://github.com/sparkle-project/Sparkle/releases/download/2.9.4/Sparkle-for-Swift-Package-Manager.zip",
    checksum: "cb6fdbdc8884f15d62a616e79face92b08322410fd2d425edc6596ccbf4ba3b0"
),
```

并仅在 `AIMeterApp` 增加：

```swift
"Sparkle"
```

SwiftPM 必须验证下载产物与固定 SHA-256 一致；不使用 `branch`、范围依赖或仓库内 vendored framework。官方 Git 仓库在当前主机发生持续 TLS 握手失败，因此选用同一 `2.9.4` Release 的直接 binary target；此变化不改变版本、来源或安全边界。

- [ ] **步骤 5：运行状态测试和完整编译**

运行：`bash scripts/test.sh --filter SoftwareUpdateState`

预期：状态测试通过；AIMeterCore 和 Widget 不依赖 Sparkle。

- [ ] **步骤 6：提交检查点**

```bash
git add Package.swift Sources/AIMeterApp/SoftwareUpdate Tests/AIMeterAppTests/SoftwareUpdateStateTests.swift docs/design/implementation-plans/2026-09-02-github-app-update.md
git commit -m "feat: add software update state model"
```

## 任务 2：实现可测试 coordinator 与 Sparkle 适配层

**文件：**
- 创建：`Sources/AIMeterApp/SoftwareUpdate/SoftwareUpdateEngine.swift`
- 创建：`Sources/AIMeterApp/SoftwareUpdate/SoftwareUpdateCoordinator.swift`
- 创建：`Sources/AIMeterApp/SoftwareUpdate/SparkleUpdateEngine.swift`
- 创建：`Tests/AIMeterAppTests/SoftwareUpdateCoordinatorTests.swift`

- [ ] **步骤 1：编写失败的 coordinator 测试与 Fake engine**

```swift
@MainActor
private final class FakeSoftwareUpdateEngine: SoftwareUpdateEngine {
    var eventHandler: ((SoftwareUpdateEvent) -> Void)?
    var canCheckForUpdates = true
    private(set) var informationChecks = 0
    private(set) var presentedChecks = 0

    func start() throws {}
    func checkForUpdateInformation() { informationChecks += 1 }
    func presentAvailableUpdate() { presentedChecks += 1 }
}

@Test("Manual check records availability and Update Now delegates to the installer")
@MainActor
func manualUpdateFlow() throws {
    let engine = FakeSoftwareUpdateEngine()
    let coordinator = try SoftwareUpdateCoordinator(engine: engine, now: { Date(timeIntervalSince1970: 10) })
    coordinator.checkForUpdates()
    #expect(engine.informationChecks == 1)
    #expect(coordinator.state == .checking)

    let release = SoftwareUpdateRelease(version: "0.2.1", build: "5", publishedAt: nil, summary: nil)
    engine.eventHandler?(.found(release))
    coordinator.installAvailableUpdate()
    #expect(engine.presentedChecks == 1)
    #expect(coordinator.state == .installing(release))
}
```

另测：重复检查被忽略、无更新、离线/超时/签名错误使用脱敏文案、无可用版本时安装不转发、`stop()` 后事件不回写。

- [ ] **步骤 2：运行测试确认失败**

运行：`bash scripts/test.sh --filter SoftwareUpdateCoordinator`

预期：编译失败，协议、事件和 coordinator 尚未定义。

- [ ] **步骤 3：实现引擎协议和 coordinator**

协议只暴露应用需要的能力：

```swift
@MainActor
protocol SoftwareUpdateEngine: AnyObject {
    var eventHandler: ((SoftwareUpdateEvent) -> Void)? { get set }
    var canCheckForUpdates: Bool { get }
    func start() throws
    func checkForUpdateInformation()
    func presentAvailableUpdate()
}
```

`SoftwareUpdateCoordinator` 使用 `@MainActor @Observable`，初始化时绑定 handler 并启动引擎；`checkForUpdates()` 在 `state.canCheck && engine.canCheckForUpdates` 时切换到 `.checking`；终态统一写入 `lastCheckedAt`；错误仅通过固定分类映射显示，不直接把 URL、请求头或框架调试文本放入 UI。

- [ ] **步骤 4：实现 Sparkle 2.9.4 适配器**

`SparkleUpdateEngine`：

- 延迟创建并强持有 `SPUStandardUpdaterController(startingUpdater: true, updaterDelegate: self, userDriverDelegate: nil)`；
- `checkForUpdateInformation()` 调用 `controller.updater.checkForUpdateInformation()`，不显示 Sparkle 检查窗口；
- `presentAvailableUpdate()` 调用 `controller.checkForUpdates(nil)`，使用 Sparkle 标准、可聚焦的安装 UI；
- `updater(_:didFindValidUpdate:)` 映射 `displayVersionString`、`versionString`、`date` 和安全摘要；
- `updaterDidNotFindUpdate(_:error:)` 发出 `.noUpdate`；
- `updater(_:didAbortWithError:)` 发出分类错误；
- `willDownloadUpdate`、`didDownloadUpdate`、`willExtractUpdate`、`willInstallUpdate` 更新阶段；
- 不实现自定义版本比较器、不覆盖 feed、不接受 beta channel、不调用 Sparkle 私有 API。

- [ ] **步骤 5：运行 coordinator 测试和全量测试**

运行：

```bash
bash scripts/test.sh --filter SoftwareUpdate
bash scripts/test.sh
```

预期：新增测试通过；既有 331 项测试无回归。

- [ ] **步骤 6：提交检查点**

```bash
git add Sources/AIMeterApp/SoftwareUpdate Tests/AIMeterAppTests/SoftwareUpdateCoordinatorTests.swift
git commit -m "feat: bridge Sparkle update checks"
```

## 任务 3：接入 App 生命周期和 About 双按钮界面

**文件：**
- 修改：`Sources/AIMeterApp/AIMeterApp.swift`
- 修改：`Sources/AIMeterApp/AppDelegate.swift`
- 修改：`Sources/AIMeterApp/Views/SettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/AboutSettingsView.swift`
- 创建：`Sources/AIMeterApp/Views/SoftwareUpdateSettingsView.swift`
- 修改：`Tests/AIMeterAppTests/SettingsStructureTests.swift`

- [ ] **步骤 1：编写失败的 Settings 合同和状态展示测试**

测试 `AboutSettingsView` 必须接收 coordinator；Software Update 视图同时存在 `Check for Updates` 与 `Update Now`，后者只由 `state.canInstall` 启用。测试纯展示映射：

```swift
@Test("About update controls expose both user initiated actions")
func updateControlLabels() {
    #expect(SoftwareUpdateSettingsCopy.checkButton == "Check for Updates")
    #expect(SoftwareUpdateSettingsCopy.installButton == "Update Now")
}
```

- [ ] **步骤 2：运行测试确认失败**

运行：`bash scripts/test.sh --filter Settings`

预期：失败，更新卡片/文案尚未定义。

- [ ] **步骤 3：实现 Software Update 卡片**

卡片显示：

```swift
Section("Software Update") {
    LabeledContent("Current version", value: coordinator.currentVersionText)
    LabeledContent("Status", value: coordinator.state.statusText)
    if let lastCheckedAt = coordinator.lastCheckedAt {
        LabeledContent("Last checked", value: lastCheckedAt.formatted(date: .abbreviated, time: .shortened))
    }
    if let release = coordinator.state.availableRelease {
        UpdateReleaseSummaryView(release: release)
    }
    HStack {
        Button("Check for Updates") { coordinator.checkForUpdates() }
            .disabled(!coordinator.canCheck)
        Button("Update Now") { coordinator.installAvailableUpdate() }
            .buttonStyle(.borderedProminent)
            .disabled(!coordinator.canInstall)
    }
}
```

Settings 继续使用 `.aiMeterFontScope(.settings)`，不受用户选择字体影响。卡片使用系统控件和 Dynamic Type，不引入新的视觉主题选择。

- [ ] **步骤 4：接入唯一生命周期实例**

`AppDelegate` 增加 `let softwareUpdateCoordinator`，生产初始化使用 `SparkleUpdateEngine`；`AIMeterApp` 把它传给 `SettingsView`；后者再传给 About。`applicationWillTerminate` 调用 `stop()`，不在 `AppModel` 的五分钟 Provider 刷新循环里检查更新。

- [ ] **步骤 5：运行 Settings 测试与完整测试**

运行：

```bash
bash scripts/test.sh --filter Settings
bash scripts/test.sh
```

预期：两个按钮行为、系统字体隔离、现有四标签顺序和全部回归通过。

- [ ] **步骤 6：提交检查点**

```bash
git add Sources/AIMeterApp/AIMeterApp.swift Sources/AIMeterApp/AppDelegate.swift Sources/AIMeterApp/Views Tests/AIMeterAppTests/SettingsStructureTests.swift
git commit -m "feat: add update controls to settings"
```

## 任务 4：把 Sparkle 完整嵌入 Release App 并加构建门禁

**文件：**
- 修改：`Sources/AIMeterApp/Resources/Info.plist`
- 修改：`scripts/build-app.sh`
- 修改：`scripts/verify-app-resources.sh`
- 创建：`scripts/verify-update-bundle.sh`
- 创建：`Tests/AIMeterAppTests/SoftwareUpdatePackagingTests.swift`
- 修改：`Tests/AIMeterAppTests/AppBundleMetadataTests.swift`

- [ ] **步骤 1：编写失败的元数据与构建脚本测试**

断言：

```swift
#expect(plist["SUFeedURL"] as? String == "https://raw.githubusercontent.com/sljzdotcom/AI-Token-Meter/main/appcast.xml")
#expect((plist["SUPublicEDKey"] as? String)?.isEmpty == false)
#expect(plist["SUEnableAutomaticChecks"] as? Bool == false)
#expect(plist["SUAutomaticallyUpdate"] as? Bool == false)
#expect(plist["CFBundleShortVersionString"] as? String == "0.2.0")
#expect(plist["CFBundleVersion"] as? String == "4")
```

脚本合同测试断言 `Contents/Frameworks/Sparkle.framework`、Updater helper 和 XPC services 是硬失败检查项，且在复制 framework 之后、签主 App 之前完成嵌套签名。

- [ ] **步骤 2：运行测试确认失败**

运行：`bash scripts/test.sh --filter 'Software update packaging|bundle metadata'`

预期：缺少 Sparkle 元数据、验证脚本和 build `4`。

- [ ] **步骤 3：生成并登记生产更新公钥**

从 Sparkle `2.9.4` 官方 Release 获取 `generate_keys`，核对 Release 来源与校验后运行一次。工具把私钥保存到登录 Keychain，只把输出的公钥写入 `SUPublicEDKey`。随后运行公开发布扫描，确认工作树和 Git 历史没有私钥格式或导出文件。

- [ ] **步骤 4：更新 Info.plist**

写入 `0.2.0`、build `4`、固定 `SUFeedURL`、真实 `SUPublicEDKey`、`SUEnableAutomaticChecks=false`、`SUAutomaticallyUpdate=false`。不加入用户可编辑 feed，不加入 GitHub Token。

- [ ] **步骤 5：扩展构建与签名顺序**

`copy_main_app` 创建 `Contents/Frameworks`，从 SwiftPM binary artifact 的实际产物路径完整复制 `Sparkle.framework`。签名按 Sparkle 官方要求由最内层 helper/XPC 到 framework、可选 Widget、最后主 App；ad-hoc 和 Apple Development 两条路径都走相同嵌套顺序，不再依赖 `codesign --deep` 替代显式嵌套签名。

- [ ] **步骤 6：实现独立更新 bundle 验证器**

`scripts/verify-update-bundle.sh` 检查：

```bash
test -d "$APP/Contents/Frameworks/Sparkle.framework"
plutil -extract SUFeedURL raw "$APP/Contents/Info.plist"
plutil -extract SUPublicEDKey raw "$APP/Contents/Info.plist"
codesign --verify --strict --verbose=2 "$APP/Contents/Frameworks/Sparkle.framework"
codesign --verify --deep --strict --verbose=2 "$APP"
otool -L "$APP/Contents/MacOS/AIMeterApp"
```

还要验证主二进制通过 `@rpath/Sparkle.framework` 链接、framework 内关键 helper 存在且没有指向 worktree/`.build` 的绝对依赖。

- [ ] **步骤 7：构建并运行验证**

运行：

```bash
AI_METER_INCLUDE_WIDGET=0 bash scripts/build-app.sh
scripts/verify-app-resources.sh "dist/AI Token Meter.app"
scripts/verify-update-bundle.sh "dist/AI Token Meter.app"
```

预期：arm64 Release、资源、Sparkle framework/helper、Info.plist 和严格签名全部通过；从临时目录启动至少 10 秒不发生 dyld 或 `NSBundle.module` 崩溃。

- [ ] **步骤 8：提交检查点**

```bash
git add Sources/AIMeterApp/Resources/Info.plist scripts Tests/AIMeterAppTests
git commit -m "build: embed and verify Sparkle"
```

## 任务 5：建立签名 Release 与 appcast 发布工具

**文件：**
- 创建：`scripts/package-update-release.sh`
- 创建：`appcast.xml`
- 修改：`scripts/check-public-release.sh`
- 修改：`Tests/AIMeterAppTests/PublicReleaseSafetyScriptTests.swift`
- 修改：`Tests/AIMeterAppTests/SoftwareUpdatePackagingTests.swift`

- [ ] **步骤 1：编写失败的发布脚本合同测试**

测试脚本在以下情况必须非零退出：版本与 Info.plist 不一致、build 未递增、工作树不符合发布规则、Sparkle 工具缺失、Keychain 私钥缺失、ZIP 未生成、签名为空或 appcast 不包含预期 URL/版本/build/长度/EdDSA。

另测 `check-public-release.sh` 会扫描 `.pem`、`.key`、Sparkle 私钥导出与已知 secret 标记，但不会把命中内容打印到日志。

- [ ] **步骤 2：运行测试确认失败**

运行：`bash scripts/test.sh --filter 'Software update packaging|Public release safety'`

预期：新合同失败，发布脚本和 appcast 尚不存在。

- [ ] **步骤 3：实现失败即停止的发布脚本**

接口固定为：

```bash
SPARKLE_TOOLS_DIR=/absolute/path/to/Sparkle/bin \
scripts/package-update-release.sh 0.2.0 4
```

脚本执行：全量测试、安全扫描、无 Widget Release 构建、bundle 验证、`ditto -c -k --keepParent` ZIP、`shasum -a 256`、`sign_update`、`generate_appcast --download-url-prefix https://github.com/sljzdotcom/AI-Token-Meter/releases/download/v0.2.0/`。输出固定到 `dist/releases/0.2.0/`；不会打印环境、Keychain 内容或私钥。

- [ ] **步骤 4：生成并验证首个 appcast 条目**

条目必须包含：

```xml
<enclosure
  url="https://github.com/sljzdotcom/AI-Token-Meter/releases/download/v0.2.0/AI-Token-Meter-0.2.0-macOS-arm64.zip"
  sparkle:version="4"
  sparkle:shortVersionString="0.2.0"
  length="实际 ZIP 字节数"
  type="application/octet-stream"
  sparkle:edSignature="由 sign_update 生成的签名" />
```

不手填签名或长度；只接受生成工具产物。用 Sparkle `generate_appcast`/验证器重新解析，确认默认 channel、macOS 14 和 arm64 约束。

- [ ] **步骤 5：运行发布脚本测试与公开安全扫描**

运行：

```bash
bash scripts/test.sh --filter 'Software update packaging|Public release safety'
scripts/check-public-release.sh "dist/releases/0.2.0/AI-Token-Meter-0.2.0-macOS-arm64.zip"
```

预期：合同通过；仓库、完整历史和 ZIP 不含生产私钥或其他凭证。

- [ ] **步骤 6：提交检查点**

```bash
git add scripts appcast.xml Tests/AIMeterAppTests
git commit -m "build: add signed update release pipeline"
```

## 任务 6：完成真实更新集成验收

**文件：**
- 创建：`Tests/AIMeterAppTests/Fixtures/Updates/` 下的独立测试 feed 元数据（不含生产密钥）
- 修改：`Tests/AIMeterAppTests/SoftwareUpdateCoordinatorTests.swift`
- 修改：`docs/development/2026-09-02-github-app-update.md`

- [ ] **步骤 1：准备独立测试签名材料**

在临时目录生成只用于测试的 Sparkle key pair；私钥只存在临时目录并在测试退出时清理，fixture 只保留非敏感公钥和最小 appcast 输入。测试不读取生产 Keychain 项。

- [ ] **步骤 2：构建旧版与测试新版夹具**

从相同源码生成隔离的 `0.1.9 (3)` 测试 host 与 `0.2.0 (4)` 测试更新包，使用临时 feed URL 和测试公钥。两个 bundle 均从临时路径启动，避免覆盖 `/Applications/AI Token Meter.app`。

- [ ] **步骤 3：验收成功更新路径**

启动旧版，确认初始状态不联网；触发 Check for Updates 后发现 `0.2.0`；触发 Update Now；完成下载、签名校验、替换与重启；读取重启后 Info.plist 和进程路径确认版本/build 已变化。

- [ ] **步骤 4：验收失败安全路径**

复制并篡改测试 ZIP 的一个字节，保持 appcast 签名不变。再次运行更新，必须得到签名/完整性失败；旧版 bundle 哈希与可启动性保持不变。随后测试无更新与 feed 不可达路径。

- [ ] **步骤 5：记录证据并运行全量验证**

开发日志记录测试 feed、旧/新版本、安装前后 bundle 哈希、篡改结果和清理结果，不记录任何私钥内容。运行：

```bash
bash scripts/test.sh
AI_METER_INCLUDE_WIDGET=0 bash scripts/build-app.sh
scripts/verify-update-bundle.sh "dist/AI Token Meter.app"
git diff --check
```

预期：全部测试、文档、公开安全、Release 构建和更新 bundle 验证通过。

- [ ] **步骤 6：提交检查点**

```bash
git add Tests/AIMeterAppTests docs/development/2026-09-02-github-app-update.md
git commit -m "test: verify signed self update flow"
```

## 任务 7：同步全部文档并发布 v0.2.0

**文件：**
- 修改：`README.md`
- 修改：`CHANGELOG.md`
- 修改：`docs/README.md`
- 修改：`docs/project-status.md`
- 修改：`docs/user-guide/settings.md`
- 修改：`docs/user-guide/getting-started.md`
- 修改：`docs/user-guide/troubleshooting.md`
- 修改：`docs/architecture/overview.md`
- 修改：`docs/architecture/decisions.md`
- 修改：`docs/architecture/repository-structure.md`
- 修改：`docs/security-and-privacy.md`
- 修改：`docs/development/testing.md`
- 修改：`docs/development/release-process.md`
- 修改：`docs/development/maintenance-playbook.md`
- 修改：`docs/development/commit-history.md`
- 修改：`docs/development/README.md`
- 修改：`docs/requirements-backlog.md`

- [ ] **步骤 1：更新面向用户和维护者的文档**

写明：About 双按钮、只手动检查、`v0.1.2 → v0.2.0` 必须手动安装、以后可应用内更新、EdDSA 与 SHA-256 的不同职责、ad-hoc/Gatekeeper 限制、生产私钥位置边界、故障排查和回滚方式。

- [ ] **步骤 2：把版本记录转为 0.2.0**

把本次功能从 `CHANGELOG.md` 的 Unreleased 移到 `0.2.0 - 2026-09-02`；更新 README、项目状态、测试基线和发布文档，不能保留“尚无 Git tag”等已过时说法。

- [ ] **步骤 3：运行最终本地门禁**

```bash
bash scripts/test.sh
AI_METER_INCLUDE_WIDGET=0 bash scripts/build-app.sh
scripts/verify-app-resources.sh "dist/AI Token Meter.app"
scripts/verify-update-bundle.sh "dist/AI Token Meter.app"
scripts/check-public-release.sh "dist/releases/0.2.0/AI-Token-Meter-0.2.0-macOS-arm64.zip"
codesign --verify --deep --strict --verbose=2 "dist/AI Token Meter.app"
git diff --check
```

预期：全绿，生产更新私钥扫描为无命中。

- [ ] **步骤 4：提交发布候选检查点并审查**

```bash
git add README.md CHANGELOG.md docs
git commit -m "docs: publish app update guidance"
```

执行独立代码审查，优先检查供应链信任、私钥边界、Sparkle 公共 API、LSUIElement 前台窗口、嵌套签名和失败回滚；修复后重新跑最终门禁。

- [ ] **步骤 5：合并到 main 并创建 GitHub Release**

确认分支干净后合并 `codex/github-app-updates`；在 `main` 复跑全量门禁，创建 `release: AI Token Meter v0.2.0` 提交、带注释 tag `v0.2.0`，推送 main/tag，创建正式 GitHub Release 并上传 ZIP 与 `.sha256`。不覆盖同名未知资产。

- [ ] **步骤 6：发布 appcast 并远程复验**

推送生成的 `appcast.xml`，等待 GitHub Actions 通过；匿名读取 raw appcast 和 Release ZIP，核对 URL、长度、SHA-256、EdDSA、版本/build 和可解压启动。安装 `v0.2.0` 后在真实 Settings 验证“检查更新”报告最新版本。

- [ ] **步骤 7：完成需求台账与最终提交**

把 `REQ-20260902-019` 标记为 `已完成`，记录完成日期、设计规格、实施计划、开发日志、关键提交、最终 CI、Release URL、ZIP SHA-256 和真实检查更新结果。重新读取 backlog，确认没有被本任务遗漏的 `进行中` 或 `待处理` 高优先级事项。

## 计划自检

- **规格覆盖度：** Settings 双按钮、手动联网、GitHub 正式 Release、Sparkle 安装、EdDSA、首次手动升级、失败回退、ad-hoc 限制、测试和文档均对应到具体任务。
- **步骤完整性：** 每一步都给出具体目标、文件、接口、命令和预期结果；版本、路径、按钮文字、feed URL、工具输入和验收命令均明确。
- **类型一致性：** 全计划统一使用 `SoftwareUpdateRelease`、`SoftwareUpdateState`、`SoftwareUpdateEvent`、`SoftwareUpdateEngine`、`SoftwareUpdateCoordinator` 和 `SparkleUpdateEngine`；UI 动作固定为 `checkForUpdates()` 与 `installAvailableUpdate()`。
- **安全一致性：** 生产私钥只进入 Keychain，测试使用独立临时密钥，公钥进入 Info.plist，GitHub/API 元数据不是信任根，签名失败不可绕过。
- **发布一致性：** Sparkle 精确锁定 `2.9.4`；首个更新器版本固定 `0.2.0 (4)`；appcast 只列稳定 arm64、macOS 14+ Release。
- **范围一致性：** 不引入自动检查、静默安装、Intel、Widget 证书、Developer ID 购买或自制特权 helper。
