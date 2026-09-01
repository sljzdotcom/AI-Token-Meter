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

        #expect(model.settingsMessage == "Claude workspace setup could not be opened.")
        #expect(model.settingsMessageKind == .claudeWorkspace)
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
