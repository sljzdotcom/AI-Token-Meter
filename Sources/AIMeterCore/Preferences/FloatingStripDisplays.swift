import Foundation

public enum FloatingStripDisplayMode: String, Codable, CaseIterable, Sendable {
    case primary, selected, all
}

public enum FloatingStripPlacementIntent: Sendable {
    case drag, edit
}

public struct FloatingStripScreenPlacement: Codable, Equatable, Sendable {
    public var edge: FloatingStripEdge
    public private(set) var normalizedCenterY: Double

    public init(edge: FloatingStripEdge = .right, normalizedCenterY: Double = 0.5) {
        self.edge = edge
        self.normalizedCenterY = normalizedCenterY.isFinite ? min(1, max(0, normalizedCenterY)) : 0.5
    }
}

public struct FloatingStripDisplays: Codable, Equatable, Sendable {
    public var mode: FloatingStripDisplayMode
    public var selectedIdentifier: String?
    public private(set) var placements: [String: FloatingStripScreenPlacement]
    public var fallbackPlacement: FloatingStripScreenPlacement

    public init(mode: FloatingStripDisplayMode = .primary, selectedIdentifier: String? = nil,
                placements: [String: FloatingStripScreenPlacement] = [:],
                fallbackPlacement: FloatingStripScreenPlacement = .init()) {
        self.mode = mode
        self.selectedIdentifier = selectedIdentifier
        self.placements = placements
        self.fallbackPlacement = fallbackPlacement
    }

    public func targetIdentifiers(online: [String], primary: String?) -> [String] {
        var seen = Set<String>()
        let unique = online.filter { seen.insert($0).inserted }
        guard let first = unique.first else { return [] }
        if mode == .all { return unique }
        if mode == .selected, let selectedIdentifier, unique.contains(selectedIdentifier) {
            return [selectedIdentifier]
        }
        return [primary.flatMap { unique.contains($0) ? $0 : nil } ?? first]
    }

    public func shouldSelectTarget(after intent: FloatingStripPlacementIntent, actualIdentifier: String) -> Bool {
        guard mode != .all else { return false }
        return intent == .drag || (mode == .selected && selectedIdentifier != actualIdentifier)
    }

    public func placement(for identifier: String, preference: FloatingStripEdgePreference = .automatic) -> FloatingStripScreenPlacement {
        let value = placements[identifier] ?? fallbackPlacement
        let edge: FloatingStripEdge = switch preference {
        case .automatic: value.edge
        case .left: .left
        case .right: .right
        }
        return .init(edge: edge, normalizedCenterY: value.normalizedCenterY)
    }

    public mutating func record(identifier: String, edge: FloatingStripEdge, normalizedCenterY: Double) {
        placements[identifier] = .init(edge: edge, normalizedCenterY: normalizedCenterY)
    }

    public mutating func migrate(from old: String, to new: String) {
        guard old != new else { return }
        if selectedIdentifier == old { selectedIdentifier = new }
        if let placement = placements.removeValue(forKey: old), placements[new] == nil {
            placements[new] = placement
        }
    }
}

public struct FloatingStripDisplaysStore {
    private let defaults: UserDefaults
    private let key = "floatingStrip.displays.v1"
    public init(defaults: UserDefaults = .standard) { self.defaults = defaults }

    public func load() -> FloatingStripDisplays {
        if let data = defaults.data(forKey: key),
           let decoded = try? JSONDecoder().decode(FloatingStripDisplays.self, from: data) {
            return decoded
        }
        let legacy = FloatingStripPositionStore(defaults: defaults).load()
        let placement = FloatingStripScreenPlacement(edge: legacy.lastResolvedEdge,
                                                      normalizedCenterY: legacy.normalizedCenterY)
        var value = FloatingStripDisplays(mode: legacy.screenIdentifier == nil ? .primary : .selected,
                                          selectedIdentifier: legacy.screenIdentifier,
                                          fallbackPlacement: placement)
        if let identifier = legacy.screenIdentifier {
            value.record(identifier: identifier, edge: placement.edge, normalizedCenterY: placement.normalizedCenterY)
        }
        save(value)
        return value
    }

    public func save(_ value: FloatingStripDisplays) {
        guard let data = try? JSONEncoder().encode(value) else { return }
        defaults.set(data, forKey: key)
    }
}
