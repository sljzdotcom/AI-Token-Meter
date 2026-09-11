import AIMeterCore
import SwiftUI

struct FloatingStripShape: Shape {
    let edge: FloatingStripEdge
    var density: FloatingStripDensity = .comfortable
    var providerCount = 3

    func path(in rect: CGRect) -> Path {
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
        if density == .mini {
            path.move(to: point(65, 8))
            path.addCurve(to: point(42, 22), control1: point(59, 14), control2: point(53, 21))
            path.addCurve(to: point(0, 70), control1: point(18, 23), control2: point(0, 42))
            path.addLine(to: point(0, contentHeight - 70))
            path.addCurve(to: point(42, totalHeight - 22),
                          control1: point(0, totalHeight - 42), control2: point(18, totalHeight - 23))
            path.addCurve(to: point(65, totalHeight - 8),
                          control1: point(53, totalHeight - 21), control2: point(59, totalHeight - 14))
            path.closeSubpath()
            return path
        }
        if density == .compact {
            path.move(to: point(78, 8))
            path.addCurve(to: point(48, 22), control1: point(71, 14), control2: point(63, 21))
            path.addCurve(to: point(0, 70), control1: point(21, 23), control2: point(0, 42))
            path.addLine(to: point(0, contentHeight - 70))
            path.addCurve(to: point(48, totalHeight - 22),
                          control1: point(0, totalHeight - 42), control2: point(21, totalHeight - 23))
            path.addCurve(to: point(78, totalHeight - 8),
                          control1: point(63, totalHeight - 21), control2: point(71, totalHeight - 14))
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
            to: point(66, totalHeight - 28),
            control1: point(0, totalHeight - 54),
            control2: point(29, totalHeight - 29)
        )
        path.addCurve(
            to: point(108, totalHeight - 16),
            control1: point(88, totalHeight - 27),
            control2: point(98, totalHeight - 23)
        )
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
