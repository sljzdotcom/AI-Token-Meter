import AIMeterCore
import SwiftUI

struct ProviderCard: View {
    let snapshot: UsageSnapshot
    let onClaudeSetup: (() -> Void)?

    init(snapshot: UsageSnapshot, onClaudeSetup: (() -> Void)? = nil) {
        self.snapshot = snapshot
        self.onClaudeSetup = onClaudeSetup
    }

    private var presentation: ProviderPresentation {
        ProviderPresentation(snapshot: snapshot)
    }

    var body: some View {
        HStack(spacing: 14) {
            UsageRing(presentation: presentation, size: 54)
                .padding(5)
                .background(Color.black.opacity(0.88), in: Circle())

            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(presentation.title)
                        .font(.headline)
                    Spacer()
                    Text(presentation.valueText)
                        .font(.system(.title3, design: .rounded, weight: .semibold))
                        .foregroundStyle(presentation.semantic.color)
                }
                Text(presentation.detailText)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                if let reset = presentation.primaryResetText {
                    Text(reset)
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }
                if let secondary = snapshot.secondaryMetric {
                    metricLine(secondary)
                    if let reset = presentation.secondaryResetText {
                        Text(reset)
                            .font(.caption2)
                            .foregroundStyle(.tertiary)
                    }
                }
                if snapshot.provider == .codex,
                   let resetCredits = snapshot.codexResetCredits {
                    CodexResetCreditsView(summary: resetCredits)
                        .padding(.top, 2)
                }
                HStack(spacing: 6) {
                    if let status = presentation.statusText {
                        Text(status)
                    }
                    Spacer()
                    Text("Updated \(snapshot.fetchedAt.formatted(date: .omitted, time: .shortened))")
                }
                .font(.caption2)
                .foregroundStyle(.secondary)
                if snapshot.provider == .claude,
                   snapshot.collectionStatus == .setupRequired,
                   let onClaudeSetup {
                    Button("Open one-time setup", action: onClaudeSetup)
                        .buttonStyle(.borderedProminent)
                        .controlSize(.small)
                }
            }
        }
        .padding(12)
        .background(.quaternary.opacity(0.45), in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }

    private func metricLine(_ metric: UsageMetric) -> some View {
        HStack(spacing: 4) {
            Text(metric.label)
            Spacer()
            if let fraction = metric.usedFraction {
                Text("\(Int((fraction * 100).rounded()))%")
            }
        }
        .font(.caption2)
        .foregroundStyle(.secondary)
    }
}
