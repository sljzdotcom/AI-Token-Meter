import AIMeterCore
import SwiftUI

struct ServiceAccountStatusView: View {
    @Environment(\.locale) private var locale
    private var localizer: AppLocalizer { AppLocalizer(locale: locale) }
    let status: ServiceAccountStatus

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: symbolName)
                .foregroundStyle(symbolColor)
                .frame(width: 18)

            VStack(alignment: .leading, spacing: 2) {
                Text(primaryText)
                    .fontWeight(.medium)
                if let detail = status.accountDetail, !detail.isEmpty {
                    Text(ProviderDetailText.diagnostic(detail, localizer: localizer))
                        .foregroundStyle(.secondary)
                }
            }
            .aiMeterFont(.caption)

            Spacer(minLength: 8)

            if status.connectionState == .checking {
                ProgressView()
                    .controlSize(.small)
            }
        }
        .accessibilityElement(children: .combine)
    }

    private var primaryText: String {
        ProviderDetailText.accountText(status, localizer: localizer)
    }

    private var symbolName: String {
        switch status.connectionState {
        case .connected: "checkmark.circle.fill"
        case .signInRequired: "person.crop.circle.badge.exclamationmark"
        case .notInstalled: "terminal.fill"
        case .checking: "arrow.triangle.2.circlepath"
        case .unavailable: "exclamationmark.triangle.fill"
        }
    }

    private var symbolColor: Color {
        switch status.connectionState {
        case .connected: .green
        case .checking: .secondary
        case .signInRequired, .notInstalled, .unavailable: .orange
        }
    }
}
