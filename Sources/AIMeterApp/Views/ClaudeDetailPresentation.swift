import AIMeterCore
import Foundation

struct ClaudeModelRowPresentation: Equatable, Identifiable {
    var id: String { modelID }

    let modelID: String
    let tokenCount: Int64
    let sharePercent: Int
}

enum ClaudeDetailPresentation {
    static let localActivityEmptyTitle = "No local Claude Code activity"

    static func hasDailyActivity(_ summary: ClaudeLocalActivitySummary) -> Bool {
        summary.totalTokens > 0
    }

    static func topModelRows(
        _ summary: ClaudeLocalActivitySummary
    ) -> [ClaudeModelRowPresentation] {
        let models = summary.models
            .filter { $0.tokenCount > 0 }
            .sorted {
                if $0.tokenCount == $1.tokenCount { return $0.modelID < $1.modelID }
                return $0.tokenCount > $1.tokenCount
            }
        let total = models.reduce(Int64(0)) { partial, model in
            let (sum, overflow) = partial.addingReportingOverflow(model.tokenCount)
            return overflow ? .max : sum
        }
        guard total > 0 else { return [] }

        return models.prefix(3).map { model in
            ClaudeModelRowPresentation(
                modelID: model.modelID,
                tokenCount: model.tokenCount,
                sharePercent: Int((Double(model.tokenCount) / Double(total) * 100).rounded())
            )
        }
    }

    static func officialQuotaAccessibilityLabel(_ metric: UsageMetric) -> String {
        let percentage = Int(((metric.usedFraction ?? 0) * 100).rounded())
        return "Official quota, \(metric.label), \(percentage) percent used"
    }

    static func localStatAccessibilityLabel(title: String, value: String) -> String {
        "Local estimate, \(title), \(value)"
    }
}
