import AIMeterCore
import SwiftUI

struct FloatingStripDragShape: Shape {
    let edge: FloatingStripEdge

    func path(in rect: CGRect) -> Path {
        let path = FloatingStripShape(edge: edge).path(in: rect)
        var dragRegion = Path()
        let scaleX = rect.width / 108
        let scaleY = rect.height / 356
        let buttonRows = [76.0, 148.0, 220.0]

        dragRegion.addRect(CGRect(x: 0, y: 0, width: rect.width, height: 76 * scaleY))
        dragRegion.addRect(CGRect(x: 0, y: 136 * scaleY, width: rect.width, height: 12 * scaleY))
        dragRegion.addRect(CGRect(x: 0, y: 208 * scaleY, width: rect.width, height: 12 * scaleY))
        dragRegion.addRect(CGRect(x: 0, y: 280 * scaleY, width: rect.width, height: 76 * scaleY))
        for row in buttonRows {
            dragRegion.addRect(CGRect(x: 0, y: row * scaleY, width: 24 * scaleX, height: 60 * scaleY))
            dragRegion.addRect(CGRect(x: 84 * scaleX, y: row * scaleY, width: 24 * scaleX, height: 60 * scaleY))
        }
        return path.intersection(dragRegion, eoFill: true)
    }
}
