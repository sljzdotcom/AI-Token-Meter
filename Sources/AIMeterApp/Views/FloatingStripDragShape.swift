import AIMeterCore
import SwiftUI

enum FloatingStripContentLayout {
    static let providerButtonSize: CGFloat = 60
    static let providerSpacing: CGFloat = 12
    static let verticalPadding: CGFloat = 17
    static func horizontalPadding(for density: FloatingStripDensity) -> CGFloat {
        density == .mini ? 0 : 11
    }

    static func providerFrames(in rect: CGRect, density: FloatingStripDensity = .comfortable, count: Int = 3) -> [CGRect] {
        let providerButtonSize = density.ringSize
        let providerSpacing = density.spacing
        let providerStackHeight = Double(count) * providerButtonSize + Double(count - 1) * providerSpacing + 2 * verticalPadding
        let originX = rect.midX - providerButtonSize / 2
        let contentHeight = density.contentHeight(providerCount: count) * rect.height / density.height(providerCount: count)
        let originY = rect.minY + contentHeight / 2 - providerStackHeight / 2 + verticalPadding

        return (0..<count).map { index in
            CGRect(
                x: originX,
                y: originY + CGFloat(index) * (providerButtonSize + providerSpacing),
                width: providerButtonSize,
                height: providerButtonSize
            )
        }
    }

    static func settingsButtonFrame(
        in rect: CGRect,
        edge: FloatingStripEdge,
        density: FloatingStripDensity = .comfortable,
        count: Int = 3
    ) -> CGRect {
        let scaleX = rect.width / density.width
        let scaleY = rect.height / density.height(providerCount: count)
        let size = (density == .comfortable ? 28.0 : 24.0) * min(scaleX, scaleY)
        let trailingInset = (density == .mini ? 2.0 : 4.0) * scaleX
        let contentHeight = density.contentHeight(providerCount: count) * scaleY
        let x = edge == .right ? rect.maxX - trailingInset - size : rect.minX + trailingInset
        return CGRect(x: x, y: rect.minY + contentHeight + 2 * scaleY, width: size, height: size)
    }
}

struct FloatingStripDragShape: Shape {
    let edge: FloatingStripEdge
    var density: FloatingStripDensity = .comfortable
    var providerCount = 3

    func path(in rect: CGRect) -> Path {
        let path = FloatingStripShape(edge: edge, density: density, providerCount: providerCount).path(in: rect)
        var dragRegion = Path()
        let frames = FloatingStripContentLayout.providerFrames(in: rect, density: density, count: providerCount)
        let settingsFrame = FloatingStripContentLayout.settingsButtonFrame(
            in: rect, edge: edge, density: density, count: providerCount
        )
        var nextY = rect.minY

        for frame in frames {
            dragRegion.addRect(CGRect(x: rect.minX, y: nextY, width: rect.width, height: frame.minY - nextY))
            dragRegion.addRect(CGRect(x: rect.minX, y: frame.minY, width: frame.minX - rect.minX, height: frame.height))
            dragRegion.addRect(CGRect(x: frame.maxX, y: frame.minY, width: rect.maxX - frame.maxX, height: frame.height))
            nextY = frame.maxY
        }
        dragRegion.addRect(CGRect(x: rect.minX, y: nextY, width: rect.width, height: rect.maxY - nextY))
        return path.intersection(dragRegion, eoFill: true).subtracting(Path(settingsFrame), eoFill: true)
    }
}
