import AIMeterCore
import SwiftUI

struct FloatingStripShape: Shape {
    let edge: FloatingStripEdge

    func path(in rect: CGRect) -> Path {
        let shoulder = min(rect.width * 0.31, rect.height * 0.12)
        let bodyInset = min(rect.width * 0.14, 15)
        let radius = min(rect.width * 0.32, 35)

        func point(_ x: CGFloat, _ y: CGFloat) -> CGPoint {
            let resolvedX = switch edge {
            case .right: rect.minX + x
            case .left: rect.maxX - x
            }
            return CGPoint(x: resolvedX, y: rect.minY + y)
        }

        var path = Path()
        path.move(to: point(rect.width, 0))
        path.addLine(to: point(rect.width, rect.height))
        path.addCurve(
            to: point(rect.width - bodyInset, rect.height - shoulder),
            control1: point(rect.width - shoulder * 0.72, rect.height),
            control2: point(rect.width - bodyInset, rect.height - shoulder * 0.35)
        )
        path.addLine(to: point(radius, rect.height - shoulder))
        path.addCurve(
            to: point(0, rect.height - shoulder - radius),
            control1: point(radius * 0.42, rect.height - shoulder),
            control2: point(0, rect.height - shoulder - radius * 0.42)
        )
        path.addLine(to: point(0, shoulder + radius))
        path.addCurve(
            to: point(radius, shoulder),
            control1: point(0, shoulder + radius * 0.42),
            control2: point(radius * 0.42, shoulder)
        )
        path.addLine(to: point(rect.width - bodyInset, shoulder))
        path.addCurve(
            to: point(rect.width, 0),
            control1: point(rect.width - bodyInset, shoulder * 0.35),
            control2: point(rect.width - shoulder * 0.72, 0)
        )
        path.closeSubpath()
        return path
    }
}
