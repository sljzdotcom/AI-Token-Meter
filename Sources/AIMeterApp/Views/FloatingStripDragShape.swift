import AIMeterCore
import SwiftUI

enum FloatingStripContentLayout {
    static let providerButtonSize: CGFloat = 60
    static let providerSpacing: CGFloat = 12
    static let verticalPadding: CGFloat = 17
    static let horizontalPadding: CGFloat = 11

    static func providerFrames(in rect: CGRect) -> [CGRect] {
        let providerStackHeight = 3 * providerButtonSize + 2 * providerSpacing + 2 * verticalPadding
        let originX = rect.midX - providerButtonSize / 2
        let originY = rect.midY - providerStackHeight / 2 + verticalPadding

        return (0..<3).map { index in
            CGRect(
                x: originX,
                y: originY + CGFloat(index) * (providerButtonSize + providerSpacing),
                width: providerButtonSize,
                height: providerButtonSize
            )
        }
    }
}

struct FloatingStripDragShape: Shape {
    let edge: FloatingStripEdge

    func path(in rect: CGRect) -> Path {
        let path = FloatingStripShape(edge: edge).path(in: rect)
        var dragRegion = Path()
        let frames = FloatingStripContentLayout.providerFrames(in: rect)
        var nextY = rect.minY

        for frame in frames {
            dragRegion.addRect(CGRect(x: rect.minX, y: nextY, width: rect.width, height: frame.minY - nextY))
            dragRegion.addRect(CGRect(x: rect.minX, y: frame.minY, width: frame.minX - rect.minX, height: frame.height))
            dragRegion.addRect(CGRect(x: frame.maxX, y: frame.minY, width: rect.maxX - frame.maxX, height: frame.height))
            nextY = frame.maxY
        }
        dragRegion.addRect(CGRect(x: rect.minX, y: nextY, width: rect.width, height: rect.maxY - nextY))
        return path.intersection(dragRegion, eoFill: true)
    }
}
