import AIMeterCore
import SwiftUI

enum CodexResetCreditsDisplayMode {
    case detail
    case compact
}

struct CodexResetCreditsView: View {
    @Environment(\.locale) private var locale
    private var localizer: AppLocalizer { AppLocalizer(locale: locale) }
    let summary: CodexResetCreditsSummary
    let mode: CodexResetCreditsDisplayMode

    private var presentation: CodexResetCreditsPresentation {
        CodexResetCreditsPresentation(summary: summary)
    }

    private var accentStart: Color {
        UsageProvider.codex.accentPalette.startColor
    }

    private var accentEnd: Color {
        UsageProvider.codex.accentPalette.endColor
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
                    .foregroundStyle(accentStart)
                Text(localizer.text("Reset credits"))
                    .aiMeterFont(.subheadline, weight: .semibold)
                Spacer()
                Text(localizer.text("%lld available", Int64(summary.availableCount)))
                    .aiMeterFont(.caption2, weight: .bold)
                    .foregroundStyle(accentStart)
                    .padding(.horizontal, 9)
                    .padding(.vertical, 5)
                    .background(
                        Capsule()
                            .fill(accentStart.opacity(0.12))
                    )
            }
            .accessibilityElement(children: .combine)

            ForEach(Array(presentation.rows.enumerated()), id: \.offset) { _, row in
                detailRow(row)
            }

            if presentation.showsIncompleteDetails {
                Label {
                    Text(localizer.text("Some expiration details are unavailable"))
                } icon: {
                    Image(systemName: "info.circle")
                        .aiMeterSymbolFont(.caption2)
                }
                .aiMeterFont(.caption2)
                .foregroundStyle(AIMeterVisualTheme.tertiaryText)
            }
        }
    }

    private func detailRow(_ row: CodexResetCreditRowPresentation) -> some View {
        VStack(alignment: .leading, spacing: 11) {
            HStack(spacing: 11) {
                Image(systemName: "arrow.counterclockwise")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(accentStart)
                    .frame(width: 34, height: 34)
                    .background(
                        accentStart.opacity(0.10),
                        in: RoundedRectangle(
                            cornerRadius: AIMeterVisualTheme.capsuleInsetRadius,
                            style: .continuous
                        )
                    )

                VStack(alignment: .leading, spacing: 3) {
                    Text(ProviderDetailText.creditTitle(row.title, localizer: localizer))
                        .aiMeterFont(.caption, weight: .semibold)
                        .lineLimit(1)
                    Text(expirationDateText(row.expiresAt))
                        .aiMeterFont(.subheadline, weight: .semibold)
                        .lineLimit(1)
                        .minimumScaleFactor(0.75)
                }
                Spacer(minLength: 0)
            }

            HStack {
                Text(localizer.text("Expiration"))
                    .foregroundStyle(AIMeterVisualTheme.tertiaryText)
                Spacer()
                Text(ProviderDetailText.creditStatus(row.expirationState, localizer: localizer))
                    .fontWeight(.semibold)
                    .foregroundStyle(statusColor(row.expirationState))
            }
            .aiMeterFont(.caption2)
        }
        .padding(13)
        .background(
            RoundedRectangle(
                cornerRadius: AIMeterVisualTheme.cardCornerRadius,
                style: .continuous
            )
            .fill(
                LinearGradient(
                    colors: [
                        accentStart.opacity(0.11),
                        accentEnd.opacity(0.07),
                        AIMeterVisualTheme.cardSurface.opacity(0.72),
                    ],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                )
            )
        )
        .accessibilityElement(children: .combine)
        .accessibilityLabel(accessibilityLabel(row))
    }

    private var compactContent: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Label {
                    Text(localizer.text("Reset credits"))
                } icon: {
                    Image(systemName: "arrow.counterclockwise.circle")
                        .aiMeterSymbolFont(.caption2)
                }
                    .fontWeight(.semibold)
                Spacer()
                Text(localizer.text("%lld available", Int64(summary.availableCount)))
                    .fontWeight(.semibold)
            }

            ForEach(Array(presentation.rows.enumerated()), id: \.offset) { _, row in
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    Text(ProviderDetailText.creditTitle(row.title, localizer: localizer))
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
                Text(localizer.text("Some expiration details are unavailable"))
                    .foregroundStyle(.tertiary)
            }
        }
        .aiMeterFont(.caption2)
        .foregroundStyle(.secondary)
    }

    private func expirationDateText(_ expiresAt: Date?) -> String {
        guard let expiresAt else { return localizer.text("Date unavailable") }
        return localizer.date(expiresAt)
    }

    private func statusColor(_ state: CodexResetCreditExpirationState) -> Color {
        switch state {
        case .remaining: accentStart
        case .today: .orange
        case .expired: .red
        case .unavailable: AIMeterVisualTheme.tertiaryText
        }
    }

    private func accessibilityLabel(_ row: CodexResetCreditRowPresentation) -> String {
        localizer.text("%@, %@, %@", ProviderDetailText.creditTitle(row.title, localizer: localizer),
                       expirationDateText(row.expiresAt), ProviderDetailText.creditStatus(row.expirationState, localizer: localizer))
    }
}
