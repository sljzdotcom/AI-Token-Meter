import AIMeterCore
import CoreGraphics
import Testing
@testable import AIMeterApp

@Suite("Floating strip pointer drag state")
struct FloatingStripPointerDragStateTests {
    @Test("Background pointer drag reports SwiftUI-style screen translation")
    func backgroundDragTranslation() {
        var state = FloatingStripPointerDragState()
        let size = CGSize(width: 108, height: 356)

        let began = state.begin(
            windowPoint: CGPoint(x: 86, y: 316),
            screenPoint: CGPoint(x: 1900, y: 700),
            panelSize: size,
            edge: .right
        )
        #expect(began)
        #expect(state.translation(to: CGPoint(x: 1870, y: 640)) == CGSize(width: -30, height: 60))
        let endedTranslation = state.end(at: CGPoint(x: 1870, y: 640))
        #expect(endedTranslation == CGSize(width: -30, height: 60))
        #expect(!state.isActive)
    }

    @Test("Provider rings do not begin a pointer drag")
    func providerRingDoesNotBeginDrag() {
        var state = FloatingStripPointerDragState()

        let began = state.begin(
            windowPoint: CGPoint(x: 54, y: 250),
            screenPoint: CGPoint(x: 1900, y: 700),
            panelSize: CGSize(width: 108, height: 356),
            edge: .right
        )
        #expect(!began)
        #expect(!state.isActive)
        #expect(state.translation(to: CGPoint(x: 1800, y: 600)) == nil)
    }

    @Test("Transparent shoulder does not begin a pointer drag")
    func transparentShoulderDoesNotBeginDrag() {
        var state = FloatingStripPointerDragState()

        let began = state.begin(
            windowPoint: CGPoint(x: 20, y: 326),
            screenPoint: CGPoint(x: 1900, y: 700),
            panelSize: CGSize(width: 108, height: 356),
            edge: .right
        )
        #expect(!began)
    }
}
