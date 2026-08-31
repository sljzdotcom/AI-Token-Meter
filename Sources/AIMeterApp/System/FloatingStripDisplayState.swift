import AIMeterCore
import Observation

@MainActor
@Observable
final class FloatingStripDisplayState {
    var resolvedEdge: FloatingStripEdge
    var isDragging = false

    init(resolvedEdge: FloatingStripEdge = .right) {
        self.resolvedEdge = resolvedEdge
    }
}
