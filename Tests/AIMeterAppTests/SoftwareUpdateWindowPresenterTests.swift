import Testing
@testable import AIMeterApp

@MainActor
@Suite("Software update window presentation")
struct SoftwareUpdateWindowPresenterTests {
    @Test("Starting installation hides only Settings and activates the app")
    func hidesSettingsBeforePresentingSparkle() {
        let settingsWindow = TestUpdateWindow(identifier: "com_apple_SwiftUI_Settings_window")
        let unrelatedWindow = TestUpdateWindow(identifier: "AIMeter.Unrelated")
        let unidentifiedWindow = TestUpdateWindow(identifier: nil)
        var activationCount = 0

        let presenter = SoftwareUpdateWindowPresenter(
            windows: { [settingsWindow, unrelatedWindow, unidentifiedWindow] },
            activate: { activationCount += 1 }
        )
        let engine = SparkleUpdateEngine(windowPresenter: presenter)

        engine.presentAvailableUpdate()

        #expect(settingsWindow.isHidden)
        #expect(!unrelatedWindow.isHidden)
        #expect(!unidentifiedWindow.isHidden)
        #expect(activationCount == 1)
    }
}

@MainActor
private final class TestUpdateWindow: SoftwareUpdatePresentableWindow {
    let softwareUpdateWindowIdentifier: String?
    private(set) var isHidden = false

    init(identifier: String?) {
        softwareUpdateWindowIdentifier = identifier
    }

    func hideForSoftwareUpdate() {
        isHidden = true
    }
}
