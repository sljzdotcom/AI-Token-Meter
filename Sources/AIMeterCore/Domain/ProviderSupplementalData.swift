import Foundation

public struct CodexResetCreditDisplay: Codable, Equatable, Sendable {
    public let title: String?
    public let expiresAt: Date?

    public init(title: String?, expiresAt: Date?) {
        self.title = title
        self.expiresAt = expiresAt
    }
}

public struct CodexResetCreditsSummary: Codable, Equatable, Sendable {
    public let availableCount: Int
    public let credits: [CodexResetCreditDisplay]
    public let hasCompleteDetails: Bool

    public init(
        availableCount: Int,
        credits: [CodexResetCreditDisplay],
        hasCompleteDetails: Bool
    ) {
        self.availableCount = availableCount
        self.credits = credits
        self.hasCompleteDetails = hasCompleteDetails
    }
}

public struct CodexLocalActivitySummary: Codable, Equatable, Sendable {
    public let tokenCount: Int64
    public let currentStreakDays: Int
    public let longestSessionDuration: TimeInterval
    public let dayCount: Int

    public init(
        tokenCount: Int64,
        currentStreakDays: Int,
        longestSessionDuration: TimeInterval,
        dayCount: Int = 30
    ) {
        self.tokenCount = max(tokenCount, 0)
        self.currentStreakDays = max(currentStreakDays, 0)
        self.longestSessionDuration = max(longestSessionDuration, 0)
        self.dayCount = max(dayCount, 1)
    }
}

public struct ClaudeDailyActivity: Codable, Equatable, Identifiable, Sendable {
    public var id: Date { date }

    public let date: Date
    public let inputTokens: Int64
    public let outputTokens: Int64
    public let cacheTokens: Int64

    public init(
        date: Date,
        inputTokens: Int64,
        outputTokens: Int64,
        cacheTokens: Int64
    ) {
        self.date = date
        self.inputTokens = max(inputTokens, 0)
        self.outputTokens = max(outputTokens, 0)
        self.cacheTokens = max(cacheTokens, 0)
    }

    public var totalTokens: Int64 {
        inputTokens.addingClamped(outputTokens).addingClamped(cacheTokens)
    }
}

public struct ClaudeModelActivity: Codable, Equatable, Identifiable, Sendable {
    static let unknownModelID = "Unknown model"

    public var id: String { modelID }

    public let modelID: String
    public let tokenCount: Int64

    public init(modelID: String, tokenCount: Int64) {
        self.modelID = Self.normalizedModelID(modelID) ?? Self.unknownModelID
        self.tokenCount = max(tokenCount, 0)
    }

    public init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            modelID: try values.decode(String.self, forKey: .modelID),
            tokenCount: try values.decode(Int64.self, forKey: .tokenCount)
        )
    }

    static func normalizedModelID(_ value: String) -> String? {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty,
              trimmed.utf8.count <= 80,
              SensitiveTextRedactor.redact(trimmed) == trimmed else {
            return nil
        }
        let allowed = CharacterSet.alphanumerics.union(
            CharacterSet(charactersIn: "-._:/")
        )
        guard trimmed.unicodeScalars.allSatisfy(allowed.contains) else {
            return nil
        }
        return trimmed
    }
}

public struct ClaudeLocalActivitySummary: Codable, Equatable, Sendable {
    public let days: [ClaudeDailyActivity]
    public let sessionCount: Int
    public let activeDayCount: Int
    public let models: [ClaudeModelActivity]
    public let updatedAt: Date
    public let dayCount: Int

    public init(
        days: [ClaudeDailyActivity],
        sessionCount: Int,
        activeDayCount: Int,
        models: [ClaudeModelActivity],
        updatedAt: Date,
        dayCount: Int = 30
    ) {
        self.days = days
        self.sessionCount = max(sessionCount, 0)
        self.activeDayCount = max(activeDayCount, 0)
        self.models = models.filter {
            $0.tokenCount > 0 && $0.modelID != ClaudeModelActivity.unknownModelID
        }
        self.updatedAt = updatedAt
        self.dayCount = max(dayCount, 1)
    }

    public init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            days: try values.decode([ClaudeDailyActivity].self, forKey: .days),
            sessionCount: try values.decode(Int.self, forKey: .sessionCount),
            activeDayCount: try values.decode(Int.self, forKey: .activeDayCount),
            models: try values.decode([ClaudeModelActivity].self, forKey: .models),
            updatedAt: try values.decode(Date.self, forKey: .updatedAt),
            dayCount: try values.decode(Int.self, forKey: .dayCount)
        )
    }

    public var totalInputTokens: Int64 {
        days.reduce(0) { $0.addingClamped($1.inputTokens) }
    }

    public var totalOutputTokens: Int64 {
        days.reduce(0) { $0.addingClamped($1.outputTokens) }
    }

    public var totalCacheTokens: Int64 {
        days.reduce(0) { $0.addingClamped($1.cacheTokens) }
    }

    public var totalTokens: Int64 {
        days.reduce(0) { $0.addingClamped($1.totalTokens) }
    }
}

private extension Int64 {
    func addingClamped(_ value: Int64) -> Int64 {
        let (sum, overflow) = addingReportingOverflow(value)
        return overflow ? .max : Swift.max(sum, 0)
    }
}

public struct DeepSeekDailyUsage: Codable, Equatable, Identifiable, Sendable {
    public var id: Date { date }

    public let date: Date
    public let costCNY: Double
    public let requestCount: Int
    public let tokenCount: Int

    public init(
        date: Date,
        costCNY: Double,
        requestCount: Int,
        tokenCount: Int
    ) {
        self.date = date
        self.costCNY = costCNY
        self.requestCount = requestCount
        self.tokenCount = tokenCount
    }
}

public struct DeepSeekUsageHistory: Codable, Equatable, Sendable {
    public let days: [DeepSeekDailyUsage]
    public let updatedAt: Date
    public let statusMessage: String?

    public init(
        days: [DeepSeekDailyUsage],
        updatedAt: Date,
        statusMessage: String?
    ) {
        self.days = days
        self.updatedAt = updatedAt
        self.statusMessage = statusMessage
    }

    public var totalCostCNY: Double {
        days.reduce(0) { $0 + $1.costCNY }
    }

    public var totalRequests: Int {
        days.reduce(0) { $0 + $1.requestCount }
    }

    public var totalTokens: Int {
        days.reduce(0) { $0 + $1.tokenCount }
    }
}
