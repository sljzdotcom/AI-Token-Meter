import Foundation

/// Parses the secret-free, tab-separated report produced by `agy -p /usage`.
/// The CLI reports remaining quota; UsageMetric stores the consumed percentage.
public struct GeminiUsageParser: Sendable {
    public init() {}

    public func parse(_ text: String, sourceVersion: String = "1.1.28") throws -> UsageSnapshot {
        let rows = text.components(separatedBy: .newlines)
            .map { ANSITextSanitizer.sanitize($0).trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
        guard rows.count == QuotaKey.allCases.count else {
            throw UsageCollectionError.unrecognizedOutput
        }

        var byKey: [QuotaKey: UsageMetric] = [:]
        for row in rows {
            let columns = row.split(separator: "\t", omittingEmptySubsequences: false).map(String.init)
            guard columns.count == 4,
                  let key = QuotaKey(group: columns[0], window: columns[1]),
                  byKey[key] == nil,
                  let remaining = Self.percent(columns[2]),
                  let resetAt = Self.date(columns[3]) else {
                throw UsageCollectionError.unrecognizedOutput
            }
            byKey[key] = UsageMetric(
                label: key.label,
                current: 100 - remaining,
                limit: 100,
                unit: .percent,
                resetAt: resetAt
            )
        }

        let metrics = try QuotaKey.allCases.map { key in
            guard let metric = byKey[key] else { throw UsageCollectionError.unrecognizedOutput }
            return metric
        }
        let ranked = metrics.enumerated().sorted { left, right in
            left.element.current == right.element.current
                ? left.offset < right.offset
                : left.element.current > right.element.current
        }.map(\.element)
        return UsageSnapshot(
            provider: .gemini,
            primaryMetric: ranked[0],
            secondaryMetric: ranked[1],
            sourceVersion: sourceVersion,
            geminiQuotaMetrics: metrics
        )
    }

    private static func percent(_ value: String) -> Double? {
        guard value.last == "%",
              let number = Double(value.dropLast()),
              number.isFinite,
              (0...100).contains(number) else { return nil }
        return number
    }

    private static func date(_ value: String) -> Date? {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        if let date = formatter.date(from: value) { return date }
        formatter.formatOptions = [.withInternetDateTime]
        return formatter.date(from: value)
    }
}

private enum QuotaKey: CaseIterable, Hashable {
    case geminiFiveHour
    case geminiWeekly
    case otherFiveHour
    case otherWeekly

    init?(group: String, window: String) {
        switch (group, window) {
        case ("Gemini Models", "Five Hour Limit Remaining"): self = .geminiFiveHour
        case ("Gemini Models", "Weekly Limit Remaining"): self = .geminiWeekly
        case ("Claude and GPT models", "Five Hour Limit Remaining"): self = .otherFiveHour
        case ("Claude and GPT models", "Weekly Limit Remaining"): self = .otherWeekly
        default: return nil
        }
    }

    var label: String {
        switch self {
        case .geminiFiveHour: "Gemini · Five hour"
        case .geminiWeekly: "Gemini · Weekly"
        case .otherFiveHour: "Claude/GPT · Five hour"
        case .otherWeekly: "Claude/GPT · Weekly"
        }
    }
}
