import Foundation

/// Monotonic time is supplied by the window owner; no detached timer can fold a new interaction.
public struct FloatingStripFoldState: Sendable {
    /// Polling is only the fallback for pointer/focus changes that do not arrive as view events.
    /// Keep it below the 50 ms preference step so configured delays are not visibly quantized.
    public static let pollingInterval: TimeInterval = 0.025

    public private(set) var isFolded = false
    private var revealDeadline: TimeInterval?
    private var collapseDeadline: TimeInterval?
    public init() {}

    public mutating func update(
        now: TimeInterval,
        revealDelay: TimeInterval,
        collapseDelay: TimeInterval,
        hovering: Bool,
        lockedOpen: Bool,
        automaticallyCollapses: Bool = true
    ) {
        if lockedOpen || !automaticallyCollapses {
            isFolded = false
            revealDeadline = nil
            collapseDeadline = nil
            return
        }

        if isFolded {
            collapseDeadline = nil
            guard hovering else {
                revealDeadline = nil
                return
            }
            if revealDeadline == nil { revealDeadline = now + max(0, revealDelay) }
            if let revealDeadline, now >= revealDeadline {
                isFolded = false
                self.revealDeadline = nil
            }
            return
        }

        revealDeadline = nil
        guard !hovering else {
            collapseDeadline = nil
            return
        }
        if collapseDeadline == nil { collapseDeadline = now + max(0, collapseDelay) }
        if let collapseDeadline, now >= collapseDeadline {
            isFolded = true
            self.collapseDeadline = nil
        }
    }
}
