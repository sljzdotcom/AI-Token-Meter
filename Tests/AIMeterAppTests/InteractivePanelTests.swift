import AppKit
import Testing
@testable import AIMeterApp

@Suite("Interactive floating panel")
struct InteractivePanelTests {
    @Test("Borderless detail panel accepts keyboard focus without becoming main")
    @MainActor
    func keyWindowCapability() {
        let panel = InteractivePanel(
            contentRect: .zero,
            styleMask: [.borderless],
            backing: .buffered,
            defer: false
        )

        #expect(panel.canBecomeKey)
        #expect(!panel.canBecomeMain)
    }
}
