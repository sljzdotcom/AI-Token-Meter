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
        path.addLine(to: point(89, 0))
        path.addCurve(to: point(58, 31), control1: point(67, 0), control2: point(58, 11))
        path.addLine(to: point(58, 41))
        path.addCurve(to: point(28, 68), control1: point(58, 59), control2: point(48, 68))
        path.addLine(to: point(18, 68))
        path.addCurve(to: point(0, 89), control1: point(7, 68), control2: point(0, 77))
        path.addLine(to: point(0, 267))
        path.addCurve(to: point(18, 288), control1: point(0, 279), control2: point(7, 288))
        path.addLine(to: point(28, 288))
        path.addCurve(to: point(58, 315), control1: point(48, 288), control2: point(58, 297))
        path.addLine(to: point(58, 325))
        path.addCurve(to: point(89, 356), control1: point(58, 345), control2: point(67, 356))
        path.addLine(to: point(108, 356))
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
