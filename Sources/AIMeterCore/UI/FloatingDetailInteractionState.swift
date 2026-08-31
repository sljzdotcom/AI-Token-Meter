public struct FloatingDetailInteractionState: Equatable, Sendable {
    public var isPointerInside = false
    public var hasFocusedControl = false
    public var hasInteractiveContent = false
    public var isAccessibilityReaderActive = false

    public init() {}

    public var shouldPauseAutoHide: Bool {
        isPointerInside
            || hasFocusedControl
            || hasInteractiveContent
            || isAccessibilityReaderActive
    }
}
