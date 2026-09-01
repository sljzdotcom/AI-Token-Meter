import Foundation

public enum AITokenMeterWidgetContract {
    public static let kind = "com.millerpan.AIMeter.usage"
    public static let appGroupInfoKey = "AIWidgetAppGroupIdentifier"
    public static let snapshotFileName = "widget-snapshot.json"
}

public enum WidgetProvider: String, Codable, CaseIterable, Sendable {
    case claude
    case codex
    case deepSeek
}

public enum WidgetSnapshotSemantic: String, Codable, Sendable {
    case normal
    case warning
    case critical
    case stale
    case unavailable
}

public struct WidgetProviderSnapshot: Codable, Equatable, Sendable {
    public let provider: WidgetProvider
    public let valueText: String
    public let detailText: String
    public let fraction: Double?
    public let semantic: WidgetSnapshotSemantic
    public let fetchedAt: Date?
    public let expiresAt: Date?

    public init(
        provider: WidgetProvider,
        valueText: String,
        detailText: String,
        fraction: Double?,
        semantic: WidgetSnapshotSemantic,
        fetchedAt: Date?,
        expiresAt: Date?
    ) {
        self.provider = provider
        self.valueText = valueText
        self.detailText = detailText
        self.fraction = fraction.map { min(max($0, 0), 1) }
        self.semantic = semantic
        self.fetchedAt = fetchedAt
        self.expiresAt = expiresAt
    }
}

public struct WidgetResetSummary: Codable, Equatable, Sendable {
    public let provider: WidgetProvider
    public let label: String
    public let text: String
    public let resetAt: Date?

    public init(provider: WidgetProvider, label: String, text: String, resetAt: Date?) {
        self.provider = provider
        self.label = label
        self.text = text
        self.resetAt = resetAt
    }
}

public struct WidgetResetCreditsSummary: Codable, Equatable, Sendable {
    public let availableCount: Int
    public let nearestExpiration: Date?

    public init(availableCount: Int, nearestExpiration: Date?) {
        self.availableCount = max(availableCount, 0)
        self.nearestExpiration = nearestExpiration
    }
}

public struct WidgetSnapshotEnvelope: Codable, Equatable, Sendable {
    public static let currentVersion = 1

    public let version: Int
    public let generatedAt: Date
    public let providers: [WidgetProviderSnapshot]
    public let nextReset: WidgetResetSummary?
    public let codexResetCredits: WidgetResetCreditsSummary?

    public init(
        version: Int = WidgetSnapshotEnvelope.currentVersion,
        generatedAt: Date,
        providers: [WidgetProviderSnapshot],
        nextReset: WidgetResetSummary?,
        codexResetCredits: WidgetResetCreditsSummary?
    ) {
        self.version = version
        self.generatedAt = generatedAt
        self.providers = providers
        self.nextReset = nextReset
        self.codexResetCredits = codexResetCredits
    }
}
