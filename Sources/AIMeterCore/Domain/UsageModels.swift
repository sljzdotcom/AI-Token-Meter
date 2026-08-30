import Foundation

public enum UsageProvider: String, Codable, CaseIterable, Sendable {
    case claude
    case codex
    case deepSeek
}

public enum UsageUnit: String, Codable, Sendable {
    case percent
    case cny
    case usd
    case tokens
    case requests
}

public enum UsageMetricKind: String, Codable, Sendable {
    case officialLimit
    case balance
    case localBudget
}

public enum Availability: String, Codable, Sendable {
    case available
    case unavailable
    case unknown
}

public enum CollectionStatus: String, Codable, Sendable {
    case fresh
    case cached
    case refreshing
    case notInstalled
    case authenticationRequired
    case setupRequired
    case unavailable
    case unrecognizedOutput
}

public struct UsageMetric: Codable, Equatable, Identifiable, Sendable {
    public let id: UUID
    public let label: String
    public let current: Double
    public let limit: Double?
    public let unit: UsageUnit
    public let kind: UsageMetricKind
    public let resetAt: Date?
    public let resetDescription: String?

    public init(
        id: UUID = UUID(),
        label: String,
        current: Double,
        limit: Double?,
        unit: UsageUnit,
        kind: UsageMetricKind = .officialLimit,
        resetAt: Date? = nil,
        resetDescription: String? = nil
    ) {
        self.id = id
        self.label = label
        self.current = current
        self.limit = limit
        self.unit = unit
        self.kind = kind
        self.resetAt = resetAt
        self.resetDescription = resetDescription
    }

    public var usedFraction: Double? {
        guard let limit, limit > 0 else { return nil }
        return min(max(current / limit, 0), 1)
    }
}

public struct UsageSnapshot: Codable, Equatable, Identifiable, Sendable {
    public var id: UsageProvider { provider }

    public let provider: UsageProvider
    public let primaryMetric: UsageMetric?
    public let secondaryMetric: UsageMetric?
    public let availability: Availability
    public let fetchedAt: Date
    public let staleAfter: TimeInterval
    public let sourceVersion: String?
    public let collectionStatus: CollectionStatus
    public let statusMessage: String?
    public let codexResetCredits: CodexResetCreditsSummary?
    public let codexLocalActivity: CodexLocalActivitySummary?
    public let deepSeekUsageHistory: DeepSeekUsageHistory?

    public init(
        provider: UsageProvider,
        primaryMetric: UsageMetric? = nil,
        secondaryMetric: UsageMetric? = nil,
        availability: Availability = .available,
        fetchedAt: Date = Date(),
        staleAfter: TimeInterval = 300,
        sourceVersion: String? = nil,
        collectionStatus: CollectionStatus = .fresh,
        statusMessage: String? = nil,
        codexResetCredits: CodexResetCreditsSummary? = nil,
        codexLocalActivity: CodexLocalActivitySummary? = nil,
        deepSeekUsageHistory: DeepSeekUsageHistory? = nil
    ) {
        self.provider = provider
        self.primaryMetric = primaryMetric
        self.secondaryMetric = secondaryMetric
        self.availability = availability
        self.fetchedAt = fetchedAt
        self.staleAfter = staleAfter
        self.sourceVersion = sourceVersion
        self.collectionStatus = collectionStatus
        self.statusMessage = statusMessage
        self.codexResetCredits = codexResetCredits
        self.codexLocalActivity = codexLocalActivity
        self.deepSeekUsageHistory = deepSeekUsageHistory
    }

    public func isStale(at date: Date = Date()) -> Bool {
        date >= fetchedAt.addingTimeInterval(staleAfter)
    }
}

public extension UsageSnapshot {
    func withCodexLocalActivity(_ activity: CodexLocalActivitySummary?) -> UsageSnapshot {
        UsageSnapshot(
            provider: provider,
            primaryMetric: primaryMetric,
            secondaryMetric: secondaryMetric,
            availability: availability,
            fetchedAt: fetchedAt,
            staleAfter: staleAfter,
            sourceVersion: sourceVersion,
            collectionStatus: collectionStatus,
            statusMessage: statusMessage,
            codexResetCredits: codexResetCredits,
            codexLocalActivity: activity,
            deepSeekUsageHistory: deepSeekUsageHistory
        )
    }
}
