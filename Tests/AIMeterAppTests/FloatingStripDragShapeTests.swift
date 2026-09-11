import AIMeterCore
import CoreGraphics
import Testing
@testable import AIMeterApp

@Suite("Floating strip drag region")
struct FloatingStripDragShapeTests {
    @Test func allDensitiesUseExpectedHorizontalPadding() {
        #expect(FloatingStripContentLayout.horizontalPadding(for: .mini) == 0)
        #expect(FloatingStripContentLayout.horizontalPadding(for: .compact) == 11)
        #expect(FloatingStripContentLayout.horizontalPadding(for: .comfortable) == 11)
    }

    @Test func fourthProviderHasCompleteClickRegionOnBothEdges() {
        for (density, height, lastCenter) in [(FloatingStripDensity.compact, 386.0, 259.0), (.comfortable, 476.0, 322.0)] {
            let rect = CGRect(x: 0, y: 0, width: density.width, height: height)
            for edge in [FloatingStripEdge.left, .right] {
                let shape = FloatingStripShape(edge: edge, density: density, providerCount: 4).path(in: rect)
                let drag = FloatingStripDragShape(edge: edge, density: density, providerCount: 4).path(in: rect)
                for x in [rect.midX - density.ringSize / 2 + 1, rect.midX, rect.midX + density.ringSize / 2 - 1] {
                    for y in [lastCenter - density.ringSize / 2 + 1, lastCenter, lastCenter + density.ringSize / 2 - 1] {
                        #expect(shape.contains(CGPoint(x: x, y: y)))
                        #expect(!drag.contains(CGPoint(x: x, y: y), eoFill: true))
                    }
                }
            }
        }
    }

    @Test("All ring circumferences and the Settings icon remain inside the mirrored surface")
    func ringsAndSettingsStayInsideSurface() {
        for density in FloatingStripDensity.allCases {
            for count in 1...4 {
                let rect = CGRect(x: 0, y: 0, width: density.width, height: density.height(providerCount: count))
                for edge in [FloatingStripEdge.left, .right] {
                    let path = FloatingStripShape(edge: edge, density: density, providerCount: count).path(in: rect)
                    for frame in FloatingStripContentLayout.providerFrames(in: rect, density: density, count: count) {
                        for step in 0..<72 {
                            let angle = Double(step) * .pi * 2 / 72
                            let point = CGPoint(
                                x: frame.midX + cos(angle) * (frame.width / 2 - 0.5),
                                y: frame.midY + sin(angle) * (frame.height / 2 - 0.5)
                            )
                            #expect(path.contains(point), "\(density.rawValue)/\(edge)/\(count) misses \(point)")
                        }
                    }
                    let gear = FloatingStripContentLayout.settingsButtonFrame(in: rect, edge: edge, density: density, count: count)
                    #expect(path.contains(CGPoint(x: gear.midX, y: gear.midY)))
                }
            }
        }
    }

    @Test("Undecorated top stays draggable while every Provider remains click-only")
    func topAndProviderHitRegionsAcrossDensitiesAndEdges() {
        for density in FloatingStripDensity.allCases {
            let rect = CGRect(x: 0, y: 0, width: density.width, height: density.baseHeight)
            for edge in [FloatingStripEdge.left, .right] {
                let path = FloatingStripDragShape(edge: edge, density: density).path(in: rect)
                let centerX = density.width / 2
                #expect(path.contains(CGPoint(x: centerX, y: density == .comfortable ? 44 : 36), eoFill: true))
                let centers = density == .comfortable ? [106.0, 178.0, 250.0] : [85.0, 143.0, 201.0]
                for y in centers {
                    #expect(!path.contains(CGPoint(x: centerX, y: y), eoFill: true))
                }
                #expect(path.contains(CGPoint(x: centerX, y: density == .comfortable ? 142 : 114), eoFill: true))
            }
        }
    }
    @Test("Compact hit testing excludes every visible ring after provider removal")
    func compactHitRegions() {
        let rect = CGRect(x: 0, y: 0, width: 78, height: 270)
        let shape = FloatingStripDragShape(edge: .right, density: .compact, providerCount: 2)
        let path = shape.path(in: rect)
        #expect(!path.contains(CGPoint(x: 39, y: 85), eoFill: true))
        #expect(!path.contains(CGPoint(x: 39, y: 143), eoFill: true))
        #expect(path.contains(CGPoint(x: 39, y: 114), eoFill: true))
        #expect(path.contains(CGPoint(x: 71, y: 50), eoFill: true))
    }

    @Test("Settings hover follows the visible arc instead of the transparent bottom row")
    func settingsHoverRegion() {
        for density in FloatingStripDensity.allCases {
            let rect = CGRect(x: 0, y: 0, width: density.width, height: density.settingsZoneHeight)
            let right = FloatingStripSettingsHitShape(edge: .right).path(in: rect)
            let left = FloatingStripSettingsHitShape(edge: .left).path(in: rect)

            #expect(right.contains(CGPoint(x: rect.maxX - 6, y: rect.midY)))
            #expect(!right.contains(CGPoint(x: rect.minX + 5, y: rect.midY)))
            #expect(left.contains(CGPoint(x: rect.minX + 6, y: rect.midY)))
            #expect(!left.contains(CGPoint(x: rect.maxX - 5, y: rect.midY)))
        }
    }

    @Test("Glass background drags while provider buttons remain click-only")
    func dragRegionExcludesProviderButtons() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 404)
        let right = FloatingStripDragShape(edge: .right).path(in: rect)

        #expect(right.contains(CGPoint(x: 75, y: 58), eoFill: true))
        #expect(right.contains(CGPoint(x: 40, y: 50), eoFill: true))
        #expect(right.contains(CGPoint(x: 54, y: 142), eoFill: true))
        #expect(!right.contains(CGPoint(x: 54, y: 106), eoFill: true))
        #expect(!right.contains(CGPoint(x: 54, y: 178), eoFill: true))
        #expect(!right.contains(CGPoint(x: 54, y: 250), eoFill: true))
        #expect(!right.contains(CGPoint(x: 20, y: 30), eoFill: true))
    }

    @Test("Left drag region mirrors the right region")
    func dragRegionMirrors() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 404)
        let left = FloatingStripDragShape(edge: .left).path(in: rect)

        #expect(left.contains(CGPoint(x: 33, y: 58), eoFill: true))
        #expect(!left.contains(CGPoint(x: 54, y: 178), eoFill: true))
        #expect(!left.contains(CGPoint(x: 88, y: 30), eoFill: true))
    }

    @Test("Drag exclusions remain fixed-size and centered in a scaled translated shape")
    func dragRegionUsesFixedCenteredProviderFrames() {
        let rect = CGRect(x: 100, y: 200, width: 216, height: 808)
        let right = FloatingStripDragShape(edge: .right).path(in: rect)

        #expect(right.contains(CGPoint(x: 250, y: 316), eoFill: true))
        #expect(!right.contains(CGPoint(x: 208, y: 484), eoFill: true))
        #expect(!right.contains(CGPoint(x: 208, y: 556), eoFill: true))
        #expect(!right.contains(CGPoint(x: 208, y: 628), eoFill: true))
        #expect(right.contains(CGPoint(x: 208, y: 453), eoFill: true))
        #expect(right.contains(CGPoint(x: 208, y: 515), eoFill: true))
    }

    @Test("Every provider exclusion keeps adjacent glass draggable")
    func providerExclusionBoundariesLeaveAdjacentGlassDraggable() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 404)
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
