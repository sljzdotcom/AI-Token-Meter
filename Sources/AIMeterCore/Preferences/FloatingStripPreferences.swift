import Foundation

public enum FloatingStripDensity: String, Codable, CaseIterable, Sendable {
    case comfortable, compact, mini
    public var width: Double {
        switch self { case .mini: 65; case .compact: 78; case .comfortable: 108 }
    }
    public var ringSize: Double { self == .comfortable ? 60 : 48 }
    public var spacing: Double { self == .comfortable ? 12 : 10 }
    public var settingsZoneHeight: Double { self == .comfortable ? 48 : 42 }
    public var baseContentHeight: Double { self == .comfortable ? 356 : 286 }
    public var baseHeight: Double { baseContentHeight + settingsZoneHeight }
    public func contentHeight(providerCount: Int) -> Double {
        let firstHeight = self == .comfortable ? 212.0 : 170.0
        return firstHeight + Double(min(max(providerCount, 1), 4) - 1) * (ringSize + spacing)
    }
    public func height(providerCount: Int) -> Double {
        contentHeight(providerCount: providerCount) + settingsZoneHeight
    }
}

public enum FloatingStripFoldDelay: Int, Codable, CaseIterable, Sendable {
    case never = 0, fiveSeconds = 5, fifteenSeconds = 15
    public var displayName: String { self == .never ? "Never" : "After \(rawValue) seconds" }
}

public struct FloatingStripPreferences: Codable, Equatable, Sendable {
    public private(set) var schemaVersion = 4
    public var density: FloatingStripDensity = .compact
    public var automaticallyCollapses = true
    public var revealDelayMilliseconds = 150
    public var collapseDelayMilliseconds = 800
    public var orderedProviders: [UsageProvider] = UsageProvider.allCases
    public var hiddenProviders: [UsageProvider] = []
    public var hiddenUntil: TimeInterval?

    public init(
        hiddenUntil: TimeInterval? = nil,
        revealDelayMilliseconds: Int = 150,
        collapseDelayMilliseconds: Int = 800,
        automaticallyCollapses: Bool = true
    ) {
        self.hiddenUntil = hiddenUntil
        self.revealDelayMilliseconds = revealDelayMilliseconds
        self.collapseDelayMilliseconds = collapseDelayMilliseconds
        self.automaticallyCollapses = automaticallyCollapses
        normalize()
    }

    private enum CodingKeys: String, CodingKey {
        case schemaVersion, density, automaticallyCollapses, revealDelayMilliseconds,
             collapseDelayMilliseconds, orderedProviders, hiddenProviders, hiddenUntil
    }
    private enum LegacyCodingKeys: String, CodingKey { case foldDelay }
    public init(from decoder: any Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        density = (try? values.decode(FloatingStripDensity.self, forKey: .density)) ?? .compact
        let version = (try? values.decode(Int.self, forKey: .schemaVersion)) ?? 1
        automaticallyCollapses = version >= 4
            ? ((try? values.decode(Bool.self, forKey: .automaticallyCollapses)) ?? true)
            : true
        if version >= 3 {
            revealDelayMilliseconds = (try? values.decode(Int.self, forKey: .revealDelayMilliseconds)) ?? 150
            collapseDelayMilliseconds = (try? values.decode(Int.self, forKey: .collapseDelayMilliseconds)) ?? 800
        } else {
            let legacyValues = try decoder.container(keyedBy: LegacyCodingKeys.self)
            let legacySeconds = (try? legacyValues.decode(Int.self, forKey: .foldDelay)) ?? 0
            revealDelayMilliseconds = 150
            collapseDelayMilliseconds = legacySeconds == 0 ? 800 : min(legacySeconds * 1_000, 5_000)
        }
        orderedProviders = (try? values.decode([String].self, forKey: .orderedProviders))?.compactMap(UsageProvider.init(rawValue:)) ?? UsageProvider.allCases
        hiddenProviders = (try? values.decode([String].self, forKey: .hiddenProviders))?.compactMap(UsageProvider.init(rawValue:)) ?? []
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
        schemaVersion = 4
        if !(0...2_000).contains(revealDelayMilliseconds) { revealDelayMilliseconds = 150 }
        if !(0...5_000).contains(collapseDelayMilliseconds) { collapseDelayMilliseconds = 800 }
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
