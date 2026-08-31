import AIMeterCore
import SwiftUI

struct CodexDetailView: View {
    let snapshot: UsageSnapshot

    private var presentation: ProviderPresentation {
        ProviderPresentation(snapshot: snapshot)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 13) {
            header
            quotaSection
            if let credits = snapshot.codexResetCredits {
                CodexResetCreditsView(summary: credits, mode: .detail)
            }
            Divider().overlay(Color.white.opacity(0.12))
            localSection
            Spacer(minLength: 0)
            footer
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .aiMeterDetailSurface()
    }

    private var header: some View {
        HStack(spacing: 10) {
            ProviderLogo(provider: .codex, size: 27)
            VStack(alignment: .leading, spacing: 2) {
                Text("Codex")
                    .font(.headline)
                Text("Official quota · Local activity")
                    .font(.caption2)
                    .foregroundStyle(AIMeterVisualTheme.secondaryText)
            }
            Spacer()
            Text(presentation.valueText)
                .font(.system(.title2, design: .rounded, weight: .bold))
                .foregroundStyle(valueStyle)
        }
    }

    private var quotaSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Official quota")
                .font(.caption.weight(.semibold))
                .foregroundStyle(AIMeterVisualTheme.secondaryText)
            HStack(spacing: 9) {
                if let metric = snapshot.primaryMetric {
                    quotaCard(metric, resetText: presentation.primaryResetText)
                }
                if let metric = snapshot.secondaryMetric {
                    quotaCard(metric, resetText: presentation.secondaryResetText)
                }
            }
        }
    }

    private func quotaCard(_ metric: UsageMetric, resetText: String?) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline) {
                Text(metric.label)
                    .font(.caption)
                    .foregroundStyle(AIMeterVisualTheme.secondaryText)
                    .lineLimit(1)
                Spacer(minLength: 4)
                Text(percentText(metric))
                    .font(.system(.headline, design: .rounded, weight: .semibold))
            }
            AIMeterProgressBar(
                fraction: metric.usedFraction ?? 0,
                semantic: presentation.semantic
            )
            Text(resetText ?? "Reset time unavailable")
                .font(.caption2)
                .foregroundStyle(AIMeterVisualTheme.tertiaryText)
                .lineLimit(1)
                .minimumScaleFactor(0.75)
        }
        .padding(11)
        .frame(maxWidth: .infinity, alignment: .leading)
        .aiMeterGlassCard()
        .accessibilityElement(children: .combine)
    }

    @ViewBuilder
    private var localSection: some View {
        VStack(alignment: .leading, spacing: 9) {
            HStack {
                Text("Last 30 days · This Mac")
                    .font(.caption.weight(.semibold))
                Spacer()
                Text("Local estimate")
                    .font(.caption2)
                    .foregroundStyle(AIMeterVisualTheme.tertiaryText)
            }
            if let summary = snapshot.codexLocalActivity {
                let values = CodexLocalActivityPresentation(summary: summary)
                HStack(spacing: 8) {
                    localStat(title: "Token", value: values.tokenText, symbol: "number")
                    localStat(title: "Current streak", value: values.streakText, symbol: "flame")
                    localStat(title: "Longest session", value: values.longestSessionText, symbol: "clock")
                }
                Text("Counts only aggregate Codex thread activity readable on this Mac.")
                    .font(.caption2)
                    .foregroundStyle(AIMeterVisualTheme.tertiaryText)
            } else {
                Text("Local Codex activity is unavailable; official quota data is unaffected.")
                    .font(.caption)
                    .foregroundStyle(AIMeterVisualTheme.secondaryText)
                    .padding(11)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .aiMeterGlassCard()
            }
        }
    }

    private func localStat(title: String, value: String, symbol: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Image(systemName: symbol)
                .font(.caption)
                .foregroundStyle(AIMeterVisualTheme.mintAccent)
            Text(value)
                .font(.system(.headline, design: .rounded, weight: .semibold))
                .lineLimit(1)
                .minimumScaleFactor(0.68)
            Text(title)
                .font(.caption2)
                .foregroundStyle(AIMeterVisualTheme.secondaryText)
                .lineLimit(1)
                .minimumScaleFactor(0.75)
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .aiMeterGlassCard()
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(title), \(value), local estimate")
    }

    private var footer: some View {
        HStack {
            if let status = presentation.statusText {
                Text(status)
                    .lineLimit(1)
            }
            Spacer()
            Text("Updated \(snapshot.fetchedAt.formatted(date: .omitted, time: .shortened))")
        }
        .font(.caption2)
        .foregroundStyle(AIMeterVisualTheme.tertiaryText)
    }

    private var valueStyle: AnyShapeStyle {
        if presentation.semantic == .normal {
            return AnyShapeStyle(AIMeterVisualTheme.accentGradient)
        }
        return AnyShapeStyle(presentation.semantic.color)
    }

    private func percentText(_ metric: UsageMetric) -> String {
        guard let fraction = metric.usedFraction else { return "—" }
        return "\(Int((fraction * 100).rounded()))%"
    }
}
