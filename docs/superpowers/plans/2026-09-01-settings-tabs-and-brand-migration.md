# AI Token Meter Settings 分类与品牌迁移实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 把单页 Settings 重构为顶部四 Tab，并把所有当前用户可见品牌迁移到 `AI Token Meter`，同时保留既有 Bundle Identifier、Keychain、偏好和数据目录。

**架构：** `SettingsView` 只负责 Tab 导航和临时 API Key 状态，四个专用子视图分别承载外观、监控、服务和关于内容；`SettingsTab` 提供稳定分类与消息路由。共享 `AppBrand` 放在 AIMeterCore，供菜单、可访问性和设置复用；系统 Bundle 元数据与构建脚本负责安装包名称，旧存储标识明确保留为兼容层。

**技术栈：** Swift 6、SwiftUI、AppKit、Swift Testing、Swift Package Manager、macOS 14、shell 构建脚本、ad-hoc codesign。

---

## 文件结构

### 新建

- `Sources/AIMeterApp/Views/SettingsTab.swift`：四个 Settings 分类、稳定顺序、图标与消息目的地。
- `Sources/AIMeterApp/Views/AppearanceSettingsView.swift`：浮动条、边缘、自动隐藏和字体设置。
- `Sources/AIMeterApp/Views/MonitoringSettingsView.swift`：刷新、通知和登录项设置。
- `Sources/AIMeterApp/Views/ServicesSettingsView.swift`：Claude/Codex 说明与 DeepSeek 基准、Keychain 操作。
- `Sources/AIMeterApp/Views/AboutSettingsView.swift`：App Icon、名称、副标题、Bundle 版本和隐私说明。
- `Sources/AIMeterCore/Presentation/AppBrand.swift`：用户可见品牌常量和 Bundle 版本格式化。
- `Tests/AIMeterAppTests/SettingsStructureTests.swift`：Tab 结构、控件归属、字体作用域和消息路由合同。
- `Tests/AIMeterCoreTests/AppBrandTests.swift`：品牌与版本格式化测试。
- `Tests/AIMeterAppTests/AppBundleMetadataTests.swift`：真实解析 Info.plist，验证用户可见 Bundle 元数据与兼容标识。
- `docs/development/2026-09-01-settings-tabs-and-brand-migration.md`：TDD、构建、安装、迁移和人工验收日志。

### 修改

- `Sources/AIMeterApp/Views/SettingsView.swift`：变为四 Tab 根容器。
- `Sources/AIMeterApp/AppModel.swift`：为设置消息记录目标 Tab，并更新用户可见 Claude 工作区文案。
- `Sources/AIMeterApp/AIMeterApp.swift`：采用确认后的 Settings 尺寸。
- `Sources/AIMeterApp/Views/MenuBarPanel.swift`：新名称、副标题和退出提示。
- `Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift`：用户可见品牌名。
- `Sources/AIMeterCore/Presentation/AppPresentation.swift`：可访问性品牌名。
- `Sources/AIMeterCore/Collectors/ClaudeUsageWorkspace.swift`：只加兼容注释，保留旧目录。
- `Sources/AIMeterApp/Resources/Info.plist`：显示名与 Bundle 名。
- `scripts/build-app.sh`：生成 `dist/AI Token Meter.app`。
- `Tests/AIMeterCoreTests/AppPresentationTests.swift`：新可访问性文本。
- `README.md`、`CHANGELOG.md`、`docs/README.md`、`docs/architecture/*.md`、`docs/user-guide/*.md`、`docs/development/*.md`、`docs/security-and-privacy.md`：当前文档品牌与状态同步；历史规格和历史日志不批量改写。
- `docs/next-phase-requirements.md`：R1/R4/R5 完成，R2 保留人工门槛，R3 待 Widget 阶段。

## 任务 1：四 Tab Settings 与消息路由

**文件：**
- 创建：`Sources/AIMeterApp/Views/SettingsTab.swift`
- 创建：`Sources/AIMeterApp/Views/AppearanceSettingsView.swift`
- 创建：`Sources/AIMeterApp/Views/MonitoringSettingsView.swift`
- 创建：`Sources/AIMeterApp/Views/ServicesSettingsView.swift`
- 创建：`Sources/AIMeterApp/Views/AboutSettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/SettingsView.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 修改：`Sources/AIMeterApp/AIMeterApp.swift`
- 测试：`Tests/AIMeterAppTests/SettingsStructureTests.swift`

- [ ] **步骤 1：编写四分类与消息路由的失败测试**

```swift
import Testing
@testable import AIMeterApp

@Suite("Settings information architecture")
struct SettingsStructureTests {
    @Test("Defines four ordered top tabs")
    func orderedTabs() {
        #expect(SettingsTab.allCases == [.appearance, .monitoring, .services, .about])
        #expect(SettingsTab.allCases.map(\.title) == ["Appearance", "Monitoring", "Services", "About"])
    }

    @Test("Routes settings messages to their owning tab")
    func messageRouting() {
        #expect(SettingsTab.monitoring.accepts(.launchAtLogin))
        #expect(SettingsTab.services.accepts(.claudeWorkspace))
        #expect(SettingsTab.services.accepts(.deepSeekCredential))
        #expect(!SettingsTab.appearance.accepts(.deepSeekCredential))
    }
}
```

- [ ] **步骤 2：运行测试并确认红灯来自缺失类型/接线**

运行：

```bash
swift test --filter SettingsStructureTests
```

预期：编译失败，提示 `SettingsTab` 或 `SettingsMessageKind` 不存在。

- [ ] **步骤 3：实现稳定 Tab 与消息种类**

```swift
enum SettingsMessageKind: Equatable {
    case launchAtLogin
    case claudeWorkspace
    case deepSeekCredential
}

enum SettingsTab: String, CaseIterable, Identifiable {
    case appearance, monitoring, services, about
    var id: Self { self }

    var title: String {
        switch self {
        case .appearance: "Appearance"
        case .monitoring: "Monitoring"
        case .services: "Services"
        case .about: "About"
        }
    }

    var systemImage: String {
        switch self {
        case .appearance: "paintbrush"
        case .monitoring: "waveform.path.ecg"
        case .services: "server.rack"
        case .about: "info.circle"
        }
    }

    func accepts(_ kind: SettingsMessageKind) -> Bool {
        switch (self, kind) {
        case (.monitoring, .launchAtLogin),
             (.services, .claudeWorkspace),
             (.services, .deepSeekCredential): true
        default: false
        }
    }
}
```

`AppModel` 新增只读 `settingsMessageKind`，所有写入 `settingsMessage` 的位置同时设置或清空对应 kind。不得把 API Key、错误对象或 CLI 原始输出放入消息。

- [ ] **步骤 4：拆分四个专用设置视图并接入 TabView**

`SettingsView` 保留 `@State private var pendingAPIKey = ""`，保证切换 Tab 不清空输入；使用：

```swift
TabView(selection: $selectedTab) {
    AppearanceSettingsView(model: model)
        .tabItem { Label(SettingsTab.appearance.title, systemImage: SettingsTab.appearance.systemImage) }
        .tag(SettingsTab.appearance)
    MonitoringSettingsView(model: model)
        .tabItem { Label(SettingsTab.monitoring.title, systemImage: SettingsTab.monitoring.systemImage) }
        .tag(SettingsTab.monitoring)
    ServicesSettingsView(model: model, pendingAPIKey: $pendingAPIKey)
        .tabItem { Label(SettingsTab.services.title, systemImage: SettingsTab.services.systemImage) }
        .tag(SettingsTab.services)
    AboutSettingsView()
        .tabItem { Label(SettingsTab.about.title, systemImage: SettingsTab.about.systemImage) }
        .tag(SettingsTab.about)
}
.aiMeterFontScope(.settings)
```

`AppearanceSettingsView`、`MonitoringSettingsView` 和 `ServicesSettingsView` 逐项搬移旧 `SettingsView` 控件，不改变绑定和说明；服务页为 Claude/Codex 加只读 CLI 管理说明。只有 `settingsMessageKind` 与当前页匹配时显示消息。任务 1 的 `AboutSettingsView` 只从真实 `Bundle.main.infoDictionary` 读取当前显示名和版本，不引入第二份品牌常量；任务 2 再将其统一接入 `AppBrand`。`AIMeterApp` 的 Settings frame 改为 560 × 540。

- [ ] **步骤 5：运行定向与完整字体/启动测试**

运行：

```bash
swift test --filter SettingsStructureTests
swift test --filter TypographyTests
swift test --filter AppModelStartupTests
```

预期：新 Settings 测试、现有 14 个字体测试和启动测试全部通过；没有设置内容使用 Antonio/DIN 预览。

- [ ] **步骤 6：提交任务 1**

```bash
git add Sources/AIMeterApp/Views/SettingsTab.swift \
  Sources/AIMeterApp/Views/AppearanceSettingsView.swift \
  Sources/AIMeterApp/Views/MonitoringSettingsView.swift \
  Sources/AIMeterApp/Views/ServicesSettingsView.swift \
  Sources/AIMeterApp/Views/AboutSettingsView.swift \
  Sources/AIMeterApp/Views/SettingsView.swift Sources/AIMeterApp/AppModel.swift \
  Sources/AIMeterApp/AIMeterApp.swift Tests/AIMeterAppTests/SettingsStructureTests.swift
git commit -m "feat: organize settings into four tabs"
```

## 任务 2：品牌常量、Bundle 元数据与构建产物

**文件：**
- 创建：`Sources/AIMeterCore/Presentation/AppBrand.swift`
- 创建：`Tests/AIMeterCoreTests/AppBrandTests.swift`
- 创建：`Tests/AIMeterAppTests/AppBundleMetadataTests.swift`
- 修改：`Sources/AIMeterCore/Presentation/AppPresentation.swift`
- 修改：`Sources/AIMeterApp/Views/MenuBarPanel.swift`
- 修改：`Sources/AIMeterApp/Views/AboutSettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift`
- 修改：`Sources/AIMeterApp/Views/MonitoringSettingsView.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 修改：`Sources/AIMeterApp/Resources/Info.plist`
- 修改：`scripts/build-app.sh`
- 修改：`Tests/AIMeterCoreTests/AppPresentationTests.swift`

- [ ] **步骤 1：编写失败的品牌与版本测试**

```swift
import Testing
@testable import AIMeterCore

@Suite("AI Token Meter brand")
struct AppBrandTests {
    @Test("Exposes approved visible copy")
    func visibleCopy() {
        #expect(AppBrand.displayName == "AI Token Meter")
        #expect(AppBrand.subtitle == "Private AI usage monitor")
    }

    @Test("Formats bundle version without inventing fields")
    func versionText() {
        #expect(AppBrand.versionText(info: [
            "CFBundleShortVersionString": "0.1.0",
            "CFBundleVersion": "1",
        ]) == "Version 0.1.0 (1)")
        #expect(AppBrand.versionText(info: [:]) == "Version unavailable")
    }
}
```

`AppBundleMetadataTests` 用 `PropertyListSerialization` 真实解析 `Sources/AIMeterApp/Resources/Info.plist`，断言 `CFBundleDisplayName`、`CFBundleName`、`CFBundleIdentifier`、`CFBundleExecutable` 和 `CFBundleIconFile`。构建脚本不靠文本扫描测试；任务 4 会真实执行脚本并验证产物路径、签名和 Bundle 内容。

```swift
@Test("Declares the visible brand without changing compatibility identity")
func bundleMetadata() throws {
    let data = try Data(contentsOf: Self.infoPlistURL)
    let plist = try #require(
        PropertyListSerialization.propertyList(from: data, format: nil) as? [String: Any]
    )
    #expect(plist["CFBundleDisplayName"] as? String == "AI Token Meter")
    #expect(plist["CFBundleName"] as? String == "AI Token Meter")
    #expect(plist["CFBundleIdentifier"] as? String == "com.millerpan.AIMeter")
    #expect(plist["CFBundleExecutable"] as? String == "AIMeterApp")
    #expect(plist["CFBundleIconFile"] as? String == "AppIcon")
}
```

- [ ] **步骤 2：运行测试并确认旧品牌导致失败**

运行：

```bash
swift test --filter AppBrandTests
swift test --filter AppBundleMetadataTests
swift test --filter AppPresentationTests
```

预期：`AppBrand` 缺失或 Info.plist/构建脚本/可访问性仍为 `AI Meter`。

- [ ] **步骤 3：实现 AppBrand 并替换当前用户可见文案**

```swift
public enum AppBrand {
    public static let displayName = "AI Token Meter"
    public static let subtitle = "Private AI usage monitor"

    public static func versionText(info: [String: Any]) -> String {
        guard let version = info["CFBundleShortVersionString"] as? String,
              let build = info["CFBundleVersion"] as? String else {
            return "Version unavailable"
        }
        return "Version \(version) (\(build))"
    }
}
```

菜单、About、DeepSeek 说明、登录项标签、Claude 工作区提示和 `AppPresentation` 可访问性全部使用新品牌。旧 `Application Support/AI Meter` 路径只加 `Legacy storage compatibility` 注释，不改目录；Bundle Identifier、可执行文件和 Keychain 标识不变。

- [ ] **步骤 4：更新 Info.plist 与构建脚本**

将 `CFBundleDisplayName`、`CFBundleName` 和 `scripts/build-app.sh` 的 `APP_NAME` 改为 `AI Token Meter`。保留：

```text
CFBundleIdentifier = com.millerpan.AIMeter
CFBundleExecutable = AIMeterApp
CFBundleIconFile = AppIcon
```

构建输出必须为 `dist/AI Token Meter.app`，图标生成与签名流程不变。

- [ ] **步骤 5：运行品牌、展示与完整测试**

运行：

```bash
swift test --filter AppBrandTests
swift test --filter AppBundleMetadataTests
swift test --filter AppPresentationTests
bash scripts/test.sh
```

预期：品牌合同通过；完整测试数量不少于任务开始时的 187 项，0 失败；环境门控检查只按既有规则跳过。

- [ ] **步骤 6：提交任务 2**

```bash
git add Sources/AIMeterCore/Presentation/AppBrand.swift \
  Sources/AIMeterCore/Presentation/AppPresentation.swift \
  Sources/AIMeterApp/Views/MenuBarPanel.swift Sources/AIMeterApp/Views/AboutSettingsView.swift \
  Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift \
  Sources/AIMeterApp/Views/MonitoringSettingsView.swift Sources/AIMeterApp/AppModel.swift \
  Sources/AIMeterApp/Resources/Info.plist scripts/build-app.sh \
  Tests/AIMeterCoreTests/AppBrandTests.swift Tests/AIMeterCoreTests/AppPresentationTests.swift \
  Tests/AIMeterAppTests/AppBundleMetadataTests.swift
git commit -m "feat: rename the product to AI Token Meter"
```

## 任务 3：当前文档、兼容说明与需求状态

**文件：**
- 修改：`README.md`
- 修改：`CHANGELOG.md`
- 修改：`docs/README.md`
- 修改：`docs/architecture/overview.md`
- 修改：`docs/architecture/repository-structure.md`
- 修改：`docs/user-guide/getting-started.md`
- 修改：`docs/user-guide/providers.md`
- 修改：`docs/user-guide/settings.md`
- 修改：`docs/user-guide/troubleshooting.md`
- 修改：`docs/development/README.md`
- 修改：`docs/development/commit-history.md`
- 修改：`docs/development/release-process.md`
- 修改：`docs/development/setup.md`
- 修改：`docs/development/testing.md`
- 修改：`docs/security-and-privacy.md`
- 修改：`docs/next-phase-requirements.md`
- 创建：`docs/development/2026-09-01-settings-tabs-and-brand-migration.md`

- [ ] **步骤 1：建立当前文档品牌扫描基线**

运行：

```bash
rg -n "AI Meter|dist/AI Meter\.app|/Applications/AI Meter\.app|Private usage monitor" \
  README.md CHANGELOG.md docs/README.md docs/architecture docs/user-guide \
  docs/development docs/security-and-privacy.md docs/next-phase-requirements.md
```

把命中分成：当前用户文档必须改名；历史开发记录保留；兼容路径必须保留并解释。不得对历史规格做无差别全局替换。

- [ ] **步骤 2：更新当前文档和迁移说明**

README 与用户指南使用 `AI Token Meter`、`AI Token Meter.app` 和新副标题。安装章节明确：旧包先备份到 `/private/tmp`，新包安装后 `/Applications` 只保留新名称；内部数据仍位于 `Application Support/AI Meter` 是兼容设计，不代表安装失败。

Settings 文档新增四 Tab 表格；安全文档说明 Bundle Identifier、Keychain 和数据目录未迁移；发布流程更新产物和签名命令。

- [ ] **步骤 3：修正阶段状态并记录本次开发日志骨架**

`docs/next-phase-requirements.md` 标记 R1/R4/R5 为本阶段已实现，R2 保留 Mission Control/普通 Space/多显示器门槛，R3 仍待独立 Widget 规格。删除“功能分支尚未合并 main”等已经过期的当前状态，但历史日志不改写。

新开发日志先写实现提交、红绿测试和兼容策略；构建/安装哈希与人工验收结果留到任务 4 以真实输出填写，不能预填成功。

- [ ] **步骤 4：运行文档与空白检查**

```bash
git diff --check
rg -n "Private usage monitor|dist/AI Meter\.app|/Applications/AI Meter\.app" \
  README.md docs/README.md docs/architecture docs/user-guide docs/security-and-privacy.md
```

预期：当前文档零命中旧副标题和旧安装包路径；旧内部数据目录只在带兼容解释的位置出现；没有未完成占位标记、虚构哈希或凭证。

- [ ] **步骤 5：提交任务 3**

```bash
git add README.md CHANGELOG.md docs
git commit -m "docs: document AI Token Meter migration"
```

## 任务 4：Release 构建、安装迁移与本机验收

**文件：**
- 修改：`docs/development/2026-09-01-settings-tabs-and-brand-migration.md`
- 修改：`docs/development/README.md`
- 修改：`docs/development/commit-history.md`
- 修改：`docs/next-phase-requirements.md`

- [ ] **步骤 1：运行完成前全量验证**

```bash
bash scripts/test.sh
bash scripts/build-app.sh
plutil -lint "dist/AI Token Meter.app/Contents/Info.plist"
codesign --verify --deep --strict --verbose=2 "dist/AI Token Meter.app"
file "dist/AI Token Meter.app/Contents/MacOS/AIMeterApp"
```

预期：完整测试 0 失败；产物存在；plist、严格签名通过；可执行文件为 arm64。记录测试数、套件数、耗时、候选 SHA-256 和文件大小。

- [ ] **步骤 2：安全备份并安装新名称**

解析明确路径，不使用 glob 或未验证变量：

```bash
pkill -x AIMeterApp || true
mkdir "/private/tmp/AI-Token-Meter-brand-migration-plan4"
```

创建前先只读确认该精确目录不存在；如果已存在则停止并另选一个明确路径，不能合并或覆盖旧备份。把实际存在的 `/Applications/AI Meter.app` 与 `/Applications/AI Token Meter.app` 分别移动到该备份目录，再用 `ditto` 安装候选。不得删除 Keychain、UserDefaults 或 `Application Support/AI Meter`。安装后重新执行 plist、严格签名、候选/安装主可执行文件 SHA-256 和目录清单比对。

- [ ] **步骤 3：人工验收 Settings、品牌和兼容状态**

检查：

1. `/Applications` 只存在 `AI Token Meter.app`；
2. 菜单标题为 `AI Token Meter`，副标题为 `Private AI usage monitor`；
3. 外观、监控、服务、关于四 Tab 可切换且无截断；
4. Settings 全程为系统字体，外部内容仍为 Antonio +1pt；
5. DeepSeek 临时 Key 输入切换 Tab 后不清空，保存后清空且 Keychain 配置仍存在；
6. About 显示当前图标与 `Version 0.1.0 (1)`；
7. 改名前保存的 Right、约 97%、Antonio、8 秒、通知与登录项状态保持；
8. Launch at Login 关闭再开启后只指向新包；
9. Claude、Codex、DeepSeek 刷新和详情打开无回归。

- [ ] **步骤 4：补写真实验收证据并复核隐私**

把真实命令、退出码、测试数、哈希、备份目录和人工结果写入开发日志。只记录 `apiKeyConfigured` 布尔状态，不输出 Keychain 内容、OAuth URL、Cookie、手机号、验证码或原始账户响应。

- [ ] **步骤 5：运行最终文档、签名、身份和工作区门控**

```bash
git diff --check
codesign --verify --deep --strict "/Applications/AI Token Meter.app"
shasum -a 256 "dist/AI Token Meter.app/Contents/MacOS/AIMeterApp" \
  "/Applications/AI Token Meter.app/Contents/MacOS/AIMeterApp"
git status --short
```

预期：两个 SHA-256 相同；状态只包含本任务验收文档；所有未覆盖人工门槛明确写为未覆盖，不能伪报通过。

- [ ] **步骤 6：提交任务 4**

```bash
git add docs/development/2026-09-01-settings-tabs-and-brand-migration.md \
  docs/development/README.md docs/development/commit-history.md \
  docs/next-phase-requirements.md
git commit -m "docs: record AI Token Meter release verification"
```

## 任务 5：独立审查与合并准备

- [ ] **步骤 1：逐条对照规格覆盖度**

核对四 Tab、系统字体隔离、品牌文案、旧存储兼容、安装迁移、文档状态和非目标。确认 WidgetKit、Provider 算法和浮动条视觉没有被意外修改。

- [ ] **步骤 2：运行新鲜完整验证**

```bash
bash scripts/test.sh
codesign --verify --deep --strict "/Applications/AI Token Meter.app"
git diff main...HEAD --check
git status --short --branch
```

预期：所有测试通过、安装签名通过、diff 无空白错误、功能工作区干净。

- [ ] **步骤 3：检查提交历史与回滚边界**

```bash
git log --oneline --decorate main..HEAD
git diff --stat main...HEAD
```

预期：设计、Settings、品牌、文档和验收形成可理解的独立提交；无凭证、构建缓存、`.superpowers` 原型或安装包进入 Git。

- [ ] **步骤 4：准备合并说明**

列出实现结果、测试证据、安装备份位置、未覆盖的 R2 人工门槛和仍未实现的 R3 Widget。按 finishing-a-development-branch 流程让用户选择本地合并、PR、保留或放弃；用户已经要求持续开发，但不得未经选择自动推送远端。
