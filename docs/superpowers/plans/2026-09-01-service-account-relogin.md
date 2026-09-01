# 服务账户显示与重新登录实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 在 Settings > Services 中持续显示 Claude、Codex、DeepSeek 的当前账户状态，提供 Claude/Codex 官方 CLI 重新登录，并让 DeepSeek 只在候选 Key 验证成功后安全替换旧 Key。

**架构：** `AIMeterCore` 新增不持久化的服务账户状态、Claude/Codex 状态探针和 DeepSeek 凭据管理器；`AIMeterApp` 新增只启动固定官方命令的 Terminal 启动器，并由 AppModel 管理状态回查、反馈和 Settings 展示。账户邮箱和 Key 后缀只存在内存，不进入 `UsageSnapshot`、缓存或 Widget。

**技术栈：** Swift 6、SwiftUI、Observation、AppKit `NSWorkspace`、现有 `CommandRunning`/Codex app-server/`DeepSeekClient`/`SecretStore`、Swift Testing、Keychain。

---

## 文件结构

### 新建

- `Sources/AIMeterCore/Accounts/ServiceAccountStatus.swift`：账户连接状态、最小展示身份和读取协议。
- `Sources/AIMeterCore/Accounts/ClaudeAccountReader.swift`：解析并执行 `claude auth status --json`。
- `Sources/AIMeterCore/Accounts/CodexAccountReader.swift`：通过 `account/read` 映射 ChatGPT/API Key 账户。
- `Sources/AIMeterCore/Accounts/DeepSeekCredentialManager.swift`：异步读取遮罩 Key、验证候选 Key、失败时保留旧 Key。
- `Sources/AIMeterCore/Accounts/ServiceAccountCoordinator.swift`：按 Provider 路由账户读取，不持久化身份。
- `Sources/AIMeterCore/Accounts/CLIAuthenticationScriptBuilder.swift`：生成两个固定、安全 quoting 的登录脚本。
- `Sources/AIMeterApp/System/CLIAuthenticationLauncher.swift`：定位 CLI、写入 0700 `.command` 并用 Terminal 打开。
- `Sources/AIMeterApp/Views/ServiceAccountStatusView.swift`：Services 中复用的状态/身份行。
- `Tests/AIMeterCoreTests/ServiceAccountStatusTests.swift`：状态模型与降级语义。
- `Tests/AIMeterCoreTests/ClaudeAccountReaderTests.swift`：Claude JSON、缺少邮箱、退出码、超时与未安装。
- `Tests/AIMeterCoreTests/CodexAccountReaderTests.swift`：ChatGPT 邮箱/套餐、API Key、空账户和错误。
- `Tests/AIMeterCoreTests/DeepSeekCredentialManagerTests.swift`：两阶段替换和全部旧 Key 保留路径。
- `Tests/AIMeterCoreTests/CLIAuthenticationScriptBuilderTests.swift`：固定命令、quoting 和无秘密合同。
- `Tests/AIMeterAppTests/CLIAuthenticationLauncherTests.swift`：目录、权限、Provider 路由与打开失败。
- `Tests/AIMeterAppTests/ServiceAccountSettingsTests.swift`：AppModel 状态、登录轮询、按钮和反馈路由。
- `docs/development/2026-09-01-service-account-relogin.md`：开发、测试、构建、安装和实机验收日志。

### 修改

- `Sources/AIMeterCore/Collectors/ClaudeCollector.swift`：复用统一 Claude 状态解析，保持用量预检行为。
- `Sources/AIMeterCore/Collectors/CodexAppServerClient.swift`：抽取初始化请求生命周期并增加 `account/read`。
- `Sources/AIMeterApp/AppModel.swift`：内存账户状态、刷新、登录回查和 DeepSeek 替换状态。
- `Sources/AIMeterApp/Views/ServicesSettingsView.swift`：三服务账户状态、始终可见操作和安全换 Key UI。
- `Sources/AIMeterApp/Views/SettingsTab.swift`：区分 Claude 登录、Claude 工作区、Codex 登录、DeepSeek Credential 反馈。
- `Sources/AIMeterApp/Views/SettingsView.swift`：Settings 打开时触发账户状态读取，保留系统字体边界。
- `Tests/AIMeterAppTests/AppModelStartupTests.swift`：初始化不阻塞 Keychain、账户刷新和隐私边界。
- `Tests/AIMeterAppTests/SettingsStructureTests.swift`：新增反馈类型与 Services 路由。
- `Tests/AIMeterCoreTests/CLICollectorTests.swift`：Claude 统一解析和 Codex app-server 重构回归。
- `Tests/AIMeterCoreTests/PrivacyRegressionTests.swift`：账户身份不进入缓存/Widget/日志合同。
- `README.md`、`CHANGELOG.md`、`docs/user-guide/settings.md`、`docs/user-guide/troubleshooting.md`、`docs/security-and-privacy.md`：用户说明和安全边界。
- `docs/development/README.md`、`docs/development/commit-history.md`、`docs/requirements-backlog.md`：开发索引、提交节点和需求状态。

## 任务 1：账户状态模型与 Claude 状态探针

**文件：**
- 创建：`Sources/AIMeterCore/Accounts/ServiceAccountStatus.swift`
- 创建：`Sources/AIMeterCore/Accounts/ClaudeAccountReader.swift`
- 创建：`Tests/AIMeterCoreTests/ServiceAccountStatusTests.swift`
- 创建：`Tests/AIMeterCoreTests/ClaudeAccountReaderTests.swift`
- 修改：`Sources/AIMeterCore/Collectors/ClaudeCollector.swift`
- 修改：`Tests/AIMeterCoreTests/CLICollectorTests.swift`

- [ ] **步骤 1：编写账户状态与 Claude 解析失败测试**

```swift
@Test("Claude connected status keeps only display identity")
func claudeConnected() throws {
    let status = try ClaudeAccountStatusParser().parse(#"{"loggedIn":true,"email":"m@example.com","authMethod":"oauth"}"#)
    #expect(status.provider == .claude)
    #expect(status.connectionState == .connected)
    #expect(status.accountLabel == "m@example.com")
    #expect(status.accountDetail == "OAuth")
}

@Test("Claude logged out is distinct from an unavailable probe")
func claudeLoggedOut() throws {
    let status = try ClaudeAccountStatusParser().parse(#"{"loggedIn":false,"authMethod":"none"}"#)
    #expect(status.connectionState == .signInRequired)
    #expect(status.accountLabel == nil)
}
```

- [ ] **步骤 2：运行定向测试确认因类型缺失而失败**

运行：

```bash
swift test --filter 'ServiceAccountStatusTests|ClaudeAccountReaderTests'
```

预期：编译失败，报告 `ServiceAccountStatus`、`ClaudeAccountStatusParser` 或 `ClaudeAccountReader` 不存在。

- [ ] **步骤 3：实现最小状态模型、解析器和读取器**

```swift
public enum ServiceAccountConnectionState: Equatable, Sendable {
    case connected
    case signInRequired
    case notInstalled
    case checking
    case unavailable
}

public struct ServiceAccountStatus: Equatable, Sendable {
    public let provider: UsageProvider
    public let connectionState: ServiceAccountConnectionState
    public let accountLabel: String?
    public let accountDetail: String?
    public let checkedAt: Date?
}

public protocol ServiceAccountReading: Sendable {
    var provider: UsageProvider { get }
    func read() async -> ServiceAccountStatus
}
```

`ClaudeAccountReader` 定位 `claude`，以 5 秒上限执行 `auth status --json`，先从输出中提取 JSON，再解析 `loggedIn`、可选 `email`、`authMethod` 和可选订阅字段。未安装返回 `.notInstalled`；明确 `loggedIn = false` 返回 `.signInRequired`；无法解析或执行失败返回 `.unavailable`。不得把原始输出保存在状态中。

- [ ] **步骤 4：让 ClaudeCollector 复用统一解析器**

把 `ClaudeCollector` 私有 `ClaudeAuthStatus` 替换为 `ClaudeAccountStatusParser` 的最小认证结果；保持现有 `.authenticationRequired`、`.transportFailure`、工作区授权和 `/usage` 行为不变。

- [ ] **步骤 5：运行定向回归并确认通过**

运行：

```bash
swift test --filter 'ServiceAccountStatusTests|ClaudeAccountReaderTests|CLICollectorTests'
```

预期：新增 Claude 状态测试和既有 Collector 测试全部通过。

- [ ] **步骤 6：提交任务 1**

```bash
git add Sources/AIMeterCore/Accounts Tests/AIMeterCoreTests/ServiceAccountStatusTests.swift Tests/AIMeterCoreTests/ClaudeAccountReaderTests.swift Sources/AIMeterCore/Collectors/ClaudeCollector.swift Tests/AIMeterCoreTests/CLICollectorTests.swift
git commit -m "feat: read Claude account status"
```

## 任务 2：Codex `account/read` 与账户展示映射

**文件：**
- 创建：`Sources/AIMeterCore/Accounts/CodexAccountReader.swift`
- 创建：`Tests/AIMeterCoreTests/CodexAccountReaderTests.swift`
- 修改：`Sources/AIMeterCore/Collectors/CodexAppServerClient.swift`
- 修改：`Tests/AIMeterCoreTests/CLICollectorTests.swift`
- 修改：`Tests/AIMeterCoreTests/Fixtures/fake-interactive-cli.sh`

- [ ] **步骤 1：扩展假 Codex app-server 响应并编写失败测试**

假 CLI 对 `account/read` 返回：

```json
{
  "id": 2,
  "result": {
    "account": {
      "type": "chatgpt",
      "email": "codex@example.com",
      "planType": "pro"
    },
    "requiresOpenaiAuth": true
  }
}
```

测试：

```swift
@Test("Codex account read exposes ChatGPT email and plan")
func chatGPTAccount() async {
    let status = await CodexAccountReader(locator: FixedLocator(url: fixtureExecutable)).read()
    #expect(status.connectionState == .connected)
    #expect(status.accountLabel == "codex@example.com")
    #expect(status.accountDetail == "ChatGPT · Pro")
}
```

另外覆盖 `type = apiKey`、`account = null`、未安装和无效响应。

- [ ] **步骤 2：运行测试确认 `account/read` 能力缺失**

运行：

```bash
swift test --filter 'CodexAccountReaderTests|CLICollectorTests'
```

预期：编译失败或读取失败，因为 `CodexAppServerClient.readAccount` 尚不存在。

- [ ] **步骤 3：抽取 app-server 请求生命周期并实现 `readAccount`**

在 `CodexAppServerClient` 内复用现有超时、注册竞态、进程树清理和 JSON 行读取：

```swift
func readAccount(
    executableURL: URL,
    timeout: TimeInterval = 10
) async throws -> CodexAccountResult
```

请求固定为：

```swift
[
    "id": 2,
    "method": "account/read",
    "params": ["refreshToken": false],
]
```

ChatGPT 映射邮箱和规范化套餐名；API Key 只显示 `API Key account`；空账户映射 `.signInRequired`。不把账户字段加入 `UsageSnapshot`。

- [ ] **步骤 4：运行 Codex 定向测试并确认额度读取未回归**

运行：

```bash
swift test --filter 'CodexAccountReaderTests|CLICollectorTests'
```

预期：账户测试、额度窗口选择、重置券、超时杀进程和早期超时竞态全部通过。

- [ ] **步骤 5：提交任务 2**

```bash
git add Sources/AIMeterCore/Accounts/CodexAccountReader.swift Sources/AIMeterCore/Collectors/CodexAppServerClient.swift Tests/AIMeterCoreTests/CodexAccountReaderTests.swift Tests/AIMeterCoreTests/CLICollectorTests.swift Tests/AIMeterCoreTests/Fixtures/fake-interactive-cli.sh
git commit -m "feat: read Codex account identity"
```

## 任务 3：官方 CLI 登录启动器与有限状态回查

**文件：**
- 创建：`Sources/AIMeterCore/Accounts/CLIAuthenticationScriptBuilder.swift`
- 创建：`Sources/AIMeterApp/System/CLIAuthenticationLauncher.swift`
- 创建：`Tests/AIMeterCoreTests/CLIAuthenticationScriptBuilderTests.swift`
- 创建：`Tests/AIMeterAppTests/CLIAuthenticationLauncherTests.swift`

- [ ] **步骤 1：编写固定命令、quoting、权限和错误测试**

```swift
@Test("Claude and Codex scripts contain only the approved login commands")
func approvedCommands() throws {
    let builder = CLIAuthenticationScriptBuilder()
    let claude = try builder.build(provider: .claude, executableURL: URL(fileURLWithPath: "/tmp/Claude CLI/claude"))
    let codex = try builder.build(provider: .codex, executableURL: URL(fileURLWithPath: "/tmp/Codex CLI/codex"))
    #expect(claude.contains("'\/tmp\/Claude CLI\/claude' auth login"))
    #expect(codex.contains("'\/tmp\/Codex CLI\/codex' login"))
    #expect(!claude.localizedCaseInsensitiveContains("token"))
    #expect(!codex.localizedCaseInsensitiveContains("api key"))
}
```

App 测试注入临时目录、固定 locator 和假的 `open` 闭包，断言两个脚本文件权限为 `0700`；DeepSeek 路由被拒绝；缺少 CLI 与 `NSWorkspace.open` 失败映射成明确错误。

- [ ] **步骤 2：运行测试确认 builder/launcher 不存在**

运行：

```bash
swift test --filter 'CLIAuthenticationScriptBuilderTests|CLIAuthenticationLauncherTests'
```

预期：编译失败，报告两个新类型不存在。

- [ ] **步骤 3：实现脚本 Builder 与 AppKit Launcher**

Builder 只接受 `.claude` 和 `.codex`，用单引号安全引用可执行路径；脚本不继承或写入秘密。Launcher 把脚本写到：

```text
~/Library/Application Support/AI Meter/Authentication/Open Claude Login.command
~/Library/Application Support/AI Meter/Authentication/Open Codex Login.command
```

以 `Data.write(.atomic)` 写入后设置 `0700`，再调用注入的 `open(scriptURL)`；生产使用 `NSWorkspace.shared.open`。

- [ ] **步骤 4：运行启动器测试并确认通过**

运行：

```bash
swift test --filter 'CLIAuthenticationScriptBuilderTests|CLIAuthenticationLauncherTests'
```

预期：固定命令、quoting、权限、错误和无秘密检查全部通过。

- [ ] **步骤 5：提交任务 3**

```bash
git add Sources/AIMeterCore/Accounts/CLIAuthenticationScriptBuilder.swift Sources/AIMeterApp/System/CLIAuthenticationLauncher.swift Tests/AIMeterCoreTests/CLIAuthenticationScriptBuilderTests.swift Tests/AIMeterAppTests/CLIAuthenticationLauncherTests.swift
git commit -m "feat: launch official CLI sign in"
```

## 任务 4：DeepSeek 候选 Key 验证与旧 Key 保留

**文件：**
- 创建：`Sources/AIMeterCore/Accounts/DeepSeekCredentialManager.swift`
- 创建：`Tests/AIMeterCoreTests/DeepSeekCredentialManagerTests.swift`
- 修改：`Tests/AIMeterCoreTests/DeepSeekClientTests.swift`

- [ ] **步骤 1：编写两阶段替换失败测试**

```swift
@Test("An invalid candidate never replaces the working DeepSeek key")
func invalidCandidateKeepsOldKey() async {
    let store = RecordingSecretStore(initial: "old-working-key")
    let manager = DeepSeekCredentialManager(
        secretStore: store,
        validate: { _ in throw UsageCollectionError.authenticationRequired }
    )

    await #expect(throws: DeepSeekCredentialReplacementError.invalidKey) {
        try await manager.replace(with: "new-invalid-key")
    }
    #expect(store.secret == "old-working-key")
    #expect(store.savedValues.isEmpty)
}
```

再覆盖成功替换、首次保存、网络/超时/无效响应、Keychain 写入失败及写入部分成功后抛错的回滚。状态读取只返回 `API Key ••••ABCD`，慢 Keychain 读取在期限内降级为 unavailable。

- [ ] **步骤 2：运行测试确认 Manager 不存在**

运行：

```bash
swift test --filter 'DeepSeekCredentialManagerTests|DeepSeekClientTests'
```

预期：编译失败，报告 `DeepSeekCredentialManager` 和替换错误类型不存在。

- [ ] **步骤 3：实现凭据 Manager**

```swift
public enum DeepSeekCredentialReplacementError: Error, Equatable, Sendable {
    case emptyCandidate
    case invalidKey
    case verificationUnavailable
    case keychainFailure
}
```

`replace(with:)` 顺序固定为：标准化候选值 → 使用 `DeepSeekClient.collect(apiKey:)` 验证 → 读取旧值 → 原子保存候选值 → 失败时尽力恢复旧值 → 返回新的遮罩状态。认证错误映射 `.invalidKey`；其他 API 错误映射 `.verificationUnavailable`；不在验证前调用 `save` 或 `delete`。

`readStatus()` 把 Keychain 同步读取放入受限后台任务，完整 Key 只在局部作用域计算后四位，不存入 `ServiceAccountStatus`。

- [ ] **步骤 4：运行 DeepSeek 定向测试并确认通过**

运行：

```bash
swift test --filter 'DeepSeekCredentialManagerTests|DeepSeekClientTests'
```

预期：两阶段替换、错误分类、回滚、遮罩和现有余额测试全部通过。

- [ ] **步骤 5：提交任务 4**

```bash
git add Sources/AIMeterCore/Accounts/DeepSeekCredentialManager.swift Tests/AIMeterCoreTests/DeepSeekCredentialManagerTests.swift Tests/AIMeterCoreTests/DeepSeekClientTests.swift
git commit -m "feat: safely replace DeepSeek API keys"
```

## 任务 5：AppModel 编排与 Services UI

**文件：**
- 创建：`Sources/AIMeterCore/Accounts/ServiceAccountCoordinator.swift`
- 创建：`Sources/AIMeterApp/Views/ServiceAccountStatusView.swift`
- 创建：`Tests/AIMeterAppTests/ServiceAccountSettingsTests.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 修改：`Sources/AIMeterApp/Views/ServicesSettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/SettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/SettingsTab.swift`
- 修改：`Tests/AIMeterAppTests/AppModelStartupTests.swift`
- 修改：`Tests/AIMeterAppTests/SettingsStructureTests.swift`
- 修改：`Tests/AIMeterCoreTests/PrivacyRegressionTests.swift`

- [ ] **步骤 1：编写 AppModel 和 UI 合同失败测试**

```swift
@Test("Connected CLI accounts keep sign in again available")
@MainActor
func connectedAccountCanRelogin() async {
    let model = makeModel(statuses: [
        .claude: ServiceAccountStatus.connected(.claude, label: "m@example.com", detail: "OAuth")
    ])
    await model.refreshServiceAccounts()
    #expect(model.serviceAccounts[.claude]?.accountLabel == "m@example.com")
    #expect(model.signInButtonTitle(for: .claude) == "Sign in again")
}

@Test("A successful DeepSeek replacement clears the field and refreshes")
@MainActor
func replaceDeepSeek() async {
    let model = makeModel(replaceResult: .connected(.deepSeek, label: "API Key ••••ABCD"))
    #expect(await model.replaceDeepSeekAPIKey("new-key"))
    #expect(model.serviceAccounts[.deepSeek]?.accountLabel == "API Key ••••ABCD")
}
```

增加状态路由、未安装禁用、checking、登录启动失败、有限回查成功/超时/旧任务取消、候选 Key 失败不清空、Settings 系统字体和 UI 文案源码合同。

- [ ] **步骤 2：运行 App/Settings 定向测试确认失败**

运行：

```bash
swift test --filter 'ServiceAccountSettingsTests|AppModelStartupTests|SettingsStructureTests|PrivacyRegressionTests'
```

预期：账户状态、登录方法、DeepSeek 异步替换和 UI 类型不存在。

- [ ] **步骤 3：实现 Coordinator 与 AppModel 状态流**

`ServiceAccountCoordinator` 按 Provider 调用对应 Reader。AppModel 新增：

```swift
private(set) var serviceAccounts: [UsageProvider: ServiceAccountStatus]
private(set) var isReplacingDeepSeekAPIKey: Bool

func refreshServiceAccounts() async
func checkServiceAccount(_ provider: UsageProvider) async
func beginSignIn(_ provider: UsageProvider)
func replaceDeepSeekAPIKey(_ candidate: String) async -> Bool
func signInButtonTitle(for provider: UsageProvider) -> String
```

`start()` 异步读取账户状态，不在初始化器读取 Keychain。登录成功后自动调用 `refresh()`；登录回查最多 40 次、间隔 3 秒，测试通过注入 sleep 和策略使用即时钟。每个 Provider 只有一个回查 Task，应用停止时全部取消。

- [ ] **步骤 4：实现 Services UI**

Claude/Codex Section 显示状态、账户、套餐/认证方式、始终可见的 `Sign in` 或 `Sign in again`，并保留 Claude 工作区授权。DeepSeek 显示遮罩 Key、空 SecureField、`Save API Key`/`Replace API Key`、`Verifying…` 和 Remove。

按钮动作：

```swift
Button(model.signInButtonTitle(for: .claude)) {
    model.beginSignIn(.claude)
}

Button(model.apiKeyConfigured ? "Replace API Key" : "Save API Key") {
    Task {
        if await model.replaceDeepSeekAPIKey(pendingAPIKey) {
            pendingAPIKey = ""
        }
    }
}
```

Settings 页面出现时调用一次 `refreshServiceAccounts()`；不得使用 `.aiMeterFontScope(.content)`。

- [ ] **步骤 5：加入隐私和启动非阻塞合同**

测试证明：AppModel 初始化后 secret store readCount 仍为 0；账户邮箱不出现在 `WidgetSnapshotEnvelope` JSON、SnapshotCache 文件和命令脚本；API Key 后缀只出现在内存状态。

- [ ] **步骤 6：运行 App/Settings 定向测试并确认通过**

运行：

```bash
swift test --filter 'ServiceAccountSettingsTests|AppModelStartupTests|SettingsStructureTests|PrivacyRegressionTests'
```

预期：状态、按钮、轮询、替换、系统字体和隐私合同全部通过。

- [ ] **步骤 7：提交任务 5**

```bash
git add Sources/AIMeterCore/Accounts/ServiceAccountCoordinator.swift Sources/AIMeterApp/AppModel.swift Sources/AIMeterApp/Views/ServiceAccountStatusView.swift Sources/AIMeterApp/Views/ServicesSettingsView.swift Sources/AIMeterApp/Views/SettingsView.swift Sources/AIMeterApp/Views/SettingsTab.swift Tests/AIMeterAppTests/ServiceAccountSettingsTests.swift Tests/AIMeterAppTests/AppModelStartupTests.swift Tests/AIMeterAppTests/SettingsStructureTests.swift Tests/AIMeterCoreTests/PrivacyRegressionTests.swift
git commit -m "feat: manage service accounts in Settings"
```

## 任务 6：文档、全量验证、安装和真实验收

**文件：**
- 创建：`docs/development/2026-09-01-service-account-relogin.md`
- 修改：`README.md`
- 修改：`CHANGELOG.md`
- 修改：`docs/user-guide/settings.md`
- 修改：`docs/user-guide/troubleshooting.md`
- 修改：`docs/security-and-privacy.md`
- 修改：`docs/development/README.md`
- 修改：`docs/development/commit-history.md`
- 修改：`docs/requirements-backlog.md`

- [ ] **步骤 1：运行完整自动化验证**

运行：

```bash
bash scripts/test.sh
```

预期：原有 227 项和新增测试全部通过，0 failures；环境门控项只按既有规则跳过。

- [ ] **步骤 2：执行源码和隐私静态门控**

运行：

```bash
rg -n 'ServiceAccountStatus|accountLabel|API Key ••••' Sources/AIMeterCore/Cache Sources/AIMeterCore/Widget Sources/AIMeterWidgetExtension
rg -n 'auth login|codex.*login' Sources/AIMeterCore/Accounts/CLIAuthenticationScriptBuilder.swift
git diff --check
```

预期：第一个命令无账户身份持久化命中；第二个命令只有批准的固定登录命令；diff 无空白错误。

- [ ] **步骤 3：更新用户、隐私、故障排查和开发文档**

文档必须说明：Claude/Codex 按钮启动官方 CLI、账户字段降级、DeepSeek 新 Key 先验证、邮箱/Key 后缀只在内存、当前需求 `REQ-20260901-001` 的状态和完成证据。把 `REQ-20260901-002` 标记已完成并记录 `c369b07`。

- [ ] **步骤 4：构建 Release 候选并验证签名**

运行：

```bash
bash scripts/build-app.sh
codesign --verify --deep --strict --verbose=2 "dist/AI Token Meter.app"
plutil -lint "dist/AI Token Meter.app/Contents/Info.plist"
```

预期：构建退出 0，签名和 plist 验证通过；无 Apple Development 证书时继续构建无 Widget 主应用，不恢复已延期的 Widget 事项。

- [ ] **步骤 5：安全替换安装并启动候选版**

完全退出当前 AI Token Meter，把旧 `/Applications/AI Token Meter.app` 移到唯一 `/private/tmp` 备份目录，再用 `ditto` 安装候选。验证候选和安装版主可执行文件哈希一致，严格签名通过。

- [ ] **步骤 6：真实 Settings 验收**

使用 Computer Use 验证：

1. Services 三段始终有状态和操作；
2. 当前 Codex 显示 ChatGPT 邮箱/套餐，按钮为 Sign in again；
3. 当前 Claude 若未登录则显示 Sign-in required 和 Sign in；
4. 点击登录按钮能打开官方 Terminal 流程；不要替用户提交账户或 MFA；
5. DeepSeek 显示遮罩 Key 和 Replace API Key；
6. 不使用真实错误 Key 覆盖用户凭据；以注入测试作为失败路径证据；
7. Settings 字体仍为系统字体，其他 Tab 和浮动条不回归；
8. 验收后关闭额外 Terminal 登录窗口，保持原账户和 API Key 不变。

- [ ] **步骤 7：独立代码审查与修复**

按 correctness、并发取消、凭据安全、错误映射、UI 一致性和测试证据检查分支 diff；重要问题先修复并重新运行相关测试。

- [ ] **步骤 8：提交文档与验收节点**

```bash
git add README.md CHANGELOG.md docs
git commit -m "docs: record service account acceptance"
```

- [ ] **步骤 9：合入 main 并更新需求列表最终状态**

在所有验证通过后把 `codex/service-account-relogin` 合入 `main`。在 `docs/requirements-backlog.md` 将 `REQ-20260901-001` 与 `REQ-20260901-002` 标记 `已完成`，写入最终 merge 提交和开发日志；重新运行 `git diff --check`、`git status`、安装版签名和偏好检查。

## 完成定义

- Claude、Codex、DeepSeek 当前身份和准确降级状态在 Services 中始终可见；
- Claude/Codex 官方登录流程可一键启动，登录后自动回查，旧任务可取消；
- DeepSeek 候选 Key 只有验证成功才替换，任何失败路径都保留旧 Key；
- 账户邮箱、套餐和 Key 后缀没有进入缓存、Widget、通知或脚本；
- Settings 保持系统字体，浮动条与现有额度显示没有回归；
- 完整测试、Release 构建、签名、安装和真实 Settings 验收均有新鲜证据；
- 需求列表和 Git 历史包含可追溯的设计、计划、实现、验收与完成状态。
