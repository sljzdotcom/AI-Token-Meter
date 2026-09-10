import AIMeterCore
import SwiftUI

struct FloatingStripShape: Shape {
    let edge: FloatingStripEdge
    var density: FloatingStripDensity = .comfortable
    var providerCount = 3

    func path(in rect: CGRect) -> Path {
        let compact = density == .compact
        let removed = density.baseHeight - density.height(providerCount: providerCount)
        let widthScale = rect.width / density.width
        let heightScale = rect.height / density.height(providerCount: providerCount)

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
        if compact {
            path.move(to: point(65, 12))
            path.addCurve(to: point(40, 22), control1: point(59, 17), control2: point(53, 21))
            path.addCurve(to: point(0, 70), control1: point(18, 23), control2: point(0, 42))
            path.addLine(to: point(0, 216 - removed))
            path.addCurve(to: point(40, 264 - removed), control1: point(0, 244 - removed), control2: point(18, 263 - removed))
            path.addCurve(to: point(65, 274 - removed), control1: point(53, 265 - removed), control2: point(59, 269 - removed))
            path.closeSubpath()
            return path
        }
        path.move(to: point(108, 16))
        path.addCurve(
            to: point(66, 28),
            control1: point(98, 23),
            control2: point(88, 27)
        )
        path.addCurve(
            to: point(0, 88),
            control1: point(29, 29),
            control2: point(0, 54)
        )
        path.addLine(to: point(0, 268 - removed))
        path.addCurve(
            to: point(66, 328 - removed),
            control1: point(0, 302 - removed),
            control2: point(29, 327 - removed)
        )
        path.addCurve(
            to: point(108, 340 - removed),
            control1: point(88, 329 - removed),
            control2: point(98, 333 - removed)
        )
        path.closeSubpath()
        return path
    }
}
