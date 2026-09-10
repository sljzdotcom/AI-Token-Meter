import Foundation

public enum FloatingStripDensity: String, Codable, CaseIterable, Sendable {
    case compact, comfortable
    public var width: Double { self == .compact ? 56.5 : 108 }
    public var ringSize: Double { self == .compact ? 48 : 60 }
    public var spacing: Double { self == .compact ? 10 : 12 }
    public var baseHeight: Double { self == .compact ? 286 : 356 }
    public func height(providerCount: Int) -> Double {
        let firstHeight = self == .compact ? 170.0 : 212.0
        return firstHeight + Double(min(max(providerCount, 1), 4) - 1) * (ringSize + spacing)
    }
}

public enum FloatingStripFoldDelay: Int, Codable, CaseIterable, Sendable {
    case never = 0, fiveSeconds = 5, fifteenSeconds = 15
    public var displayName: String { self == .never ? "Never" : "After \(rawValue) seconds" }
}

public struct FloatingStripPreferences: Codable, Equatable, Sendable {
    public private(set) var schemaVersion = 2
    public var density: FloatingStripDensity = .compact
    public var foldDelay: FloatingStripFoldDelay = .never
    public var orderedProviders: [UsageProvider] = UsageProvider.allCases
    public var hiddenProviders: [UsageProvider] = []
    public var hiddenUntil: TimeInterval?

    public init(hiddenUntil: TimeInterval? = nil) { self.hiddenUntil = hiddenUntil }

    private enum CodingKeys: String, CodingKey { case schemaVersion, density, foldDelay, orderedProviders, hiddenProviders, hiddenUntil }
    public init(from decoder: any Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        density = (try? values.decode(FloatingStripDensity.self, forKey: .density)) ?? .compact
        foldDelay = (try? values.decode(FloatingStripFoldDelay.self, forKey: .foldDelay)) ?? .never
        orderedProviders = (try? values.decode([String].self, forKey: .orderedProviders))?.compactMap(UsageProvider.init(rawValue:)) ?? UsageProvider.allCases
        hiddenProviders = (try? values.decode([String].self, forKey: .hiddenProviders))?.compactMap(UsageProvider.init(rawValue:)) ?? []
        let version = (try? values.decode(Int.self, forKey: .schemaVersion)) ?? 1
        let recorded = ((try? values.decode([String].self, forKey: .orderedProviders)) ?? [])
            + ((try? values.decode([String].self, forKey: .hiddenProviders)) ?? [])
        if version < 2 && !recorded.contains("gemini") { hiddenProviders.append(.gemini) }
        hiddenUntil = try? values.decode(TimeInterval.self, forKey: .hiddenUntil)
        normalize()
    }

    public var visibleProviders: [UsageProvider] {
        orderedProviders.filter { !hiddenProviders.contains($0) }
    }

    public mutating func normalize() {
        var seen = Set<UsageProvider>()
        orderedProviders = (orderedProviders + UsageProvider.allCases).filter { seen.insert($0).inserted }
        hiddenProviders = orderedProviders.filter { hiddenProviders.contains($0) }
        if visibleProviders.isEmpty, let first = orderedProviders.first {
            hiddenProviders.removeAll { $0 == first }
        }
        if let hiddenUntil, !hiddenUntil.isFinite { self.hiddenUntil = nil }
    }

    public func settingVisible(_ provider: UsageProvider, visible: Bool) -> Self {
        var result = self
        result.hiddenProviders.removeAll { $0 == provider }
        if !visible { result.hiddenProviders.append(provider) }
        guard !result.visibleProviders.isEmpty else { return self }
        return result
    }

    public func isTemporarilyHidden(now: TimeInterval) -> Bool {
        hiddenUntil.map { $0.isFinite && $0 > now } ?? false
    }
}

public struct FloatingStripPreferencesStore {
    private let defaults: UserDefaults
    private let key = "floatingStripPreferences"
    public init(defaults: UserDefaults) { self.defaults = defaults }
    public func load() -> FloatingStripPreferences {
        guard let data = defaults.data(forKey: key),
              var value = try? JSONDecoder().decode(FloatingStripPreferences.self, from: data) else {
            return FloatingStripPreferences()
        }
        value.normalize()
        return value
    }
    public func save(_ value: FloatingStripPreferences) {
        var normalized = value
        normalized.normalize()
        if let data = try? JSONEncoder().encode(normalized) { defaults.set(data, forKey: key) }
    }
}
