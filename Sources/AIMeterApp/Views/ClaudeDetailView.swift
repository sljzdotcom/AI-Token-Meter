import AIMeterCore
import Charts
import SwiftUI

struct ClaudeDetailView: View {
    @Environment(\.locale) private var locale
    private var localizer: AppLocalizer { AppLocalizer(locale: locale) }
    let snapshot: UsageSnapshot
    let onSetup: () -> Void
    let onOpenServicesSettings: () -> Void

    private var presentation: AppProviderPresentation {
        AppProviderPresentation(snapshot: snapshot, localizer: localizer)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            header
            officialQuotaSection
            Divider().overlay(Color.white.opacity(0.12))
            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
                localActivitySection
                footer
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .scrollIndicators(.hidden)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .aiMeterDetailSurface()
    }

    private var header: some View {
        HStack(spacing: 10) {
            ProviderLogo(provider: .claude, size: 27)
            VStack(alignment: .leading, spacing: 2) {
                Text(snapshot.provider.displayName)
                    .aiMeterFont(.headline)
                    .foregroundStyle(valueStyle)
                Text(localizer.text("Official quota · Local Claude Code activity"))
                    .aiMeterFont(.caption2)
                    .foregroundStyle(AIMeterVisualTheme.secondaryText)
            }
            Spacer(minLength: 8)
            Text(presentation.valueText)
                .aiMeterFont(.title2, design: .rounded, weight: .bold)
                .foregroundStyle(valueStyle)
        }
    }

    private var officialQuotaSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(localizer.text("Official quota"))
                    .aiMeterFont(.caption, weight: .semibold)
                Spacer()
                Text("Claude Code CLI")
                    .aiMeterFont(.caption2)
                    .foregroundStyle(AIMeterVisualTheme.tertiaryText)
            }
            .foregroundStyle(AIMeterVisualTheme.secondaryText)

            if snapshot.primaryMetric != nil || snapshot.secondaryMetric != nil {
                HStack(spacing: 9) {
                    if let metric = snapshot.primaryMetric {
                        quotaCard(metric, resetText: presentation.primaryResetText)
                    }
                    if let metric = snapshot.secondaryMetric {
                        quotaCard(metric, resetText: presentation.secondaryResetText)
                    }
                }
                recoveryButton
            } else {
                VStack(alignment: .leading, spacing: 8) {
                    Text(presentation.detailText)
                        .aiMeterFont(.subheadline, weight: .semibold)
                    if let status = presentation.statusText {
                        Text(status)
                            .aiMeterFont(.caption)
                            .foregroundStyle(AIMeterVisualTheme.secondaryText)
                    }
                    recoveryButton
                }
                .padding(12)
                .frame(maxWidth: .infinity, alignment: .leading)
                .aiMeterGlassCard()
            }
        }
    }

    @ViewBuilder
    private var recoveryButton: some View {
        if let recovery = ServiceRecoveryPresentation.detailAction(
            provider: .claude,
            status: snapshot.collectionStatus,
            statusMessage: snapshot.statusMessage
        ) {
            Button(
                recovery == .openClaudeWorkspace
                    ? localizer.text("Open one-time setup")
                    : localizer.text("Open Services Settings"),
                action: recovery == .openClaudeWorkspace
                    ? onSetup
                    : onOpenServicesSettings
            )
                .buttonStyle(.borderedProminent)
                .controlSize(.small)
        }
    }

    private func quotaCard(_ metric: UsageMetric, resetText: String?) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline) {
                Text(ProviderDetailText.metricLabel(metric.label, localizer: localizer))
                    .aiMeterFont(.caption)
                    .foregroundStyle(AIMeterVisualTheme.secondaryText)
                    .lineLimit(1)
                Spacer(minLength: 4)
                Text(percentText(metric))
                    .aiMeterFont(.headline, design: .rounded, weight: .semibold)
                    .foregroundStyle(valueStyle)
            }
            AIMeterProgressBar(
                provider: .claude,
                fraction: metric.usedFraction ?? 0,
                semantic: presentation.semantic
            )
            Text(resetText ?? localizer.text("Reset time unavailable"))
                .aiMeterFont(.caption2)
                .foregroundStyle(AIMeterVisualTheme.tertiaryText)
                .lineLimit(1)
                .minimumScaleFactor(0.72)
        }
        .padding(11)
        .frame(maxWidth: .infinity, alignment: .leading)
        .aiMeterGlassCard()
        .accessibilityElement(children: .combine)
        .accessibilityLabel(
            ClaudeDetailPresentation.officialQuotaAccessibilityLabel(metric, resetText: resetText, localizer: localizer)
        )
    }

    @ViewBuilder
    private var localActivitySection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text(localizer.text("Last 30 days · This Mac"))
                    .aiMeterFont(.caption, weight: .semibold)
                Spacer()
                Text(localizer.text("Local estimate"))
                    .aiMeterFont(.caption2)
                    .foregroundStyle(AIMeterVisualTheme.tertiaryText)
            }

            if let summary = snapshot.claudeLocalActivity {
                HStack(spacing: 8) {
                    localStat(title: "Sessions", value: localizer.number(Int64(summary.sessionCount)), symbol: "bubble.left.and.bubble.right")
                    localStat(title: "Active days", value: "\(summary.activeDayCount)/\(summary.dayCount)", symbol: "calendar")
                    localStat(title: "Tokens", value: compactCount(summary.totalTokens), symbol: "number")
                }
                if ClaudeDetailPresentation.hasDailyActivity(summary) {
                    activityChart(summary)
                } else {
                    localActivityEmptyState
                }
            } else {
                VStack(alignment: .leading, spacing: 7) {
                    Label(localizer.text("Local activity unavailable"), systemImage: "chart.bar.xaxis")
                        .aiMeterFont(.subheadline, weight: .semibold)
                    Text(localizer.text("Official quota data above is unaffected. Local activity appears when Claude Code history is readable on this Mac."))
                        .aiMeterFont(.caption)
                        .foregroundStyle(AIMeterVisualTheme.secondaryText)
                }
                .padding(12)
                .frame(maxWidth: .infinity, alignment: .leading)
                .aiMeterGlassCard()
                .accessibilityElement(children: .combine)
                .accessibilityLabel(
                    ClaudeDetailPresentation.localActivityAccessibilityLabel(
                        title: "Status",
                        detail: "Local activity unavailable",
                        localizer: localizer
                    )
                )
            }
        }
    }

    private func localStat(title: String, value: String, symbol: String) -> some View {
        VStack(alignment: .leading, spacing: 5) {
            Image(systemName: symbol)
                .aiMeterSymbolFont(.caption)
                .foregroundStyle(valueStyle)
            Text(value)
                .aiMeterFont(.headline, design: .rounded, weight: .semibold)
                .foregroundStyle(valueStyle)
                .lineLimit(1)
                .minimumScaleFactor(0.65)
            Text(localizer.text(title))
                .aiMeterFont(.caption2)
                .foregroundStyle(AIMeterVisualTheme.secondaryText)
                .lineLimit(1)
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .aiMeterGlassCard()
        .accessibilityElement(children: .combine)
        .accessibilityLabel(
            ClaudeDetailPresentation.localStatAccessibilityLabel(
                title: title,
                value: value,
                localizer: localizer
            )
        )
    }

    private var localActivityEmptyState: some View {
        VStack(alignment: .leading, spacing: 6) {
            Label(
                localizer.text(ClaudeDetailPresentation.localActivityEmptyTitle),
                systemImage: "chart.bar.xaxis"
            )
            .aiMeterFont(.subheadline, weight: .semibold)
            Text(localizer.text("No token activity was found in the current 30-day window on this Mac."))
                .aiMeterFont(.caption)
                .foregroundStyle(AIMeterVisualTheme.secondaryText)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .aiMeterGlassCard()
        .accessibilityLabel(
            localizer.text("Local estimate, %@", localizer.text(ClaudeDetailPresentation.localActivityEmptyTitle))
        )
    }

    private func activityChart(_ summary: ClaudeLocalActivitySummary) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(localizer.text("Daily token activity"))
                    .aiMeterFont(.subheadline, weight: .semibold)
                Spacer()
                Text(compactCount(summary.totalTokens))
                    .aiMeterFont(.caption, design: .rounded, weight: .semibold)
                    .foregroundStyle(valueStyle)
            }
            Chart(summary.days) { day in
                BarMark(
                    x: .value(localizer.text("Day"), day.date, unit: .day),
                    y: .value(localizer.text("Tokens"), day.totalTokens)
                )
                .foregroundStyle(valueStyle)
                .cornerRadius(2)
            }
            .chartXAxis {
                AxisMarks(values: .stride(by: .day, count: 7)) {
                    AxisGridLine().foregroundStyle(Color.white.opacity(0.06))
                    AxisValueLabel(format: .dateTime.month(.abbreviated).day().locale(localizer.language.locale))
                }
            }
            .chartYAxis(.hidden)
            .frame(height: 116)
        }
        .padding(12)
        .aiMeterGlassCard()
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(
            ClaudeDetailPresentation.localActivityAccessibilityLabel(
                title: "Daily token activity",
                detail: localizer.text("%@ total tokens", localizer.number(summary.totalTokens)),
                localizer: localizer
            )
        )
    }

    private var footer: some View {
        HStack {
            if let status = presentation.statusText {
                Text(status).lineLimit(1)
            }
            Spacer()
            Text(ProviderDetailText.freshness(snapshot, localizer: localizer) + " · " + localizer.text("Updated %@", localizer.date(snapshot.fetchedAt, dateStyle: .none, timeStyle: .short)))
        }
        .aiMeterFont(.caption2)
        .foregroundStyle(AIMeterVisualTheme.tertiaryText)
    }

    private var valueStyle: AnyShapeStyle {
        presentation.semantic.accentStyle(for: .claude)
    }

    private func percentText(_ metric: UsageMetric) -> String {
        guard let fraction = metric.usedFraction else { return "—" }
        return localizer.percentage(fraction)
    }

    private func compactCount(_ count: Int64) -> String {
        ProviderDetailText.compactCount(count, localizer: localizer)
    }
}
