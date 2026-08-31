import AIMeterCore
import SwiftUI

struct FloatingStripShape: Shape {
    let edge: FloatingStripEdge

    func path(in rect: CGRect) -> Path {
        let widthScale = rect.width / 108
        let heightScale = rect.height / 356

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
        path.addLine(to: point(0, 268))
        path.addCurve(
            to: point(66, 328),
            control1: point(0, 302),
            control2: point(29, 327)
        )
        path.addCurve(
            to: point(108, 340),
            control1: point(88, 329),
            control2: point(98, 333)
        )
        path.closeSubpath()
        return path
    }
}
