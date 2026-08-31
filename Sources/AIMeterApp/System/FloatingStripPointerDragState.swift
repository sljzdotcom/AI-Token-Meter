import AIMeterCore
import CoreGraphics

struct FloatingStripPointerDragState {
    private(set) var startScreenPoint: CGPoint?

    var isActive: Bool {
        startScreenPoint != nil
    }

    mutating func begin(
        windowPoint: CGPoint,
        screenPoint: CGPoint,
        panelSize: CGSize,
        edge: FloatingStripEdge
    ) -> Bool {
        let rect = CGRect(origin: .zero, size: panelSize)
        let topLeadingPoint = CGPoint(
            x: windowPoint.x,
            y: panelSize.height - windowPoint.y
        )
        guard FloatingStripDragShape(edge: edge)
            .path(in: rect)
            .contains(topLeadingPoint, eoFill: true) else {
            startScreenPoint = nil
            return false
        }

        startScreenPoint = screenPoint
        return true
    }

    func translation(to screenPoint: CGPoint) -> CGSize? {
        guard let startScreenPoint else { return nil }
        return CGSize(
            width: screenPoint.x - startScreenPoint.x,
            height: startScreenPoint.y - screenPoint.y
        )
    }

    mutating func end(at screenPoint: CGPoint) -> CGSize? {
        let result = translation(to: screenPoint)
        startScreenPoint = nil
        return result
    }
}
