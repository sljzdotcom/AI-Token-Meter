import AIMeterCore
import SwiftUI

enum GeminiDetailPresentation {
    static func remainingText(for metric: UsageMetric, localizer: AppLocalizer = AppLocalizer(language: .english)) -> String {
        localizer.text("%lld%% remaining", Int64(100 - metric.current))
    }
}

struct GeminiDetailView: View {
    @Environment(\.locale) private var locale
    private var localizer: AppLocalizer { AppLocalizer(locale: locale) }
    let snapshot: UsageSnapshot
    let onRetry: () -> Void

    private var accentStyle: AnyShapeStyle {
        AnyShapeStyle(UsageProvider.gemini.accentPalette.gradient)
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                HStack {
                    ProviderLogo(provider: .gemini)
                    Text("Google Antigravity")
                        .aiMeterFont(.title2, weight: .semibold)
                        .foregroundStyle(accentStyle)
                }
                if let metrics = snapshot.geminiQuotaMetrics, !metrics.isEmpty {
                    Text(localizer.text(snapshot.collectionStatus == .cached ? "Last available quota" : "Official quota")).aiMeterFont(.headline)
                    ForEach(metrics) { metric in
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Text(ProviderDetailText.metricLabel(metric.label, localizer: localizer))
                                Spacer()
                                Text(GeminiDetailPresentation.remainingText(for: metric, localizer: localizer))
                                    .monospacedDigit()
                                    .foregroundStyle(accentStyle)
                            }
                            AIMeterProgressBar(
                                provider: .gemini,
                                fraction: metric.usedFraction ?? 0,
                                semantic: .normal
                            )
                            if let reset = ProviderDetailText.reset(metric, localizer: localizer, prefersTimestamp: true) {
                                Text(reset).aiMeterFont(.caption).foregroundStyle(.secondary)
                            }
                        }
                    }
                    Text(localizer.text("Source: Antigravity CLI %@ · /usage", snapshot.sourceVersion ?? ""))
                        .aiMeterFont(.caption).foregroundStyle(.secondary)
                    Text(localizer.text("Updated %@", localizer.date(snapshot.fetchedAt)))
                        .aiMeterFont(.caption).foregroundStyle(.secondary)
                } else {
                    Text(localizer.text("Quota unavailable")).aiMeterFont(.headline)
                }
                if let message = snapshot.statusMessage { Text(ProviderDetailText.diagnostic(message, localizer: localizer)).foregroundStyle(.secondary) }
                GeminiInstallationHelp(state: ServiceAccountStatus.fromGeminiSnapshot(snapshot).connectionState)
                HStack {
                    Link(localizer.text("Antigravity CLI installation guide"), destination: GeminiInstallationGuide.url)
                    Spacer()
                    Button(localizer.text("Retry"), action: onRetry)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .aiMeterDetailSurface()
    }
}
