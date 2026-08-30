import AIMeterCore
import SwiftUI

struct CodexResetCreditsView: View {
    let summary: CodexResetCreditsSummary

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Label("Reset credits", systemImage: "arrow.counterclockwise.circle")
                Spacer()
                Text("\(summary.availableCount) available")
                    .fontWeight(.semibold)
            }

            ForEach(Array(summary.credits.enumerated()), id: \.offset) { _, credit in
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    Text(credit.title ?? "Usage reset")
                        .lineLimit(1)
                    Spacer()
                    Text(expirationText(for: credit))
                        .foregroundStyle(.tertiary)
                }
            }

            if summary.availableCount > 0, !summary.hasCompleteDetails {
                Text("Expiration details unavailable")
                    .foregroundStyle(.tertiary)
            }
        }
        .font(.caption2)
        .foregroundStyle(.secondary)
        .accessibilityElement(children: .combine)
    }

    private func expirationText(for credit: CodexResetCreditDisplay) -> String {
        guard let expiresAt = credit.expiresAt else { return "No expiration provided" }
        return "Expires \(expiresAt.formatted(date: .abbreviated, time: .shortened))"
    }
}
