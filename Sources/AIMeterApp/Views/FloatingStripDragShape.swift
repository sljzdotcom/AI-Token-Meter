import AIMeterCore
import SwiftUI

enum FloatingStripContentLayout {
    static let referenceSize = CGSize(width: 108, height: 356)
    static let providerButtonSize: CGFloat = 60
    static let providerSpacing: CGFloat = 12
    static let verticalPadding: CGFloat = 17
    static let horizontalPadding: CGFloat = 11
    static let providerStackTopInset: CGFloat = 59

    static func providerFrames(in rect: CGRect) -> [CGRect] {
        let scaleX = rect.width / referenceSize.width
        let scaleY = rect.height / referenceSize.height
        let stackWidth = providerButtonSize + 2 * horizontalPadding
        let originX = rect.minX + ((referenceSize.width - stackWidth) / 2 + horizontalPadding) * scaleX
        let originY = rect.minY + (providerStackTopInset + verticalPadding) * scaleY

        return (0..<3).map { index in
            CGRect(
                x: originX,
                y: originY + CGFloat(index) * (providerButtonSize + providerSpacing) * scaleY,
                width: providerButtonSize * scaleX,
                height: providerButtonSize * scaleY
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
