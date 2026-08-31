import AIMeterCore
import Foundation

enum FloatingStripLayout {
    static let detailGap: CGFloat = 9
    static let detailScreenInset: CGFloat = 8

    static func stripFrame(
        in visibleFrame: CGRect,
        size: CGSize,
        edge: FloatingStripEdge,
        normalizedCenterY: Double
    ) -> CGRect {
        let travel = max(visibleFrame.height - size.height, 0)
        let normalized = min(max(normalizedCenterY, 0), 1)
        let originY = visibleFrame.minY + travel * normalized
        let originX = switch edge {
        case .left: visibleFrame.minX
        case .right: visibleFrame.maxX - size.width
        }
        return CGRect(origin: CGPoint(x: originX, y: originY), size: size)
    }

    static func resolvedEdge(
        preference: FloatingStripEdgePreference,
        current: FloatingStripEdge,
        proposedMidX: CGFloat,
        visibleFrame: CGRect
    ) -> FloatingStripEdge {
        switch preference {
        case .left:
            return .left
        case .right:
            return .right
        case .automatic:
            let distanceFromLeft = abs(proposedMidX - visibleFrame.minX)
            let distanceFromRight = abs(visibleFrame.maxX - proposedMidX)
            if distanceFromLeft == distanceFromRight {
                return current
            }
            return distanceFromLeft < distanceFromRight ? .left : .right
        }
    }

    static func normalizedCenterY(for frame: CGRect, in visibleFrame: CGRect) -> Double {
        let travel = visibleFrame.height - frame.height
        guard travel > 0 else { return 0.5 }
        return min(max((frame.minY - visibleFrame.minY) / travel, 0), 1)
    }

    static func detailFrame(
        size: CGSize,
        stripFrame: CGRect,
        edge: FloatingStripEdge,
        visibleFrame: CGRect
    ) -> CGRect {
        let minimumInteriorX: CGFloat
        let maximumInteriorX: CGFloat
        switch edge {
        case .left:
            minimumInteriorX = stripFrame.maxX + detailGap
            maximumInteriorX = visibleFrame.maxX - detailScreenInset
        case .right:
            minimumInteriorX = visibleFrame.minX + detailScreenInset
            maximumInteriorX = stripFrame.minX - detailGap
        }
        let availableWidth = max(maximumInteriorX - minimumInteriorX, 0)
        let availableHeight = max(visibleFrame.height - detailScreenInset * 2, 0)
        let fittedSize = CGSize(
            width: min(size.width, availableWidth),
            height: min(size.height, availableHeight)
        )
        let originX = switch edge {
        case .left: minimumInteriorX
        case .right: maximumInteriorX - fittedSize.width
        }

        let proposedY = stripFrame.midY - fittedSize.height / 2
        let minimumY = visibleFrame.minY + detailScreenInset
        let maximumY = visibleFrame.maxY - detailScreenInset - fittedSize.height
        let originY = min(max(proposedY, minimumY), maximumY)

        return CGRect(origin: CGPoint(x: originX, y: originY), size: fittedSize)
    }
}
