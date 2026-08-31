import Foundation

public enum FloatingStripEdgePreference: String, CaseIterable, Codable, Sendable {
    case automatic
    case left
    case right
}

public enum FloatingStripEdge: String, Codable, Sendable {
    case left
    case right
}

public struct FloatingStripPosition: Equatable, Sendable {
    public var preference: FloatingStripEdgePreference
    public var lastResolvedEdge: FloatingStripEdge
    public var normalizedCenterY: Double
    public var screenIdentifier: String?

    public init(
        preference: FloatingStripEdgePreference = .automatic,
        lastResolvedEdge: FloatingStripEdge = .right,
        normalizedCenterY: Double = 0.5,
        screenIdentifier: String? = nil
    ) {
        self.preference = preference
        self.lastResolvedEdge = lastResolvedEdge
        self.normalizedCenterY = Self.clamped(normalizedCenterY)
        self.screenIdentifier = screenIdentifier
    }

    private static func clamped(_ value: Double) -> Double {
        min(max(value, 0), 1)
    }
}

public struct FloatingStripPositionStore {
    private enum Key {
        static let preference = "floatingStrip.edgePreference"
        static let lastResolvedEdge = "floatingStrip.lastResolvedEdge"
        static let normalizedCenterY = "floatingStrip.normalizedCenterY"
        static let screenIdentifier = "floatingStrip.screenIdentifier"
    }

    private let defaults: UserDefaults

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    public func load() -> FloatingStripPosition {
        let preference = defaults.string(forKey: Key.preference)
            .flatMap(FloatingStripEdgePreference.init(rawValue:)) ?? .automatic
        let lastResolvedEdge = defaults.string(forKey: Key.lastResolvedEdge)
            .flatMap(FloatingStripEdge.init(rawValue:)) ?? .right
        let normalizedCenterY = (defaults.object(forKey: Key.normalizedCenterY) as? NSNumber)?
            .doubleValue ?? 0.5

        return FloatingStripPosition(
            preference: preference,
            lastResolvedEdge: lastResolvedEdge,
            normalizedCenterY: normalizedCenterY,
            screenIdentifier: defaults.string(forKey: Key.screenIdentifier)
        )
    }

    public func save(_ position: FloatingStripPosition) {
        let normalized = FloatingStripPosition(
            preference: position.preference,
            lastResolvedEdge: position.lastResolvedEdge,
            normalizedCenterY: position.normalizedCenterY,
            screenIdentifier: position.screenIdentifier
        )
        defaults.set(normalized.preference.rawValue, forKey: Key.preference)
        defaults.set(normalized.lastResolvedEdge.rawValue, forKey: Key.lastResolvedEdge)
        defaults.set(normalized.normalizedCenterY, forKey: Key.normalizedCenterY)
        if let screenIdentifier = normalized.screenIdentifier {
            defaults.set(screenIdentifier, forKey: Key.screenIdentifier)
        } else {
            defaults.removeObject(forKey: Key.screenIdentifier)
        }
    }

}
