import Testing
@testable import AIMeterApp

@Suite("Settings presentation command")
struct SettingsPresentationCommandTests {
    @Test("Activates the menu bar app before opening settings")
    @MainActor
    func activationPrecedesOpening() {
        var events: [String] = []
        let command = SettingsPresentationCommand(
            activateApplication: { events.append("activate") },
            openSettings: { events.append("open") }
        )

        command.perform()

        #expect(events == ["activate", "open"])
    }
}
