import AIMeterCore
import SwiftUI

struct FloatingStripContourGeometry {
    struct Cubic: Equatable {
        let control1: CGPoint
        let control2: CGPoint
        let end: CGPoint
    }

    let start: CGPoint
    let shoulderDepth: CGFloat
    let curves: [Cubic]
}

enum FloatingStripContour {
    static func geometry(for density: FloatingStripDensity) -> FloatingStripContourGeometry {
        let widthScale = density.width / 65
        let shoulderDepth: CGFloat = density == .comfortable ? 88 : 70
        let depthScale = shoulderDepth / 70
        func point(_ x: CGFloat, _ y: CGFloat) -> CGPoint {
            CGPoint(x: x * widthScale, y: y * depthScale)
        }
        return FloatingStripContourGeometry(
            start: point(65, 4),
            shoulderDepth: shoulderDepth,
            curves: [
                .init(control1: point(63, 18), control2: point(54, 29), end: point(37, 30)),
                .init(control1: point(18, 31), control2: point(5, 42), end: point(1, 58)),
                .init(control1: point(0, 62), control2: point(0, 66), end: point(0, 70)),
            ]
        )
    }
}

struct FloatingStripShape: Shape {
    let edge: FloatingStripEdge
    var density: FloatingStripDensity = .comfortable
    var providerCount = 3

    func path(in rect: CGRect) -> Path {
        let totalHeight = density.height(providerCount: providerCount)
        let widthScale = rect.width / density.width
        let heightScale = rect.height / density.height(providerCount: providerCount)
        let contour = FloatingStripContour.geometry(for: density)

        func point(_ x: CGFloat, _ y: CGFloat) -> CGPoint {
            let scaledX = x * widthScale
            let scaledY = y * heightScale
            let resolvedX = switch edge {
            case .right: rect.minX + scaledX
            case .left: rect.maxX - scaledX
            }
            return CGPoint(x: resolvedX, y: rect.minY + scaledY)
        }

        var path = Path()
        path.move(to: point(contour.start.x, contour.start.y))
        for curve in contour.curves {
            path.addCurve(
                to: point(curve.end.x, curve.end.y),
                control1: point(curve.control1.x, curve.control1.y),
                control2: point(curve.control2.x, curve.control2.y)
            )
        }
        path.addLine(to: point(0, totalHeight - contour.shoulderDepth))
        for (start, curve) in zip([contour.start] + contour.curves.map(\.end), contour.curves).reversed() {
            path.addCurve(
                to: point(start.x, totalHeight - start.y),
                control1: point(curve.control2.x, totalHeight - curve.control2.y),
                control2: point(curve.control1.x, totalHeight - curve.control1.y)
            )
        }
        path.closeSubpath()
        return path
    }
}

struct FloatingStripFoldedShape: Shape {
    let edge: FloatingStripEdge

    func path(in rect: CGRect) -> Path {
        func point(_ x: CGFloat, _ y: CGFloat) -> CGPoint {
            let scaledX = x * rect.width / 14
            let resolvedX = edge == .right ? rect.minX + scaledX : rect.maxX - scaledX
            return CGPoint(x: resolvedX, y: rect.minY + y * rect.height / 88)
        }

        var path = Path()
        path.move(to: point(14, 0))
        path.addCurve(to: point(6.3, 18), control1: point(14, 8), control2: point(12, 14))
        path.addCurve(to: point(0, 30), control1: point(3.3, 20), control2: point(0, 24))
        path.addLine(to: point(0, 58))
        path.addCurve(to: point(6.3, 70), control1: point(0, 64), control2: point(3.3, 68))
        path.addCurve(to: point(14, 88), control1: point(12, 74), control2: point(14, 80))
        path.closeSubpath()
        return path
    }
}
