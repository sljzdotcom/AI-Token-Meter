import AIMeterCore

enum FloatingStripAccessibilityCommand {
    case moveUp
    case moveDown
    case moveToLeftEdge
    case moveToRightEdge
}

enum FloatingStripAccessibilityMovement {
    static let verticalStep = 0.1

    static func position(
        after command: FloatingStripAccessibilityCommand,
        currentEdge: FloatingStripEdge,
        normalizedCenterY: Double
    ) -> FloatingStripResolvedPlacement {
        switch command {
        case .moveUp:
            return FloatingStripResolvedPlacement(
                edge: currentEdge,
                normalizedCenterY: min(normalizedCenterY + verticalStep, 1)
            )
        case .moveDown:
            return FloatingStripResolvedPlacement(
                edge: currentEdge,
                normalizedCenterY: max(normalizedCenterY - verticalStep, 0)
            )
        case .moveToLeftEdge:
            return FloatingStripResolvedPlacement(
                edge: .left,
                normalizedCenterY: normalizedCenterY
            )
        case .moveToRightEdge:
            return FloatingStripResolvedPlacement(
                edge: .right,
                normalizedCenterY: normalizedCenterY
            )
        }
    }
}
