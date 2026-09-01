import AIMeterCore
import Foundation

enum ClaudeDetailPresentation {
    static let localActivityEmptyTitle = "No local Claude Code activity"

    static func hasDailyActivity(_ summary: ClaudeLocalActivitySummary) -> Bool {
        summary.totalTokens > 0
    }

    static func officialQuotaAccessibilityLabel(
        _ metric: UsageMetric,
        resetText: String?
    ) -> String {
        let percentage = Int(((metric.usedFraction ?? 0) * 100).rounded())
        return "Official quota, \(metric.label), \(percentage) percent used, \(resetText ?? "Reset time unavailable")"
    }

    static func localStatAccessibilityLabel(title: String, value: String) -> String {
        "Local estimate, \(title), \(value)"
    }

    static func localActivityAccessibilityLabel(title: String, detail: String) -> String {
        "Local estimate, \(title), \(detail)"
    }
}
