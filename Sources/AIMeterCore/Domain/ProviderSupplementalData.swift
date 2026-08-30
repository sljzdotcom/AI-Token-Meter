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
