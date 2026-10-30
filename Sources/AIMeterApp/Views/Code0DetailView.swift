import AIMeterCore
import SwiftUI

/// Numeric display only. The dedicated website connection is controlled in Services.
struct Code0DetailView: View {
    @Environment(\.locale) private var locale
    let snapshot: UsageSnapshot?
    let statusKey: String
    private var localizer: AppLocalizer { AppLocalizer(locale: locale) }

    var body: some View {
        VStack(alignment: .leading, spacing: 13) {
            HStack(spacing: 10) {
                ProviderLogo(provider: .code0, size: 27)
                VStack(alignment: .leading, spacing: 2) {
                    Text(UsageProvider.code0.displayName).aiMeterFont(.headline)
                    Text(localizer.text("Website usage"))
                        .aiMeterFont(.caption2).foregroundStyle(AIMeterVisualTheme.secondaryText)
                }
                Spacer()
            }
            if let snapshot {
                creditCard("Balance", metric: snapshot.primaryMetric)
                creditCard("Cumulative usage", metric: snapshot.secondaryMetric)
                Text(localizer.text("Last successful sync: %@", localizer.date(snapshot.fetchedAt)))
                    .aiMeterFont(.caption).foregroundStyle(AIMeterVisualTheme.secondaryText)
            } else {
                Text(localizer.text("Connect Code0 in Services to view website usage."))
                    .aiMeterFont(.caption).foregroundStyle(AIMeterVisualTheme.secondaryText)
            }
            Text(localizer.text(statusKey))
                .aiMeterFont(.caption).foregroundStyle(AIMeterVisualTheme.secondaryText)
            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .aiMeterDetailSurface()
    }
    private func creditCard(_ title: String, metric: UsageMetric?) -> some View {
        VStack(alignment: .leading, spacing: 7) {
            Text(localizer.text(title)).aiMeterFont(.caption, weight: .semibold)
                .foregroundStyle(AIMeterVisualTheme.secondaryText)
            if let metric, metric.unit == .siteCredits, metric.current.isFinite, metric.current >= 0 {
                Text(localizer.text("%@ site credits", localizer.decimal(metric.current, fractionDigits: 2)))
                    .aiMeterFont(.title2, design: .rounded, weight: .bold)
            } else { Text("—").aiMeterFont(.title2) }
        }
        .padding(12).frame(maxWidth: .infinity, alignment: .leading).aiMeterGlassCard()
    }
}
