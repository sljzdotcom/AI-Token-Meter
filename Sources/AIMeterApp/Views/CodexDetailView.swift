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
                CodexResetCreditsView(summary: credits)
                    .padding(11)
                    .background(cardBackground)
            }
            Divider().overlay(Color.white.opacity(0.12))
            localSection
            Spacer(minLength: 0)
            footer
        }
        .foregroundStyle(.white)
        .padding(18)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(
            RoundedRectangle(cornerRadius: 24, style: .continuous)
                .fill(Color.black.opacity(0.95))
                .shadow(color: .black.opacity(0.28), radius: 20, x: -5, y: 8)
        )
        .padding(5)
    }

    private var header: some View {
        HStack(spacing: 10) {
            ProviderLogo(provider: .codex, size: 27)
            VStack(alignment: .leading, spacing: 2) {
                Text("Codex")
                    .font(.headline)
                Text("Official quota · Local activity")
                    .font(.caption2)
                    .foregroundStyle(.white.opacity(0.54))
            }
            Spacer()
            Text(presentation.valueText)
                .font(.system(.title2, design: .rounded, weight: .bold))
                .foregroundStyle(presentation.semantic.color)
        }
    }

    private var quotaSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Official quota")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.white.opacity(0.62))
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
                    .foregroundStyle(.white.opacity(0.62))
                    .lineLimit(1)
                Spacer(minLength: 4)
                Text(percentText(metric))
                    .font(.system(.headline, design: .rounded, weight: .semibold))
            }
            ProgressView(value: metric.usedFraction ?? 0)
                .tint(presentation.semantic.color)
            Text(resetText ?? "Reset time unavailable")
                .font(.caption2)
                .foregroundStyle(.white.opacity(0.45))
                .lineLimit(1)
                .minimumScaleFactor(0.75)
        }
        .padding(11)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(cardBackground)
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
                    .foregroundStyle(.white.opacity(0.42))
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
                    .foregroundStyle(.white.opacity(0.42))
            } else {
                Text("Local Codex activity is unavailable; official quota data is unaffected.")
                    .font(.caption)
                    .foregroundStyle(.white.opacity(0.55))
                    .padding(11)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(cardBackground)
            }
        }
    }

    private func localStat(title: String, value: String, symbol: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Image(systemName: symbol)
                .font(.caption)
                .foregroundStyle(.mint)
            Text(value)
                .font(.system(.headline, design: .rounded, weight: .semibold))
                .lineLimit(1)
                .minimumScaleFactor(0.68)
            Text(title)
                .font(.caption2)
                .foregroundStyle(.white.opacity(0.5))
                .lineLimit(1)
                .minimumScaleFactor(0.75)
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(cardBackground)
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
        .foregroundStyle(.white.opacity(0.42))
    }

    private var cardBackground: some ShapeStyle {
        Color.white.opacity(0.075)
    }

    private func percentText(_ metric: UsageMetric) -> String {
        guard let fraction = metric.usedFraction else { return "—" }
        return "\(Int((fraction * 100).rounded()))%"
    }
}
