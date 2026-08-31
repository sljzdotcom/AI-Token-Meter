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
        path.move(to: point(108, 0))
        path.addCurve(
            to: point(78, 52),
            control1: point(108, 28),
            control2: point(98, 45)
        )
        path.addCurve(
            to: point(0, 88),
            control1: point(53, 60),
            control2: point(0, 63)
        )
        path.addLine(to: point(0, 268))
        path.addCurve(
            to: point(78, 304),
            control1: point(0, 293),
            control2: point(53, 296)
        )
        path.addCurve(
            to: point(108, 356),
            control1: point(98, 311),
            control2: point(108, 328)
        )
        path.closeSubpath()
        return path
    }
}

struct FloatingStripSurface: View {
    let edge: FloatingStripEdge

    var body: some View {
        FloatingStripShape(edge: edge)
            .fill(AIMeterVisualTheme.floatingGlass)
            .shadow(
                color: .black.opacity(0.34),
                radius: 18,
                x: edge == .right ? -6 : 6,
                y: 8
            )
    }
}
