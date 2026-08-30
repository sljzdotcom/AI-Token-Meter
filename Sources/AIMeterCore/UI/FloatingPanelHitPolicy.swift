import CoreGraphics
import Foundation

public struct FloatingPanelDismissalRequest: Sendable {
    public let screenPoint: CGPoint
    public let eventTimestamp: TimeInterval
    public let selectionID: FloatingDetailSelectionID

    public init(
        screenPoint: CGPoint,
        eventTimestamp: TimeInterval,
        selectionID: FloatingDetailSelectionID
    ) {
        self.screenPoint = screenPoint
        self.eventTimestamp = eventTimestamp
        self.selectionID = selectionID
    }

    public func requestsDismissal(
        currentSelectionID: FloatingDetailSelectionID?,
        strip: CGRect,
        detail: CGRect
    ) -> Bool {
        eventTimestamp >= selectionID.presentedAtUptime &&
            selectionID == currentSelectionID &&
            FloatingPanelHitPolicy.isOutside(
                screenPoint,
                strip: strip,
                detail: detail
            )
    }
}

public enum FloatingPanelHitPolicy {
    public static func isOutside(
        _ point: CGPoint,
        strip: CGRect,
        detail: CGRect
    ) -> Bool {
        !strip.contains(point) && !detail.contains(point)
    }
}
