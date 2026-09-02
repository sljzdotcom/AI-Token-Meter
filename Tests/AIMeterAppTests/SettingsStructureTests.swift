import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("Settings information architecture")
struct SettingsStructureTests {
    @Test("Defines four ordered top tabs")
    func orderedTabs() {
        #expect(SettingsTab.allCases == [.appearance, .monitoring, .services, .about])
        #expect(
            SettingsTab.allCases.map(\.title) == [
                "Appearance",
                "Monitoring",
                "Services",
                "About",
            ]
        )
        #expect(Set(SettingsTab.allCases.map(\.systemImage)).count == 4)
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
}

private final class InMemorySecretStore: SecretStore, @unchecked Sendable {
    func read() throws -> String? { nil }
    func save(_ secret: String) throws {}
    func delete() throws {}
}

private struct MissingExecutableLocator: ExecutableLocating {
    func locate(named name: String) -> URL? { nil }
}
