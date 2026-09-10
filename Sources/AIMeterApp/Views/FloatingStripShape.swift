import AIMeterCore
import SwiftUI

struct FloatingStripShape: Shape {
    let edge: FloatingStripEdge
    var density: FloatingStripDensity = .comfortable
    var providerCount = 3

    func path(in rect: CGRect) -> Path {
        let compact = density == .compact
        let contentHeight = density.contentHeight(providerCount: providerCount)
        let totalHeight = density.height(providerCount: providerCount)
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
            path.move(to: point(65, 8))
            path.addCurve(to: point(42, 22), control1: point(59, 14), control2: point(53, 21))
            path.addCurve(to: point(0, 70), control1: point(18, 23), control2: point(0, 42))
            path.addLine(to: point(0, contentHeight - 70))
            path.addCurve(to: point(18, contentHeight - 10),
                          control1: point(0, contentHeight - 36), control2: point(6, contentHeight - 15))
            path.addCurve(to: point(36, contentHeight + 8),
                          control1: point(27, contentHeight - 8), control2: point(34, contentHeight + 1))
            path.addCurve(to: point(48, totalHeight - 24),
                          control1: point(40, contentHeight + 13), control2: point(45, totalHeight - 30))
            path.addCurve(to: point(65, totalHeight - 6),
                          control1: point(52, totalHeight - 15), control2: point(59, totalHeight - 9))
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
        path.addLine(to: point(0, contentHeight - 88))
        path.addCurve(
            to: point(34, contentHeight - 10),
            control1: point(0, contentHeight - 47),
            control2: point(13, contentHeight - 12)
        )
        path.addCurve(
            to: point(68, contentHeight + 12),
            control1: point(50, contentHeight - 8),
            control2: point(64, contentHeight + 3)
        )
        path.addCurve(
            to: point(108, totalHeight - 8),
            control1: point(78, contentHeight + 20),
            control2: point(96, totalHeight - 14)
        )
        path.closeSubpath()
        return path
    }
}
