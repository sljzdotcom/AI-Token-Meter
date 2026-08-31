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

    @Test("Drag exclusions remain fixed-size and centered in a scaled translated shape")
    func dragRegionUsesFixedCenteredProviderFrames() {
        let rect = CGRect(x: 100, y: 200, width: 216, height: 712)
        let right = FloatingStripDragShape(edge: .right).path(in: rect)

        #expect(right.contains(CGPoint(x: 272, y: 280), eoFill: true))
        #expect(!right.contains(CGPoint(x: 208, y: 484), eoFill: true))
        #expect(!right.contains(CGPoint(x: 208, y: 556), eoFill: true))
        #expect(!right.contains(CGPoint(x: 208, y: 628), eoFill: true))
        #expect(right.contains(CGPoint(x: 208, y: 453), eoFill: true))
        #expect(right.contains(CGPoint(x: 208, y: 515), eoFill: true))
    }

    @Test("Every provider exclusion keeps adjacent glass draggable")
    func providerExclusionBoundariesLeaveAdjacentGlassDraggable() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
        let right = FloatingStripDragShape(edge: .right).path(in: rect)

        for (top, center, bottom) in [(76, 106, 136), (148, 178, 208), (220, 250, 280)] {
            #expect(!right.contains(CGPoint(x: 54, y: top + 1), eoFill: true))
            #expect(!right.contains(CGPoint(x: 54, y: bottom - 1), eoFill: true))
            #expect(right.contains(CGPoint(x: 54, y: top - 1), eoFill: true))
            #expect(right.contains(CGPoint(x: 54, y: bottom + 1), eoFill: true))
            #expect(!right.contains(CGPoint(x: 25, y: center), eoFill: true))
            #expect(!right.contains(CGPoint(x: 83, y: center), eoFill: true))
            #expect(right.contains(CGPoint(x: 23, y: center), eoFill: true))
            #expect(right.contains(CGPoint(x: 85, y: center), eoFill: true))
        }
    }
}
