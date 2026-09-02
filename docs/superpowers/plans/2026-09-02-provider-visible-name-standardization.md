# Provider 用户可见名称统一实施计划

> **面向 AI 代理的工作者：** 必需子技能：使用 `executing-plans` 逐任务实施；每项行为变更先执行 `test-driven-development`，完成声明前执行 `verification-before-completion`。

**目标：** 将当前产品中所有面向用户的 `Claude` 统一为 `Claude Code`，将 `Codex` 统一为 `OpenAI Codex`，同时保持 CLI 命令、持久化标识、路径和历史记录兼容。

**架构：** 在 `AIMeterCore` 的 `UsageProvider` 与 `WidgetProvider` 建立唯一正式名称源。主应用展示模型、SwiftUI、通知和 Widget 全部复用正式名称；句子型文案只组合正式名称，不复制简称 switch。内部枚举、命令与文件路径不改名。

**技术栈：** Swift 6、SwiftUI、WidgetKit、Swift Testing、Swift Package Manager、macOS 14+

---

## 文件结构

- 修改：`Sources/AIMeterCore/Domain/UsageModels.swift` — 为 `UsageProvider` 增加正式显示名称。
- 修改：`Sources/AIMeterCore/Widget/WidgetSnapshotModels.swift` — 为 `WidgetProvider` 增加正式显示名称。
- 修改：`Sources/AIMeterCore/Presentation/AppPresentation.swift` — 展示标题复用核心名称。
- 修改：`Sources/AIMeterApp/Views/UsageVisualStyle.swift` — 删除主应用重复名称定义。
- 修改：`Sources/AIMeterApp/Views/ClaudeDetailView.swift`、`CodexDetailView.swift`、`ServicesSettingsView.swift`、`MenuBarPanel.swift`、`AboutSettingsView.swift` — 更新当前可见标题与说明。
- 修改：`Sources/AIMeterApp/AppModel.swift`、`Sources/AIMeterApp/System/NotificationService.swift` — 更新账户反馈、Demo 标签与通知。
- 修改：`Sources/AIMeterCore/Collectors/TerminalUsageParser.swift`、`Sources/AIMeterCore/Accounts/ClaudeAccountReader.swift` — 更新会直接呈现给用户的错误与账户标签。
- 修改：`Sources/AIMeterWidgetExtension/AITokenMeterWidget.swift`、`Sources/AIMeterWidgetExtension/Views/WidgetProviderLogo.swift` 及名称消费者 — Widget 复用核心名称。
- 修改：对应 Core、App、Widget 测试 — 锁定正式名称和当前文案。
- 修改：`README.md`、`CHANGELOG.md` 与现行 `docs/` 用户/架构/隐私/测试/发布文档 — 同步当前产品名称，不重写历史规格和开发日志。
- 创建：`docs/development/2026-09-02-provider-visible-name-standardization.md` — 记录红绿测试、构建、安装、真实验收与 Git 证据。
- 修改：`docs/development/README.md`、`docs/README.md`、`docs/development/commit-history.md`、`docs/requirements-backlog.md` — 添加入口并完成需求状态。

### 任务 1：建立核心正式名称源

**文件：**
- 修改：`Tests/AIMeterCoreTests/AppPresentationTests.swift`
- 修改：`Sources/AIMeterCore/Domain/UsageModels.swift`
- 修改：`Sources/AIMeterCore/Widget/WidgetSnapshotModels.swift`
- 修改：`Sources/AIMeterCore/Presentation/AppPresentation.swift`
- 修改：`Sources/AIMeterApp/Views/UsageVisualStyle.swift`

- [x] **步骤 1：先写失败测试**

在 `AppPresentationTests` 中增加并更新断言：

```swift
@Test("Providers expose canonical user-facing names")
func canonicalProviderNames() {
    #expect(UsageProvider.claude.displayName == "Claude Code")
    #expect(UsageProvider.codex.displayName == "OpenAI Codex")
    #expect(UsageProvider.deepSeek.displayName == "DeepSeek")
    #expect(WidgetProvider.claude.displayName == "Claude Code")
    #expect(WidgetProvider.codex.displayName == "OpenAI Codex")
    #expect(WidgetProvider.deepSeek.displayName == "DeepSeek")
}
```

同时把既有 `ProviderPresentation` 标题期望改为 `Claude Code`，并增加 `OpenAI Codex` 覆盖。

- [x] **步骤 2：验证测试先红**

```bash
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-provider-names-core \
  bash scripts/test.sh --filter AppPresentationTests
```

预期：编译失败或标题断言失败，因为核心正式名称尚未实现。

- [x] **步骤 3：实现最小核心映射**

为两个 Provider 枚举分别增加：

```swift
public var displayName: String {
    switch self {
    case .claude: "Claude Code"
    case .codex: "OpenAI Codex"
    case .deepSeek: "DeepSeek"
    }
}
```

让 `ProviderPresentation.title` 返回 `snapshot.provider.displayName`，删除 `UsageVisualStyle.swift` 中重复的 `displayName`，保留颜色和符号映射。

- [x] **步骤 4：运行定向测试至绿**

重复步骤 2 命令，预期全部通过。

- [x] **步骤 5：提交核心名称检查点**

```bash
git add Sources/AIMeterCore Sources/AIMeterApp/Views/UsageVisualStyle.swift Tests/AIMeterCoreTests/AppPresentationTests.swift
git commit -m "refactor: centralize provider display names"
```

### 任务 2：统一主应用、通知与 Widget 当前文案

**文件：**
- 修改：`Tests/AIMeterAppTests/ServiceAccountSettingsTests.swift`
- 修改：`Tests/AIMeterAppTests/SettingsStructureTests.swift`
- 修改：`Tests/AIMeterCoreTests/ClaudeAccountReaderTests.swift`
- 修改：必要的 Widget/App 展示测试
- 修改：本计划文件结构中列出的 App/Core/Widget 源文件

- [x] **步骤 1：更新并新增当前文案失败断言**

至少锁定：

```swift
#expect(model.settingsMessage == "Claude Code account connected.")
#expect(model.settingsMessage == "Claude Code workspace setup could not be opened.")
#expect(status.accountLabel == "Claude Code account")
```

为 Widget 名称消费者增加可执行断言，确保 Medium/Large 的辅助功能或展示模型读取 `WidgetProvider.displayName`；内部 raw value 仍断言为 `claude` / `codex`。

- [x] **步骤 2：运行定向测试验证失败**

```bash
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-provider-names-ui \
  bash scripts/test.sh --filter ServiceAccountSettingsTests
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-provider-names-ui \
  bash scripts/test.sh --filter SettingsStructureTests
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-provider-names-ui \
  bash scripts/test.sh --filter ClaudeAccountReaderTests
```

预期：旧简称断言或实现使至少一项失败。

- [x] **步骤 3：替换当前用户可见名称**

实施原则：

- 标题和辅助功能优先使用 `provider.displayName`；
- 通知直接使用 `event.provider.displayName`；
- Settings 分组标题与详情标题使用正式名称；
- `Claude CLI` 改为 `Claude Code CLI`，但执行命令仍为 `claude`；
- `Codex thread/activity` 改为 `OpenAI Codex thread/activity`；
- Widget 描述改为 `Claude Code, OpenAI Codex, and DeepSeek usage at a glance.`；
- Demo 账户标签改为 `Demo Claude Code account`；
- 不修改 `Open Claude Login.command`、`Open Codex Login.command`、`.claude`、`.codex`、枚举 raw value 或 JSON 字段。

对 `OpenAI Codex` 较长标题检查 `lineLimit(1)`、缩放或布局优先级；只在实际存在截断风险时做最小布局调整。

- [x] **步骤 4：运行全部 App/Core/Widget 定向套件**

```bash
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-provider-names-ui \
  bash scripts/test.sh --filter AppPresentationTests
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-provider-names-ui \
  bash scripts/test.sh --filter ServiceAccountSettingsTests
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-provider-names-ui \
  bash scripts/test.sh --filter SettingsStructureTests
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-provider-names-ui \
  bash scripts/test.sh --filter ClaudeAccountReaderTests
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-provider-names-ui \
  bash scripts/test.sh --filter WidgetLayoutPolicyTests
```

预期：全部通过，且 Package 编译证明主应用与 Widget 不再依赖重复名称扩展。

- [x] **步骤 5：提交用户界面检查点**

```bash
git add Sources Tests
git commit -m "feat: standardize provider names across the app"
```

### 任务 3：同步现行文档与需求证据

**文件：**
- 修改：`README.md`、`CHANGELOG.md`
- 修改：`docs/user-guide/*.md`
- 修改：`docs/architecture/overview.md`、`docs/architecture/repository-structure.md`
- 修改：`docs/security-and-privacy.md`
- 修改：`docs/development/testing.md`、`docs/development/release-process.md`
- 创建：`docs/development/2026-09-02-provider-visible-name-standardization.md`
- 修改：`docs/development/README.md`、`docs/README.md`、`docs/development/commit-history.md`、`docs/requirements-backlog.md`

- [x] **步骤 1：更新当前维护文档**

把描述 UI、Provider 名称和用户操作的简称更新为 `Claude Code` / `OpenAI Codex`。真实命令、类型名、源文件名、目录名、历史规格、历史计划和旧开发日志保持不变。

- [x] **步骤 2：记录兼容边界和 TDD 证据**

开发日志需包含：修改范围、明确未改内容、红测试、绿测试、完整测试、Release 构建、签名、哈希、真实 UI/辅助功能验收和提交号。

- [x] **步骤 3：检查文档入口与需求状态**

在文档索引和提交历史中加入本次条目；需求在真正完成前保持 `进行中`，只在全部验收后改为 `已完成`。

- [ ] **步骤 4：提交文档检查点**

```bash
git add README.md CHANGELOG.md docs
git commit -m "docs: document provider display name standardization"
```

### 任务 4：完整验证、安装验收与合并

**文件：**
- 修改：`docs/development/2026-09-02-provider-visible-name-standardization.md`
- 修改：`docs/development/commit-history.md`
- 修改：`docs/requirements-backlog.md`

- [ ] **步骤 1：运行完整自动化测试**

```bash
AIMETER_TEST_BUILD_DIR=/private/tmp/ai-meter-provider-names-full bash scripts/test.sh
```

预期：所有测试通过，0 failures。

- [ ] **步骤 2：构建并严格验证 Release**

```bash
bash scripts/build-app.sh
codesign --verify --deep --strict --verbose=2 "dist/AI Token Meter.app"
bash scripts/verify-widget-bundle.sh "dist/AI Token Meter.app"
```

预期：Release 构建、主应用和 Widget 签名结构全部通过。

- [ ] **步骤 3：安全安装候选版本**

退出正在运行的 AI Token Meter；把现有 `/Applications/AI Token Meter.app` 备份到 `/private/tmp`，再安装候选。对候选与安装版主可执行文件计算 SHA-256，必须一致；安装异常时恢复备份。

- [ ] **步骤 4：真实界面和辅助功能验收**

启动已安装应用并检查：

1. 菜单卡片朗读 `Claude Code`、`OpenAI Codex`、`DeepSeek`；
2. 浮动条三个 Logo 的辅助功能名称使用正式名称；
3. 详情页标题分别为 `Claude Code` 与 `OpenAI Codex`，长标题无截断；
4. Settings > Services 标题、账户状态和操作反馈使用正式名称；
5. Widget 当前说明/辅助功能使用正式名称；
6. 内部 CLI 与缓存仍正常刷新。

- [ ] **步骤 5：完成日志、台账与提交**

填入真实测试数量、签名身份、哈希、验收时间和提交号；把 `REQ-20260902-011` 标记为 `已完成` 后提交：

```bash
git add docs
git commit -m "docs: verify provider display names"
```

- [ ] **步骤 6：代码审查与主分支集成**

执行提交范围、差异、命名兼容和测试证据自审；在主工作区确认无未提交改动后，将 `codex/provider-visible-names` 快进合并到 `main`，再次运行完整测试并确认工作区干净。
