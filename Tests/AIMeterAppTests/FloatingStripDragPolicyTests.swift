import CoreGraphics
import Testing
@testable import AIMeterApp

@Suite("Floating strip cross-display drag")
struct FloatingStripDragPolicyTests {
    @Test func horizontalTranslationAlwaysMovesFrame() {
        let result = FloatingStripDragPolicy.translated(
            CGRect(x: 10, y: 80, width: 60, height: 200),
            by: CGSize(width: -900, height: 20))
        #expect(result.origin == CGPoint(x: -890, y: 60))
    }

    @Test func pointerTargetsFullScreenAndNearestGap() {
        let screens: [String: CGRect] = [
            "left": CGRect(x: -1000, y: 0, width: 1000, height: 800),
            "primary": CGRect(x: 0, y: 0, width: 1200, height: 900),
            "above": CGRect(x: 100, y: 1000, width: 1200, height: 800),
        ]
        #expect(FloatingStripDragPolicy.target(at: CGPoint(x: -10, y: 799), screens: screens) == "left")
        #expect(FloatingStripDragPolicy.target(at: CGPoint(x: 200, y: 1001), screens: screens) == "above")
        #expect(FloatingStripDragPolicy.target(at: CGPoint(x: 200, y: 980), screens: screens) == "above")
        #expect(FloatingStripDragPolicy.target(at: CGPoint(x: -10, y: 300), screens: screens, pinned: "primary") == "primary")
        #expect(FloatingStripDragPolicy.target(at: .zero, screens: [:]) == nil)
    }
}
