import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("Floating strip layout")
struct FloatingStripLayoutTests {
    private let visible = CGRect(x: 100, y: 50, width: 1_200, height: 800)
    private let stripSize = CGSize(width: 108, height: 356)

    @Test("Places the island flush against either screen edge")
    func flushEdges() {
        let right = FloatingStripLayout.stripFrame(
            in: visible,
            size: stripSize,
            edge: .right,
            normalizedCenterY: 0.5
        )
        let left = FloatingStripLayout.stripFrame(
            in: visible,
            size: stripSize,
            edge: .left,
            normalizedCenterY: 0.5
        )

        #expect(right.maxX == 1_300)
        #expect(left.minX == 100)
        #expect(right.minY == 272)
        #expect(left.minY == 272)
    }

    @Test("Automatic chooses the nearest side while fixed choices win")
    func resolvesEdge() {
        #expect(FloatingStripLayout.resolvedEdge(
            preference: .automatic,
            current: .right,
            proposedMidX: 200,
            visibleFrame: visible
        ) == .left)
        #expect(FloatingStripLayout.resolvedEdge(
            preference: .automatic,
            current: .left,
            proposedMidX: 1_200,
            visibleFrame: visible
        ) == .right)
        #expect(FloatingStripLayout.resolvedEdge(
            preference: .right,
            current: .left,
            proposedMidX: 200,
            visibleFrame: visible
        ) == .right)
    }

    @Test("Clamps vertical placement and round-trips its normalized center")
    func verticalPlacement() {
        let bottom = FloatingStripLayout.stripFrame(
            in: visible,
            size: stripSize,
            edge: .right,
            normalizedCenterY: -2
        )
        let top = FloatingStripLayout.stripFrame(
            in: visible,
            size: stripSize,
            edge: .right,
            normalizedCenterY: 3
        )
        let quarter = FloatingStripLayout.stripFrame(
            in: visible,
            size: stripSize,
            edge: .right,
            normalizedCenterY: 0.25
        )

        #expect(bottom.minY == 50)
        #expect(top.maxY == 850)
        #expect(abs(FloatingStripLayout.normalizedCenterY(for: quarter, in: visible) - 0.25) < 0.001)
    }

    @Test("Detail always opens toward the desktop interior")
    func detailDirection() {
        let rightStrip = FloatingStripLayout.stripFrame(
            in: visible,
            size: stripSize,
            edge: .right,
            normalizedCenterY: 0.5
        )
        let rightDetail = FloatingStripLayout.detailFrame(
            size: CGSize(width: 390, height: 520),
            stripFrame: rightStrip,
            edge: .right,
            visibleFrame: visible
        )
        let leftStrip = FloatingStripLayout.stripFrame(
            in: visible,
            size: stripSize,
            edge: .left,
            normalizedCenterY: 0.5
        )
        let leftDetail = FloatingStripLayout.detailFrame(
            size: CGSize(width: 390, height: 520),
            stripFrame: leftStrip,
            edge: .left,
            visibleFrame: visible
        )

        #expect(rightDetail.maxX == rightStrip.minX - 9)
        #expect(leftDetail.minX == leftStrip.maxX + 9)
        #expect(rightDetail.minY >= visible.minY + 8)
        #expect(leftDetail.maxY <= visible.maxY - 8)
    }

    @Test("Shrinks an oversized detail into the interior without covering the strip")
    func oversizedDetail() {
        let compactScreen = CGRect(x: 0, y: 0, width: 400, height: 500)
        let strip = FloatingStripLayout.stripFrame(
            in: compactScreen,
            size: stripSize,
            edge: .right,
            normalizedCenterY: 0.5
        )

        let detail = FloatingStripLayout.detailFrame(
            size: CGSize(width: 620, height: 700),
            stripFrame: strip,
            edge: .right,
            visibleFrame: compactScreen
        )

        #expect(detail.minX == 8)
        #expect(detail.maxX == strip.minX - 9)
        #expect(detail.minY == 8)
        #expect(detail.maxY == 492)
    }

    @Test("Fixed sides ignore horizontal drag but keep the proposed vertical center")
    func fixedSideDrag() {
        let result = FloatingStripLayout.resolvedPlacement(
            preference: .left,
            current: .right,
            proposedFrame: CGRect(x: 1_100, y: 300, width: 108, height: 356),
            visibleFrame: visible
        )

        #expect(result.edge == .left)
        #expect(abs(result.normalizedCenterY - 0.563_063) < 0.001)
    }
}
