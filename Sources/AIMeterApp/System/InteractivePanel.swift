import AppKit

@MainActor
final class KeyboardAccessibleStripPanel: NSPanel {
    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { false }
}

@MainActor
final class InteractivePanel: NSPanel {
    var onFocusedControlChange: ((Bool) -> Void)?

    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { false }

    override func makeFirstResponder(_ responder: NSResponder?) -> Bool {
        let accepted = super.makeFirstResponder(responder)
        if accepted {
            let hasFocusedControl = responder != nil
                && responder !== self
                && responder !== contentView
            onFocusedControlChange?(hasFocusedControl)
        }
        return accepted
    }

    override func resignKey() {
        super.resignKey()
        onFocusedControlChange?(false)
    }
}
