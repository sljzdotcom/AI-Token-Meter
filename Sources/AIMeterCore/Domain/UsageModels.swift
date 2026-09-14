import Foundation

public enum UsageProvider: String, Codable, CaseIterable, Hashable, Sendable {
    case claude
    case codex
    case deepSeek
    case gemini

    public var displayName: String {
        switch self {
        case .claude: "Claude Code"
        case .codex: "OpenAI Codex"
        case .deepSeek: "DeepSeek"
        case .gemini: "Google Antigravity"
        }
    }
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

public struct AntigravityCLIInfo: Codable, Equatable, Sendable {
    public let currentModel: String?
    public let availableModelCount: Int?
    public let modelFamilies: [String]

    public init(
        currentModel: String? = nil,
        availableModelCount: Int? = nil,
        modelFamilies: [String] = []
    ) {
        self.currentModel = currentModel
        self.availableModelCount = availableModelCount
        self.modelFamilies = modelFamilies
    }

    static func isValidGeminiDisplayName(_ value: String) -> Bool {
        guard value.count <= 120,
              value.unicodeScalars.allSatisfy({ scalar in
                  scalar.isASCII && (CharacterSet.alphanumerics.contains(scalar)
                      || " .-()".unicodeScalars.contains(scalar))
              }) else { return false }
        let words = value.split(separator: " ", omittingEmptySubsequences: false)
        guard words.count >= 3,
              words[0] == "Gemini",
              !words.contains(where: \.isEmpty) else { return false }
        let versionParts = words[1].split(separator: ".", omittingEmptySubsequences: false)
        guard versionParts.count >= 2,
              versionParts.allSatisfy({ !$0.isEmpty && $0.allSatisfy(\.isNumber) }) else {
            return false
        }
        let forbidden = ["bearer", "claude", "gpt", "key", "secret", "token", "sk-", "dk-"]
        let lowered = value.lowercased()
        guard !forbidden.contains(where: lowered.contains) else { return false }
        for (index, word) in words.dropFirst(2).enumerated() where word.contains("(") || word.contains(")") {
            guard index == words.count - 3,
                  ["(High)", "(Medium)", "(Low)"].contains(String(word)) else { return false }
        }
        return true
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
    public let claudeLocalActivity: ClaudeLocalActivitySummary?
    public let geminiQuotaMetrics: [UsageMetric]?
    public let antigravityCLIInfo: AntigravityCLIInfo?
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
        claudeLocalActivity: ClaudeLocalActivitySummary? = nil,
        geminiQuotaMetrics: [UsageMetric]? = nil,
        antigravityCLIInfo: AntigravityCLIInfo? = nil,
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
        self.claudeLocalActivity = claudeLocalActivity
        self.geminiQuotaMetrics = geminiQuotaMetrics
        self.antigravityCLIInfo = antigravityCLIInfo
        self.deepSeekUsageHistory = deepSeekUsageHistory
    }

    public func isStale(at date: Date = Date()) -> Bool {
        date >= fetchedAt.addingTimeInterval(staleAfter)
    }
}

public extension UsageSnapshot {
    func normalizedAntigravityQuota() -> UsageSnapshot {
        guard provider == .gemini else { return self }
        let normalizedCLIInfo = normalizedAntigravityCLIInfo()
        let expectedLabels = ["Gemini · Five hour", "Gemini · Weekly"]
        let candidates = geminiQuotaMetrics ?? [primaryMetric, secondaryMetric].compactMap { $0 }
        let byLabel = Dictionary(grouping: candidates, by: \.label)
        let metrics = expectedLabels.compactMap { label -> UsageMetric? in
            guard let matches = byLabel[label], matches.count == 1 else { return nil }
            let metric = matches[0]
            guard metric.kind == .officialLimit,
                  metric.unit == .percent,
                  metric.limit == 100,
                  metric.current.isFinite,
                  (0...100).contains(metric.current),
                  metric.resetAt != nil else { return nil }
            return metric
        }
        let complete = metrics.count == expectedLabels.count
        let published = complete ? metrics : []
        let ranked = published.enumerated().sorted { left, right in
            left.element.current == right.element.current
                ? left.offset < right.offset
                : left.element.current > right.element.current
        }.map(\.element)
        return UsageSnapshot(
            provider: provider,
            primaryMetric: ranked.first,
            secondaryMetric: ranked.dropFirst().first,
            availability: availability,
            fetchedAt: fetchedAt,
            staleAfter: staleAfter,
            sourceVersion: sourceVersion,
            collectionStatus: collectionStatus,
            statusMessage: statusMessage,
            codexResetCredits: codexResetCredits,
            codexLocalActivity: codexLocalActivity,
            claudeLocalActivity: claudeLocalActivity,
            geminiQuotaMetrics: published,
            antigravityCLIInfo: normalizedCLIInfo,
            deepSeekUsageHistory: deepSeekUsageHistory
        )
    }

    private func normalizedAntigravityCLIInfo() -> AntigravityCLIInfo? {
        guard let info = antigravityCLIInfo else { return nil }
        let validModel = AntigravityCLIInfo.isValidGeminiDisplayName
        guard info.currentModel.map(validModel) ?? true,
              info.availableModelCount.map({ (1...64).contains($0) }) ?? true,
              info.modelFamilies.count <= 16,
              Set(info.modelFamilies).count == info.modelFamilies.count,
              info.modelFamilies.allSatisfy(validModel),
              info.currentModel != nil || info.availableModelCount != nil || !info.modelFamilies.isEmpty
        else { return nil }
        return info
    }

    func withAntigravityCLIInfo(_ info: AntigravityCLIInfo?) -> UsageSnapshot {
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
            codexLocalActivity: codexLocalActivity,
            claudeLocalActivity: claudeLocalActivity,
            geminiQuotaMetrics: geminiQuotaMetrics,
            antigravityCLIInfo: info,
            deepSeekUsageHistory: deepSeekUsageHistory
        )
    }

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
            claudeLocalActivity: claudeLocalActivity,
            geminiQuotaMetrics: geminiQuotaMetrics,
            antigravityCLIInfo: antigravityCLIInfo,
            deepSeekUsageHistory: deepSeekUsageHistory
        )
    }

    func withClaudeLocalActivity(_ activity: ClaudeLocalActivitySummary?) -> UsageSnapshot {
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
            codexLocalActivity: codexLocalActivity,
            claudeLocalActivity: activity,
            geminiQuotaMetrics: geminiQuotaMetrics,
            antigravityCLIInfo: antigravityCLIInfo,
            deepSeekUsageHistory: deepSeekUsageHistory
        )
    }
}
