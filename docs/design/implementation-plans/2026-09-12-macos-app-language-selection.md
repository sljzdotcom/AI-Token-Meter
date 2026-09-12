# macOS 三语言即时切换实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 executing-plans 在当前长期开发会话逐任务实现；每项生产改动必须先有可观察的失败测试，并保留红绿证据。

**目标：** 在macOS Appearance增加默认English、可选简体中文与繁體中文的语言设置，并让应用自有界面即时、持久且一致地切换。

**架构：** 用`AppLanguage`和注入`UserDefaults`的存储作为唯一语言状态；静态SwiftUI文案由显式Locale和三份Bundle资源解析，动态/AppKit文案由`AppLocalizer`按同一语言格式化。`AppModel`发布语言变化，MenuBarExtra、Settings及FloatingPanelController创建的独立HostingView全部读取该状态。

**技术栈：** Swift 6、SwiftUI、AppKit、Foundation Bundle localization、Swift Testing、Swift Package Manager。

---

## 文件结构

- 创建`Sources/AIMeterApp/System/AppLanguage.swift`：语言枚举、稳定存储值、Locale和偏好存储。
- 创建`Sources/AIMeterApp/System/AppLocalizer.swift`：显式语言资源查找、参数化文本、日期与数字格式。
- 创建`Sources/AIMeterApp/System/SettingsNotice.swift`：Settings瞬时消息的语义状态与本地化参数。
- 创建`Sources/AIMeterApp/Resources/{en,zh-Hans,zh-Hant}.lproj/Localizable.strings`：三份键集合完全一致的应用文本。
- 创建`Tests/AIMeterAppTests/AppLanguageTests.swift`：默认、未知值、持久化和模型更新。
- 创建`Tests/AIMeterAppTests/AppLocalizationTests.swift`：资源完整性、固定翻译、变量语序和格式。
- 修改`Package.swift`：声明默认本地化并打包三份资源。
- 修改`Sources/AIMeterApp/AppModel.swift`：持有语言、保存设置、用语义Notice替代已存最终英文句子。
- 修改`Sources/AIMeterApp/AIMeterApp.swift`、`Sources/AIMeterApp/System/FloatingPanelController.swift`：向三个SwiftUI根入口注入显式Locale。
- 修改`Sources/AIMeterApp/Views/AppearanceSettingsView.swift`、`SettingsView.swift`、`SettingsTab.swift`：新增Picker并本地化五页签。
- 修改`Sources/AIMeterApp/Views/*.swift`中的应用自有可见文案：Settings、菜单栏、浮动条、四Provider详情、更新状态及辅助功能。
- 修改`Sources/AIMeterApp/System/NotificationService.swift`：按当前语言生成通知标题与正文。
- 修改相关测试与维护文档：结构、渲染、通知、项目状态、CHANGELOG、开发记录和需求台账。

### 任务1：语言偏好与模型状态

**文件：**
- 创建：`Sources/AIMeterApp/System/AppLanguage.swift`
- 创建：`Tests/AIMeterAppTests/AppLanguageTests.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`

- [x] **步骤1：编写失败的偏好测试**

测试必须直接构造隔离`UserDefaults`，断言缺失/未知值为English，三个稳定值往返不变，并断言`AppModel.setAppLanguage(.simplifiedChinese)`后立即可观察且重建模型仍保持：

```swift
@Test func languageDefaultsToEnglishAndPersistsExplicitChoice() throws {
    let defaults = try #require(UserDefaults(suiteName: "AppLanguage-\(UUID())"))
    #expect(AppLanguagePreferenceStore(defaults: defaults).load() == .english)
    defaults.set("unknown", forKey: AppLanguagePreferenceStore.key)
    #expect(AppLanguagePreferenceStore(defaults: defaults).load() == .english)
    AppLanguagePreferenceStore(defaults: defaults).save(.traditionalChinese)
    #expect(AppLanguagePreferenceStore(defaults: defaults).load() == .traditionalChinese)
}
```

- [x] **步骤2：运行测试确认正确红灯**

运行：`swift test --filter AppLanguageTests`

预期：编译失败，明确缺少`AppLanguage`和`AppLanguagePreferenceStore`；不是夹具或环境错误。

- [x] **步骤3：实现最小语言类型与存储**

实现固定顺序和自称，不读取系统Locale决定默认值：

```swift
enum AppLanguage: String, CaseIterable, Identifiable {
    case english = "en"
    case simplifiedChinese = "zh-Hans"
    case traditionalChinese = "zh-Hant"
    var id: Self { self }
    var locale: Locale { Locale(identifier: rawValue) }
    var displayName: String { switch self {
        case .english: "English"
        case .simplifiedChinese: "简体中文"
        case .traditionalChinese: "繁體中文"
    } }
}
```

存储`load()`只接受三个raw value；其他情况返回`.english`。`AppModel`初始化`appLanguage`并提供只在值变化时保存的`setAppLanguage`。

- [x] **步骤4：运行测试确认转绿并提交**

运行：`swift test --filter AppLanguageTests`

提交：`feat: add macOS app language preference`

### 任务2：显式本地化资源与完整性

**文件：**
- 创建：`Sources/AIMeterApp/System/AppLocalizer.swift`
- 创建：`Sources/AIMeterApp/Resources/en.lproj/Localizable.strings`
- 创建：`Sources/AIMeterApp/Resources/zh-Hans.lproj/Localizable.strings`
- 创建：`Sources/AIMeterApp/Resources/zh-Hant.lproj/Localizable.strings`
- 创建：`Tests/AIMeterAppTests/AppLocalizationTests.swift`
- 修改：`Package.swift`

- [x] **步骤1：编写失败的资源与格式测试**

测试加载三张表，要求键集合相同、值非空，并核对：`Appearance → 外观/外觀`、`Floating Strip → 悬浮条/懸浮條`、Provider品牌名保持、带Provider与百分比的通知可按中文语序生成。另用固定日期和`12_345`证明English与中文格式器由显式Locale控制。

- [x] **步骤2：运行测试确认资源缺失红灯**

运行：`swift test --filter AppLocalizationTests`

预期：缺少`AppLocalizer`或三份资源导致失败。

- [x] **步骤3：实现资源打包和显式查找**

`Package.swift`增加`defaultLocalization: "en"`，应用target处理三个`.lproj`目录。`AppLocalizer`从`Bundle.module`中按语言目录创建Bundle，缺键回退English，English缺键返回键本身：

```swift
struct AppLocalizer {
    let language: AppLanguage
    func text(_ key: String, _ arguments: CVarArg...) -> String {
        let format = localizedBundle.localizedString(forKey: key, value: nil, table: "Localizable")
        return String(format: format, locale: language.locale, arguments: arguments)
    }
}
```

三份资源以English语义句作为稳定key，动态句使用`%@`、`%lld`等完整模板，不在视图中拼接中文语序。

- [x] **步骤4：运行完整性测试确认转绿并提交**

运行：`swift test --filter AppLocalizationTests`

提交：`feat: add macOS localization resources`

### 任务3：根视图传播与Appearance选择器

**文件：**
- 修改：`Sources/AIMeterApp/AIMeterApp.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 修改：`Sources/AIMeterApp/System/FloatingPanelController.swift`
- 修改：`Sources/AIMeterApp/Views/AppearanceSettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/SettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/SettingsTab.swift`
- 修改：`Tests/AIMeterAppTests/SettingsStructureTests.swift`
- 测试：`Tests/AIMeterAppTests/AppLanguageTests.swift`

- [x] **步骤1：编写失败的结构和即时传播测试**

结构测试确认`Language`位于`Display font`之前，Picker顺序为English、简体中文、繁體中文。模型测试为语言变化处理器计数，并确认同值写入不重复。屏幕门禁创建已存在FloatingPanelController，切换语言后核对根视图使用最新Locale而不重建controller。

- [x] **步骤2：运行专项确认旧Appearance缺少Language**

运行：`swift test --filter 'AppLanguageTests|SettingsStructureTests'`

预期：结构断言和语言传播断言失败。

- [x] **步骤3：实现三个根入口和Picker**

在MenuBarPanel、SettingsView及FloatingPanelController的HostingView根部应用：

```swift
.environment(\.locale, model.appLanguage.locale)
```

Appearance首项绑定`model.appLanguage`并调用`setAppLanguage`；选项显示`language.displayName`。`SettingsTab.title(language:)`通过`AppLocalizer`返回当前语言标题。

- [x] **步骤4：运行专项确认转绿并提交**

运行：`AI_METER_SCREEN_TESTS=1 swift test --filter 'AppLanguageTests|SettingsStructureTests'`

提交：`feat: add language picker to macOS Appearance`

### 任务4：Settings、菜单栏与浮动条静态界面

**文件：**
- 修改：`Sources/AIMeterApp/Views/AppearanceSettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripSettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripDisplaySettings.swift`
- 修改：`Sources/AIMeterApp/Views/MonitoringSettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/ServicesSettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/AboutSettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/SoftwareUpdateSettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/MenuBarPanel.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift`
- 测试：`Tests/AIMeterAppTests/AppLocalizationTests.swift`
- 测试：`Tests/AIMeterAppTests/SettingsStructureTests.swift`

- [x] **步骤1：扩展失败测试覆盖五页签主要文案**

为每种语言核对五个页签、所有Settings分组、常用操作、浮动条右键菜单、菜单栏刷新/设置/退出与折叠辅助功能。源清单测试枚举上述文件中允许保留的品牌词，拒绝未登记的用户可见英文literal。

- [x] **步骤2：运行专项确认中文资源尚未覆盖红灯**

运行：`swift test --filter 'AppLocalizationTests|SettingsStructureTests'`

- [x] **步骤3：迁移静态文案**

SwiftUI静态`Text`/`Label`/`Section`保持稳定English key并依赖显式Locale；AppKit菜单调用`model.localizer.text(...)`。所有含动态值的句子改用完整资源模板。系统字体列表的字体名称不翻译。

- [x] **步骤4：运行专项确认转绿并提交**

运行：`swift test --filter 'AppLocalizationTests|SettingsStructureTests|FloatingStripRenderingTests'`

提交：`feat: localize macOS settings and meter controls`

### 任务5：Provider详情、动态状态与瞬时消息

**文件：**
- 创建：`Sources/AIMeterApp/System/SettingsNotice.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 修改：`Sources/AIMeterApp/Views/ProviderCard.swift`
- 修改：`Sources/AIMeterApp/Views/UsageRing.swift`
- 修改：`Sources/AIMeterApp/Views/ClaudeDetailPresentation.swift`
- 修改：`Sources/AIMeterApp/Views/ClaudeDetailView.swift`
- 修改：`Sources/AIMeterApp/Views/CodexDetailView.swift`
- 修改：`Sources/AIMeterApp/Views/CodexResetCreditsView.swift`
- 修改：`Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift`
- 修改：`Sources/AIMeterApp/Views/GeminiDetailView.swift`
- 修改：`Sources/AIMeterApp/Views/GeminiInstallationGuide.swift`
- 修改：`Sources/AIMeterApp/Views/ServiceAccountStatusView.swift`
- 测试：`Tests/AIMeterAppTests/AppLocalizationTests.swift`
- 测试：现有Provider详情与ServiceAccount测试。

- [x] **步骤1：编写失败的动态文本测试**

覆盖四Provider fresh/cached/unavailable状态、Antigravity四额度标签、剩余比例、重置时间、Claude/Codex本机统计、DeepSeek历史、账户连接/安装/登录结果。先断言同一个语义Notice在切换语言后重新渲染，而不是保存旧语言字符串。

- [x] **步骤2：运行专项确认当前动态英文不能切换**

运行：`swift test --filter 'AppLocalizationTests|GeminiDetailPanelLayoutTests|ClaudeDetailPresentationTests|ServiceAccountSettingsTests'`

- [x] **步骤3：实现语义Notice和动态本地化**

`SettingsNotice`用枚举关联Provider等参数，并在读取`AppModel.settingsMessage`时按当前`AppLocalizer`生成。详情presentation方法接收`AppLocalizer`或`AppLanguage`；额度数值继续保持原计算，只本地化标签和语句。缓存/错误的外部原始诊断按规格保持原文。

- [x] **步骤4：运行Provider专项确认转绿并提交**

运行：`swift test --filter 'AppLocalizationTests|GeminiDetailPanelLayoutTests|ClaudeDetailPresentationTests|ServiceAccountSettingsTests|GeminiAvailabilityTests'`

提交：`feat: localize macOS provider details and status`

### 任务6：通知、日期数字与辅助功能

**文件：**
- 修改：`Sources/AIMeterApp/System/NotificationService.swift`
- 修改：`Sources/AIMeterCore/Presentation/AppPresentation.swift`或在App层增加等价本地化presentation，保持Core协议文本独立。
- 修改：所有含`.formatted`及动态`accessibilityLabel`的AIMeterApp视图。
- 修改：`Tests/AIMeterAppTests/NotificationServiceTests.swift`
- 修改：`Tests/AIMeterAppTests/MenuBarMeterIconTests.swift`
- 测试：`Tests/AIMeterAppTests/AppLocalizationTests.swift`

- [x] **步骤1：编写失败的通知和格式测试**

用固定ThresholdEvent断言三语言通知标题/正文；用固定日期、百分比和计数断言三语言格式；核对菜单栏、圆环、拖动与详情辅助功能标签。Core层继续保留业务数据，不把中文写入协议模型。

- [x] **步骤2：运行专项确认当前通知与辅助功能固定英文**

运行：`swift test --filter 'NotificationServiceTests|MenuBarMeterIconTests|AppLocalizationTests'`

- [x] **步骤3：注入语言并迁移动态字符串**

`NotificationService`接收闭包`language: () -> AppLanguage`，发送时创建当前localizer。日期/数字统一由`AppLocalizer`格式方法生成；辅助功能使用完整模板，不拼接英文片段。

- [x] **步骤4：运行专项确认转绿并提交**

运行：`swift test --filter 'NotificationServiceTests|MenuBarMeterIconTests|AppLocalizationTests|VisualSystemTests'`

提交：`feat: localize macOS notifications and accessibility`

### 任务7：完整验证、文档与整合

**文件：**
- 修改：`CHANGELOG.md`
- 创建：`docs/development/2026-09-12-macos-app-language-selection.md`
- 修改：`docs/development/README.md`
- 修改：`docs/project-status.md`
- 修改：`docs/requirements-backlog.md`
- 修改：本计划复选框。

- [x] **步骤1：运行资源和界面专项**

运行：

```bash
AI_METER_SCREEN_TESTS=1 swift test --filter 'AppLanguageTests|AppLocalizationTests|SettingsStructureTests|FloatingStripRenderingTests|GeminiDetailPanelLayoutTests|NotificationServiceTests'
```

- [x] **步骤2：运行完整项目门禁**

运行：

```bash
scripts/test.sh
scripts/build-app.sh
scripts/check-docs.sh
git diff --check
```

验证Release App内包含三个`.lproj/Localizable.strings`，默认English，且不包含凭据、真实账号数据或新的网络访问。

- [x] **步骤3：完成独立审查并修复发现**

审查重点：缺翻译、中文语序、运行时Locale传播、旧瞬时消息、Bundle回退、日期数字协议污染、辅助功能、资源打包和Windows非目标变化。每条发现补失败回归后修复，最终报告Critical/Important/Minor总数。

- [ ] **步骤4：记录证据并本地整合**

开发记录写入红绿证据、测试数量、构建产物资源、审查结果和提交。需求达到验收后标记已完成并记录本地main合并SHA；没有用户独立发布指示时，不创建版本、标签、Release或修改更新源。

  - [x] 记录最终修复头、完整门禁、Release资源与审查`0/0/0`证据。
  - [ ] 完成本地`main`整合，补记合并SHA并将需求标记为已完成。
