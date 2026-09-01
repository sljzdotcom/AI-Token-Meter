import AIMeterCore
import Charts
import SwiftUI

struct ClaudeDetailView: View {
    let snapshot: UsageSnapshot
    let onSetup: () -> Void

    private var presentation: ProviderPresentation {
        ProviderPresentation(snapshot: snapshot)
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                header
                officialQuotaSection
                Divider().overlay(Color.white.opacity(0.12))
                localActivitySection
                footer
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .scrollIndicators(.hidden)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .aiMeterDetailSurface()
    }

    private var header: some View {
        HStack(spacing: 10) {
            ProviderLogo(provider: .claude, size: 27)
            VStack(alignment: .leading, spacing: 2) {
                Text("Claude")
                    .aiMeterFont(.headline)
                    .foregroundStyle(valueStyle)
                Text("Official quota · Local Claude Code activity")
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
                Text("Official quota")
                    .aiMeterFont(.caption, weight: .semibold)
                Spacer()
                Text("Claude CLI")
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
            } else {
                VStack(alignment: .leading, spacing: 8) {
                    Text(presentation.detailText)
                        .aiMeterFont(.subheadline, weight: .semibold)
                    if let status = presentation.statusText {
                        Text(status)
                            .aiMeterFont(.caption)
                            .foregroundStyle(AIMeterVisualTheme.secondaryText)
                    }
                    if snapshot.collectionStatus == .setupRequired {
                        Button("Open one-time setup", action: onSetup)
                            .buttonStyle(.borderedProminent)
                            .controlSize(.small)
                    }
                }
                .padding(12)
                .frame(maxWidth: .infinity, alignment: .leading)
                .aiMeterGlassCard()
            }
        }
    }

    private func quotaCard(_ metric: UsageMetric, resetText: String?) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline) {
                Text(metric.label)
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
            Text(resetText ?? "Reset time unavailable")
                .aiMeterFont(.caption2)
                .foregroundStyle(AIMeterVisualTheme.tertiaryText)
                .lineLimit(1)
                .minimumScaleFactor(0.72)
        }
        .padding(11)
        .frame(maxWidth: .infinity, alignment: .leading)
        .aiMeterGlassCard()
        .accessibilityElement(children: .combine)
    }

    @ViewBuilder
    private var localActivitySection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Last 30 days · This Mac")
                    .aiMeterFont(.caption, weight: .semibold)
                Spacer()
                Text("Local estimate")
                    .aiMeterFont(.caption2)
                    .foregroundStyle(AIMeterVisualTheme.tertiaryText)
            }

            if let summary = snapshot.claudeLocalActivity {
                HStack(spacing: 8) {
                    localStat(title: "Sessions", value: summary.sessionCount.formatted(), symbol: "bubble.left.and.bubble.right")
                    localStat(title: "Active days", value: "\(summary.activeDayCount)/\(summary.dayCount)", symbol: "calendar")
                    localStat(title: "Tokens", value: compactCount(summary.totalTokens), symbol: "number")
                }
                activityChart(summary)
                tokenComposition(summary)
                if !summary.models.isEmpty {
                    modelBreakdown(summary)
                }
                Label(
                    "Only aggregate timestamps, token counts, session IDs and model IDs are read. Conversation content stays private.",
                    systemImage: "lock.shield"
                )
                .aiMeterFont(.caption2)
                .foregroundStyle(AIMeterVisualTheme.tertiaryText)
            } else {
                VStack(alignment: .leading, spacing: 7) {
                    Label("Local activity unavailable", systemImage: "chart.bar.xaxis")
                        .aiMeterFont(.subheadline, weight: .semibold)
                    Text("Official quota data above is unaffected. Local activity appears when Claude Code history is readable on this Mac.")
                        .aiMeterFont(.caption)
                        .foregroundStyle(AIMeterVisualTheme.secondaryText)
                }
                .padding(12)
                .frame(maxWidth: .infinity, alignment: .leading)
                .aiMeterGlassCard()
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
            Text(title)
                .aiMeterFont(.caption2)
                .foregroundStyle(AIMeterVisualTheme.secondaryText)
                .lineLimit(1)
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .aiMeterGlassCard()
        .accessibilityElement(children: .combine)
    }

    private func activityChart(_ summary: ClaudeLocalActivitySummary) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("Daily token activity")
                    .aiMeterFont(.subheadline, weight: .semibold)
                Spacer()
                Text(compactCount(summary.totalTokens))
                    .aiMeterFont(.caption, design: .rounded, weight: .semibold)
                    .foregroundStyle(valueStyle)
            }
            Chart(summary.days) { day in
                BarMark(
                    x: .value("Day", day.date, unit: .day),
                    y: .value("Tokens", day.totalTokens)
                )
                .foregroundStyle(valueStyle)
                .cornerRadius(2)
            }
            .chartXAxis {
                AxisMarks(values: .stride(by: .day, count: 7)) {
                    AxisGridLine().foregroundStyle(Color.white.opacity(0.06))
                    AxisValueLabel(format: .dateTime.month(.abbreviated).day())
                }
            }
            .chartYAxis(.hidden)
            .frame(height: 116)
        }
        .padding(12)
        .aiMeterGlassCard()
    }

    private func tokenComposition(_ summary: ClaudeLocalActivitySummary) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Token composition")
                .aiMeterFont(.subheadline, weight: .semibold)
            tokenRow("Input", value: summary.totalInputTokens, total: summary.totalTokens)
            tokenRow("Output", value: summary.totalOutputTokens, total: summary.totalTokens)
            tokenRow("Cache", value: summary.totalCacheTokens, total: summary.totalTokens)
        }
        .padding(12)
        .aiMeterGlassCard()
    }

    private func tokenRow(_ title: String, value: Int64, total: Int64) -> some View {
        let fraction = total > 0 ? Double(value) / Double(total) : 0
        return HStack(spacing: 8) {
            Text(title)
                .frame(width: 44, alignment: .leading)
            AIMeterProgressBar(provider: .claude, fraction: fraction, semantic: .normal)
            Text(compactCount(value))
                .frame(width: 48, alignment: .trailing)
                .foregroundStyle(AIMeterVisualTheme.secondaryText)
        }
        .aiMeterFont(.caption2)
    }

    private func modelBreakdown(_ summary: ClaudeLocalActivitySummary) -> some View {
        VStack(alignment: .leading, spacing: 7) {
            Text("Top models")
                .aiMeterFont(.subheadline, weight: .semibold)
            ForEach(Array(summary.models.prefix(3))) { model in
                HStack(spacing: 8) {
                    Circle()
                        .fill(valueStyle)
                        .frame(width: 6, height: 6)
                    Text(model.modelID)
                        .lineLimit(1)
                        .truncationMode(.middle)
                    Spacer()
                    Text(compactCount(model.tokenCount))
                        .foregroundStyle(AIMeterVisualTheme.secondaryText)
                }
                .aiMeterFont(.caption)
            }
        }
        .padding(12)
        .aiMeterGlassCard()
    }

    private var footer: some View {
        HStack {
            if let status = presentation.statusText {
                Text(status).lineLimit(1)
            }
            Spacer()
            Text("Updated \(snapshot.fetchedAt.formatted(date: .omitted, time: .shortened))")
        }
        .aiMeterFont(.caption2)
        .foregroundStyle(AIMeterVisualTheme.tertiaryText)
    }

    private var valueStyle: AnyShapeStyle {
        presentation.semantic.accentStyle(for: .claude)
    }

    private func percentText(_ metric: UsageMetric) -> String {
        guard let fraction = metric.usedFraction else { return "—" }
        return "\(Int((fraction * 100).rounded()))%"
    }

    private func compactCount(_ count: Int64) -> String {
        let value = Double(max(count, 0))
        let units: [(Double, String)] = [
            (1_000_000_000, "B"),
            (1_000_000, "M"),
            (1_000, "K"),
        ]
        guard let unit = units.first(where: { value >= $0.0 }) else {
            return count.formatted()
        }
        let scaled = value / unit.0
        let digits = scaled >= 100 ? 0 : 1
        return String(format: "%.*f", digits, scaled)
            .replacingOccurrences(of: ".0", with: "") + unit.1
    }
}
