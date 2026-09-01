import AppKit
import Testing
@testable import AIMeterApp

@Suite("Floating panel presentation policy")
struct FloatingPanelPresentationPolicyTests {
    @Test("Floating strip remains below ordinary app windows")
    func stripLevel() {
        let desktopIcons = Int(CGWindowLevelForKey(.desktopIconWindow))
        let level = FloatingPanelPresentationPolicy.level(for: .strip)

        #expect(level.rawValue > desktopIcons)
        #expect(level.rawValue < NSWindow.Level.normal.rawValue)
    }

    @Test("Detail panels float above ordinary application windows")
    func detailLevel() {
        let level = FloatingPanelPresentationPolicy.level(for: .detail)

        #expect(level == .floating)
        #expect(level.rawValue > NSWindow.Level.normal.rawValue)
    }

    @Test("Desktop panels join ordinary Spaces without entering full screen")
    func collectionBehavior() {
        let behavior = FloatingPanelPresentationPolicy.collectionBehavior

        #expect(behavior.contains(.canJoinAllSpaces))
        #expect(behavior.contains(.stationary))
        #expect(behavior.contains(.ignoresCycle))
        #expect(!behavior.contains(.fullScreenAuxiliary))
    }

    @Test("Role-specific levels share the ordinary Spaces policy")
    @MainActor
    func appliesRoleSpecificLevels() {
        let strip = NSPanel()
        let detail = NSPanel()

        FloatingPanelPresentationPolicy.apply(to: strip, role: .strip)
        FloatingPanelPresentationPolicy.apply(to: detail, role: .detail)

        #expect(strip.level.rawValue < NSWindow.Level.normal.rawValue)
        #expect(detail.level == .floating)
        #expect(detail.level.rawValue > NSWindow.Level.normal.rawValue)
        #expect(strip.collectionBehavior == FloatingPanelPresentationPolicy.collectionBehavior)
        #expect(detail.collectionBehavior == strip.collectionBehavior)
    }
}
