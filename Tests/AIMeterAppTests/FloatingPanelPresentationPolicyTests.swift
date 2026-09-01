import AppKit
import Testing
@testable import AIMeterApp

@Suite("Floating panel presentation policy")
struct FloatingPanelPresentationPolicyTests {
    @Test("Desktop panels remain below ordinary app windows")
    func desktopLevel() {
        let desktopIcons = Int(CGWindowLevelForKey(.desktopIconWindow))

        #expect(FloatingPanelPresentationPolicy.level.rawValue > desktopIcons)
        #expect(FloatingPanelPresentationPolicy.level.rawValue < NSWindow.Level.normal.rawValue)
    }

    @Test("Desktop panels join ordinary Spaces without entering full screen")
    func collectionBehavior() {
        let behavior = FloatingPanelPresentationPolicy.collectionBehavior

        #expect(behavior.contains(.canJoinAllSpaces))
        #expect(behavior.contains(.stationary))
        #expect(behavior.contains(.ignoresCycle))
        #expect(!behavior.contains(.fullScreenAuxiliary))
    }

    @Test("The same policy is applied to strip and detail panels")
    @MainActor
    func appliesToEveryPanel() {
        let first = NSPanel()
        let second = NSPanel()

        FloatingPanelPresentationPolicy.apply(to: first)
        FloatingPanelPresentationPolicy.apply(to: second)

        #expect(first.level == FloatingPanelPresentationPolicy.level)
        #expect(second.level == first.level)
        #expect(first.collectionBehavior == FloatingPanelPresentationPolicy.collectionBehavior)
        #expect(second.collectionBehavior == first.collectionBehavior)
    }
}
