import AIMeterCore
import Foundation

enum ClaudeDetailPresentation {
    static let localActivityEmptyTitle = "No local Claude Code activity"

    static func hasDailyActivity(_ summary: ClaudeLocalActivitySummary) -> Bool {
        summary.totalTokens > 0
    }

    static func officialQuotaAccessibilityLabel(
        _ metric: UsageMetric,
        resetText: String?,
        localizer: AppLocalizer = AppLocalizer(language: .english)
    ) -> String {
        let percentage = Int(((metric.usedFraction ?? 0) * 100).rounded())
        return localizer.text("Official quota, %@, %lld percent used, %@", ProviderDetailText.metricLabel(metric.label, localizer: localizer), Int64(percentage), resetText ?? localizer.text("Reset time unavailable"))
    }

    static func localStatAccessibilityLabel(title: String, value: String, localizer: AppLocalizer = AppLocalizer(language: .english)) -> String {
        localizer.text("Local estimate, %@, %@", localizer.text(title), value)
    }

    static func localActivityAccessibilityLabel(title: String, detail: String, localizer: AppLocalizer = AppLocalizer(language: .english)) -> String {
        localizer.text("Local estimate, %@, %@", localizer.text(title), localizer.text(detail))
    }
}
