import AIMeterCore
import SwiftUI

enum CodexResetCreditsDisplayMode {
    case detail
    case compact
}

struct CodexResetCreditsView: View {
    let summary: CodexResetCreditsSummary
    let mode: CodexResetCreditsDisplayMode

    private var presentation: CodexResetCreditsPresentation {
        CodexResetCreditsPresentation(summary: summary)
    }

    @ViewBuilder
    var body: some View {
        switch mode {
        case .detail:
            detailContent
        case .compact:
            compactContent
        }
    }

    private var detailContent: some View {
        VStack(alignment: .leading, spacing: 11) {
            HStack(spacing: 9) {
                Image(systemName: "arrow.counterclockwise.circle.fill")
                    .font(.system(size: 15, weight: .semibold))
                    .foregroundStyle(.mint)
                Text("Reset credits")
                    .font(.subheadline.weight(.semibold))
                Spacer()
                Text(presentation.availableText)
                    .font(.caption2.weight(.bold))
                    .foregroundStyle(.mint)
                    .padding(.horizontal, 9)
                    .padding(.vertical, 5)
                    .background(
                        Capsule()
                            .fill(Color.mint.opacity(0.12))
                            .stroke(Color.mint.opacity(0.24), lineWidth: 1)
                    )
            }
            .accessibilityElement(children: .combine)

            ForEach(Array(presentation.rows.enumerated()), id: \.offset) { _, row in
                detailRow(row)
            }

            if presentation.showsIncompleteDetails {
                Label(
                    "Some expiration details are unavailable",
                    systemImage: "info.circle"
                )
                .font(.caption2)
                .foregroundStyle(.white.opacity(0.48))
            }
        }
    }

    private func detailRow(_ row: CodexResetCreditRowPresentation) -> some View {
        VStack(alignment: .leading, spacing: 11) {
            HStack(spacing: 11) {
                Image(systemName: "arrow.counterclockwise")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(.mint)
                    .frame(width: 34, height: 34)
                    .background(Color.mint.opacity(0.10), in: RoundedRectangle(cornerRadius: 10))

                VStack(alignment: .leading, spacing: 3) {
                    Text(row.title)
                        .font(.caption.weight(.semibold))
                        .lineLimit(1)
                    Text(expirationDateText(row.expiresAt))
                        .font(.subheadline.weight(.semibold))
                        .lineLimit(1)
                        .minimumScaleFactor(0.75)
                }
                Spacer(minLength: 0)
            }

            HStack {
                Text("Expiration")
                    .foregroundStyle(.white.opacity(0.46))
                Spacer()
                Text(row.statusText)
                    .fontWeight(.semibold)
                    .foregroundStyle(statusColor(row.expirationState))
            }
            .font(.caption2)
        }
        .padding(13)
        .background(
            RoundedRectangle(cornerRadius: 15, style: .continuous)
                .fill(Color.white.opacity(0.055))
                .stroke(Color.white.opacity(0.07), lineWidth: 1)
        )
        .accessibilityElement(children: .combine)
        .accessibilityLabel(accessibilityLabel(row))
    }

    private var compactContent: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Label("Reset credits", systemImage: "arrow.counterclockwise.circle")
                    .fontWeight(.semibold)
                Spacer()
                Text(presentation.availableText)
                    .fontWeight(.semibold)
            }

            ForEach(Array(presentation.rows.enumerated()), id: \.offset) { _, row in
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    Text(row.title)
                        .lineLimit(1)
                    Spacer()
                    Text(expirationDateText(row.expiresAt))
                        .foregroundStyle(.tertiary)
                        .lineLimit(1)
                        .minimumScaleFactor(0.75)
                }
                .accessibilityElement(children: .combine)
                .accessibilityLabel(accessibilityLabel(row))
            }

            if presentation.showsIncompleteDetails {
                Text("Some expiration details are unavailable")
                    .foregroundStyle(.tertiary)
            }
        }
        .font(.caption2)
        .foregroundStyle(.secondary)
    }

    private func expirationDateText(_ expiresAt: Date?) -> String {
        guard let expiresAt else { return "Date unavailable" }
        return expiresAt.formatted(date: .abbreviated, time: .shortened)
    }

    private func statusColor(_ state: CodexResetCreditExpirationState) -> Color {
        switch state {
        case .remaining: .mint
        case .today: .orange
        case .expired: .red
        case .unavailable: .white.opacity(0.48)
        }
    }

    private func accessibilityLabel(_ row: CodexResetCreditRowPresentation) -> String {
        "\(row.title), \(expirationDateText(row.expiresAt)), \(row.statusText)"
    }
}
