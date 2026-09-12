import AppKit
import SwiftUI
import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("Settings information architecture")
struct SettingsStructureTests {
    @Test("Defines five ordered top tabs")
    func orderedTabs() {
        #expect(SettingsTab.allCases == [.appearance, .floatingStrip, .monitoring, .services, .about])
        #expect(
            SettingsTab.allCases.map(\.title) == [
                "Appearance",
                "Floating Strip",
                "Monitoring",
                "Services",
                "About",
            ]
        )
        #expect(Set(SettingsTab.allCases.map(\.systemImage)).count == 5)
    }

    @Test("Appearance offers language before font with fixed native-language choices")
    func languagePickerPlacement() throws {
        let root = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let source = try String(contentsOf: root.appending(path:
            "Sources/AIMeterApp/Views/AppearanceSettingsView.swift"), encoding: .utf8)
        let language = try #require(source.range(of: "\"Language\""))
        let font = try #require(source.range(of: "\"Display font\""))
        #expect(language.lowerBound < font.lowerBound)
        #expect(source.contains("model.setAppLanguage"))
        #expect(source.contains("AppLanguage.allCases"))
        #expect(source.contains("language.displayName"))
        #expect(AppLanguage.allCases.map(\.displayName) == ["English", "简体中文", "繁體中文"])
    }

    @Test("Settings tabs resolve their title in the selected language")
    func localizedTabTitles() {
        #expect(SettingsTab.allCases.map { $0.title(language: .simplifiedChinese) }
                == ["外观", "悬浮条", "监测", "服务", "关于"])
        #expect(SettingsTab.allCases.map { $0.title(language: .traditionalChinese) }
                == ["外觀", "懸浮條", "監測", "服務", "關於"])
        #expect(SettingsTab.allCases.map { $0.title(language: .english) }
                == ["Appearance", "Floating Strip", "Monitoring", "Services", "About"])
    }

    @Test("Refresh interval controls translate in place without replacing an unsaved draft")
    @MainActor
    func refreshIntervalControlsUseSelectedLanguage() async throws {
        let host = NSHostingView(rootView: RefreshIntervalEditor(seconds: 300, save: { _ in false })
            .environment(\.locale, AppLanguage.english.locale))
        host.frame = NSRect(x: 0, y: 0, width: 480, height: 150)
        host.layoutSubtreeIfNeeded()
        let editor = try #require(findIntervalEditor(in: host))
        editor.input.stringValue = "draft"
        for (language, label, apply) in [(AppLanguage.simplifiedChinese, "刷新间隔（秒）", "应用"),
                                          (.traditionalChinese, "重新整理間隔（秒）", "套用"),
                                          (.english, "Refresh interval in seconds", "Apply")] {
            host.rootView = RefreshIntervalEditor(seconds: 300, save: { _ in false })
                .environment(\.locale, language.locale)
            host.layoutSubtreeIfNeeded()
            try await Task.sleep(for: .milliseconds(30))
            #expect(editor.input.accessibilityLabel() == label)
            #expect(editor.input.stringValue == "draft")
            let buttons = editor.subviews.flatMap { $0.subviews }.compactMap { $0 as? NSButton }
            #expect(buttons.map(\.title) == [apply])
            #expect(findIntervalEditor(in: host) === editor)
        }
    }

    @MainActor
    private func findIntervalEditor(in view: NSView) -> RefreshIntervalEditorView? {
        if let editor = view as? RefreshIntervalEditorView { return editor }
        return view.subviews.lazy.compactMap { findIntervalEditor(in: $0) }.first
    }

    @Test("The actual floating-strip menu uses the latest language while preserving actions")
    @MainActor
    func contextMenuLocalization() throws {
        let suite = "SettingsStructureTests.ContextMenu.\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let model = AppModel(defaults: defaults, secretStore: InMemorySecretStore(), widgetSnapshotPublisher: nil)
        let controller = FloatingPanelController(model: model)
        defer { controller.close() }
        let cases: [(AppLanguage, [String])] = [
            (.english, ["Refresh now", "Hide for 1 hour", "Settings…", "Quit AI Token Meter"]),
            (.simplifiedChinese, ["立即刷新", "隐藏 1 小时", "设置…", "退出 AI Token Meter"]),
            (.traditionalChinese, ["立即重新整理", "隱藏 1 小時", "設定…", "結束 AI Token Meter"]),
        ]
        for (language, titles) in cases {
            model.setAppLanguage(language)
            let menu = controller.makeContextMenu()
            let items = menu.items.filter { !$0.isSeparatorItem }
            #expect(items.map(\.title) == titles)
            #expect(items.map { $0.action.map(NSStringFromSelector) } == ["refreshFromMenu", "hideFromMenu", "settingsFromMenu", "quitFromMenu"])
            #expect(items.allSatisfy { $0.target === controller })
            #expect(menu.items[2].isSeparatorItem)
        }
    }

    @Test("Rendered Settings and menu surfaces translate after an in-place language change",
          .enabled(if: ProcessInfo.processInfo.environment["AI_METER_SCREEN_TESTS"] == "1"))
    @MainActor
    func hostedSurfacesTranslateInPlace() async throws {
        NSApplication.shared.accessibilitySetValue(true, forAttribute: NSAccessibility.Attribute(rawValue: "AXEnhancedUserInterface"))
        let suite = "SettingsStructureTests.Hosted.\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let model = AppModel(defaults: defaults, secretStore: InMemorySecretStore(), widgetSnapshotPublisher: nil)
        model.setFloatingStripVisible(false)
        let coordinator = SoftwareUpdateCoordinator(engine: LocalizationUpdateEngine(), currentVersion: "1.2.3", currentBuild: "7")
        defer { coordinator.stop() }
        let views: [(AnyView, [[String]])] = [
            (AnyView(AppearanceSettingsView(model: model)), [["Display", "Language", "Display font"], ["显示", "语言", "显示字体"], ["顯示", "語言", "顯示字體"]]),
            (AnyView(FloatingStripSettingsView(model: model)), [["Content and Size", "Screen and Position", "Behavior"], ["内容与尺寸", "屏幕与位置", "行为"], ["內容與尺寸", "螢幕與位置", "行為"]]),
            (AnyView(MonitoringSettingsView(model: model)), [["Refresh interval", "Usage alerts at 70% and 90%"], ["刷新间隔", "用量达到 70% 和 90% 时提醒"], ["重新整理間隔", "用量達到 70% 和 90% 時提醒"]]),
            (AnyView(ServicesSettingsView(model: model, pendingAPIKey: .constant(""))), [["Authorize Usage Workspace", "Balance baseline", "Save API Key"], ["授权用量工作区", "余额基准", "保存 API Key"], ["授權用量工作區", "餘額基準", "儲存 API Key"]]),
            (AnyView(AboutSettingsView(updateCoordinator: coordinator)), [["Privacy", "Software Update", "Check for Updates", "Update Now"], ["隐私", "软件更新", "检查更新", "立即更新"], ["隱私", "軟體更新", "檢查更新", "立即更新"]]),
            (AnyView(MenuBarPanel(model: model)), [["Show Floating Strip Now", "Waiting for first refresh"], ["立即显示悬浮条", "等待首次刷新"], ["立即顯示懸浮條", "等待首次重新整理"]]),
        ]
        for (view, expectations) in views {
            let host = NSHostingView(rootView: AppLanguageRoot(model: model) { view })
            let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 800, height: 1400),
                                  styleMask: [.borderless], backing: .buffered, defer: false)
            window.isReleasedWhenClosed = false
            window.contentView = host
            window.orderFrontRegardless()
            for (index, language) in AppLanguage.allCases.enumerated() {
                model.setAppLanguage(language)
                host.layoutSubtreeIfNeeded()
                try await Task.sleep(for: .milliseconds(80))
                host.layoutSubtreeIfNeeded()
                try assertHostedRendering(host)
                let labels = hostedAccessibilityStrings(host).joined(separator: "\n")
                for expected in expectations[index] {
                    #expect(labels.contains(expected), "Missing \(language.rawValue) label: \(expected). Got: \(labels)")
                    try assertHostedLabelFits(expected, in: host)
                }
                #expect(host.bounds.size == window.contentLayoutRect.size)
                #expect(window.contentView === host)
            }
            window.close()
        }
    }

    @Test("Routes settings messages to their owning tab")
    func messageRouting() {
        #expect(SettingsTab.monitoring.accepts(.launchAtLogin))
        #expect(SettingsTab.services.accepts(.claudeWorkspace))
        #expect(SettingsTab.services.accepts(.claudeAuthentication))
        #expect(SettingsTab.services.accepts(.codexAuthentication))
        #expect(SettingsTab.services.accepts(.deepSeekCredential))
        #expect(!SettingsTab.appearance.accepts(.deepSeekCredential))
        #expect(!SettingsTab.about.accepts(.launchAtLogin))
    }

    @Test("Floating strip context menu opens its owning tab")
    func floatingStripContextMenuRouting() throws {
        let root = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let source = try String(
            contentsOf: root.appending(path: "Sources/AIMeterApp/System/FloatingPanelController.swift"),
            encoding: .utf8
        )

        #expect(source.contains("settingsFromMenu() { model.requestSettings(.floatingStrip) }"))
    }

    @Test("A detail recovery request keeps Services selected while Settings opens")
    @MainActor
    func servicesPresentationRequest() {
        let suiteName = "SettingsStructureTests.Route.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let model = AppModel(defaults: defaults, secretStore: InMemorySecretStore())
        let recorder = SettingsNotificationRecorder()
        let observer = NotificationCenter.default.addObserver(
            forName: .aiMeterOpenSettings,
            object: nil,
            queue: nil
        ) { _ in
            recorder.record()
        }
        defer { NotificationCenter.default.removeObserver(observer) }

        model.requestSettings(.services)

        #expect(model.requestedSettingsTab == .services)
        #expect(recorder.count == 1)
    }

    @Test("Repeated requests for the same tab remain observable")
    @MainActor
    func repeatedTabRequest() {
        let suiteName = "SettingsStructureTests.Repeated.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let model = AppModel(defaults: defaults, secretStore: InMemorySecretStore())

        model.requestSettings(.floatingStrip)
        let firstRequest = model.settingsRequestSequence
        model.requestSettings(.floatingStrip)

        #expect(model.settingsRequestSequence == firstRequest + 1)
    }

    @Test("DeepSeek credential feedback targets the Services tab")
    @MainActor
    func deepSeekMessageDestination() {
        let suiteName = "SettingsStructureTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let model = AppModel(defaults: defaults, secretStore: InMemorySecretStore())

        model.saveDeepSeekAPIKey("   ")

        #expect(model.settingsMessage == "Enter a DeepSeek API Key first.")
        #expect(model.settingsMessageKind == .deepSeekCredential)
    }

    @Test("Claude workspace feedback targets the Services tab")
    @MainActor
    func claudeMessageDestination() {
        let suiteName = "SettingsStructureTests.Claude.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let launcher = ClaudeWorkspaceSetupLauncher(
            executableLocator: MissingExecutableLocator()
        )
        let model = AppModel(
            defaults: defaults,
            secretStore: InMemorySecretStore(),
            claudeWorkspaceSetupLauncher: launcher
        )

        model.openClaudeWorkspaceSetup()

        #expect(model.settingsMessage == "Claude Code workspace setup could not be opened.")
        #expect(model.settingsMessageKind == .claudeWorkspace)
    }

    @Test("About exposes both user initiated update actions")
    func softwareUpdateControlCopy() {
        #expect(SoftwareUpdateSettingsCopy.sectionTitle == "Software Update")
        #expect(SoftwareUpdateSettingsCopy.checkButton == "Check for Updates")
        #expect(SoftwareUpdateSettingsCopy.installButton == "Update Now")
        #expect(SoftwareUpdateSettingsCopy.currentVersion == "Current version")
        #expect(SoftwareUpdateSettingsCopy.lastChecked == "Last checked")
    }

    @Test("Settings injects the shared update coordinator into About")
    func softwareUpdateCoordinatorInjection() throws {
        let root = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let settingsSource = try String(
            contentsOf: root.appending(path: "Sources/AIMeterApp/Views/SettingsView.swift"),
            encoding: .utf8
        )
        let appSource = try String(
            contentsOf: root.appending(path: "Sources/AIMeterApp/AIMeterApp.swift"),
            encoding: .utf8
        )
        let delegateSource = try String(
            contentsOf: root.appending(path: "Sources/AIMeterApp/AppDelegate.swift"),
            encoding: .utf8
        )

        #expect(settingsSource.contains("AboutSettingsView(updateCoordinator: updateCoordinator)"))
        #expect(appSource.contains("updateCoordinator: appDelegate.softwareUpdateCoordinator"))
        #expect(delegateSource.contains("softwareUpdateCoordinator.stop()"))
    }

    @Test("Floating Strip owns strip controls while Appearance keeps display font")
    func floatingStripControls() throws {
        let root = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let appearanceSource = try String(
            contentsOf: root.appending(path: "Sources/AIMeterApp/Views/AppearanceSettingsView.swift"),
            encoding: .utf8
        )
        let source = try String(
            contentsOf: root.appending(path: "Sources/AIMeterApp/Views/FloatingStripSettingsView.swift"),
            encoding: .utf8
        )

        let comfortable = try #require(source.range(of: "Text(localizer.text(\"Comfortable\")).tag(FloatingStripDensity.comfortable)"))
        let compact = try #require(source.range(of: "Text(localizer.text(\"Compact\")).tag(FloatingStripDensity.compact)"))
        let mini = try #require(source.range(of: "Text(localizer.text(\"Mini\")).tag(FloatingStripDensity.mini)"))
        #expect(comfortable.lowerBound < compact.lowerBound)
        #expect(compact.lowerBound < mini.lowerBound)
        #expect(source.contains("Toggle(localizer.text(\"Automatically collapse floating strip\""))
        #expect(source.components(separatedBy: ".disabled(!model.stripPreferences.automaticallyCollapses)").count == 3)
        #expect(source.contains("FloatingStripDisplaySettings(model: model)"))
        #expect(appearanceSource.contains("Display font"))
        #expect(!appearanceSource.contains("Floating strip size"))
    }
}

private final class InMemorySecretStore: SecretStore, @unchecked Sendable {
    func read() throws -> String? { nil }
    func save(_ secret: String) throws {}
    func delete() throws {}
}

private struct MissingExecutableLocator: ExecutableLocating {
    func locate(named name: String) -> URL? { nil }
}

private final class SettingsNotificationRecorder: @unchecked Sendable {
    private let lock = NSLock()
    private var value = 0

    func record() { lock.withLock { value += 1 } }
    var count: Int { lock.withLock { value } }
}

@MainActor
private final class LocalizationUpdateEngine: SoftwareUpdateEngine {
    var eventHandler: ((SoftwareUpdateEvent) -> Void)?
    var canCheckForUpdates: Bool { true }
    func start() throws {}
    func checkForUpdateInformation() {}
    func presentAvailableUpdate() {}
    func stop() {}
}
