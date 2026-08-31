import AIMeterCore
import Observation

@MainActor
@Observable
final class FloatingStripDisplayState {
    var resolvedEdge: FloatingStripEdge
    var normalizedCenterY: Double
    var isDragging = false

    init(
        resolvedEdge: FloatingStripEdge = .right,
        normalizedCenterY: Double = 0.5
    ) {
        self.resolvedEdge = resolvedEdge
        self.normalizedCenterY = normalizedCenterY
    }
}
