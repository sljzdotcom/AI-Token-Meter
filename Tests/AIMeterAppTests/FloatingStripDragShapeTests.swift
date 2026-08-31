import AIMeterCore
import CoreGraphics
import Testing
@testable import AIMeterApp

@Suite("Floating strip drag region")
struct FloatingStripDragShapeTests {
    @Test("Glass background drags while provider buttons remain click-only")
    func dragRegionExcludesProviderButtons() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
        let right = FloatingStripDragShape(edge: .right).path(in: rect)

        #expect(right.contains(CGPoint(x: 86, y: 40), eoFill: true))
        #expect(right.contains(CGPoint(x: 54, y: 142), eoFill: true))
        #expect(!right.contains(CGPoint(x: 54, y: 106), eoFill: true))
        #expect(!right.contains(CGPoint(x: 54, y: 178), eoFill: true))
        #expect(!right.contains(CGPoint(x: 54, y: 250), eoFill: true))
        #expect(!right.contains(CGPoint(x: 20, y: 30), eoFill: true))
    }

    @Test("Left drag region mirrors the right region")
    func dragRegionMirrors() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
        let left = FloatingStripDragShape(edge: .left).path(in: rect)

        #expect(left.contains(CGPoint(x: 22, y: 40), eoFill: true))
        #expect(!left.contains(CGPoint(x: 54, y: 178), eoFill: true))
        #expect(!left.contains(CGPoint(x: 88, y: 30), eoFill: true))
    }
}
