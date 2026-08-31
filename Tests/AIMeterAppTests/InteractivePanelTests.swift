import AppKit
import AIMeterCore
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

    @Test("Nonactivating floating strip can accept keyboard focus when needed")
    @MainActor
    func stripKeyboardCapability() {
        let panel = KeyboardAccessibleStripPanel(
            contentRect: .zero,
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )

        #expect(panel.canBecomeKey)
        #expect(!panel.canBecomeMain)
        #expect(panel.styleMask.contains(.nonactivatingPanel))
    }

    @Test("Focused controls report interaction and resigning clears it")
    @MainActor
    func focusedControlReporting() {
        let panel = InteractivePanel(
            contentRect: NSRect(x: 0, y: 0, width: 200, height: 100),
            styleMask: [.borderless],
            backing: .buffered,
            defer: false
        )
        let root = NSView(frame: panel.contentView?.bounds ?? .zero)
        let control = FocusableTestView(frame: NSRect(x: 0, y: 0, width: 20, height: 20))
        root.addSubview(control)
        panel.contentView = root
        var values: [Bool] = []
        panel.onFocusedControlChange = { values.append($0) }

        #expect(panel.makeFirstResponder(control))
        panel.resignKey()

        #expect(values == [true, false])
    }

    @Test("A retired detail cannot update the current interaction state")
    @MainActor
    func interactionOwnershipFollowsSelection() throws {
        let session = FloatingDetailSession()
        session.present(.claude, autoHideAfter: .seconds(30))
        let claudeSelection = try #require(session.selectionID)
        session.present(.codex, autoHideAfter: .seconds(30))
        let codexSelection = try #require(session.selectionID)

        #expect(!FloatingDetailInteractionOwnership.accepts(
            renderedSelectionID: claudeSelection,
            currentSelectionID: codexSelection
        ))
        #expect(FloatingDetailInteractionOwnership.accepts(
            renderedSelectionID: codexSelection,
            currentSelectionID: codexSelection
        ))
        session.dismiss()
    }
}

private final class FocusableTestView: NSView {
    override var acceptsFirstResponder: Bool { true }
}
