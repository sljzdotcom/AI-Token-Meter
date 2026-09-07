import CoreGraphics

enum FloatingStripDragPolicy {
    static func translated(_ frame: CGRect, by translation: CGSize) -> CGRect {
        frame.offsetBy(dx: translation.width, dy: -translation.height)
    }

    static func target(at point: CGPoint, screens: [String: CGRect], pinned: String? = nil) -> String? {
        if let pinned, screens[pinned] != nil { return pinned }
        // Deterministic tie-breaking at boundaries or in gaps between staggered displays.
        let identifiers = screens.keys.sorted()
        if let hit = identifiers.first(where: { screens[$0]!.contains(point) }) { return hit }
        return identifiers.min { distance(point, screens[$0]!) < distance(point, screens[$1]!) }
    }

    private static func distance(_ point: CGPoint, _ rect: CGRect) -> CGFloat {
        let dx = max(rect.minX - point.x, 0, point.x - rect.maxX)
        let dy = max(rect.minY - point.y, 0, point.y - rect.maxY)
        return dx * dx + dy * dy
    }
}
