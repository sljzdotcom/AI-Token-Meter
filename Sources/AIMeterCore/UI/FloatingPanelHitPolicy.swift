import CoreGraphics

public enum FloatingPanelHitPolicy {
    public static func isOutside(
        _ point: CGPoint,
        strip: CGRect,
        detail: CGRect
    ) -> Bool {
        !strip.contains(point) && !detail.contains(point)
    }
}
