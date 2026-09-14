import AIMeterCore
import SwiftUI

enum GeminiDetailPresentation {
    static func remainingText(for metric: UsageMetric, localizer: AppLocalizer = AppLocalizer(language: .english)) -> String {
        localizer.text("%lld%% remaining", Int64(100 - metric.current))
    }

    static func availableModelsText(
        for info: AntigravityCLIInfo,
        localizer: AppLocalizer = AppLocalizer(language: .english)
    ) -> String? {
        guard let count = info.availableModelCount else { return nil }
        return localizer.text(
            "%lld models · %lld families",
            Int64(count),
            Int64(info.modelFamilies.count)
        )
    }

    static func familyText(for info: AntigravityCLIInfo) -> String? {
        info.modelFamilies.isEmpty ? nil : info.modelFamilies.joined(separator: " · ")
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
                    Text(localizer.text(snapshot.collectionStatus == .cached ? "Last available Gemini quota" : "Gemini quota"))
                        .aiMeterFont(.headline)
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
                } else {
                    Text(localizer.text("Quota unavailable")).aiMeterFont(.headline)
                }
                if snapshot.antigravityCLIInfo != nil || snapshot.sourceVersion != nil {
                    VStack(alignment: .leading, spacing: 10) {
                        Text(localizer.text("Antigravity CLI")).aiMeterFont(.headline)
                        VStack(alignment: .leading, spacing: 9) {
                            if let currentModel = snapshot.antigravityCLIInfo?.currentModel {
                                CLIInfoRow(label: localizer.text("Current model"), value: currentModel)
                            }
                            if let info = snapshot.antigravityCLIInfo,
                               let available = GeminiDetailPresentation.availableModelsText(for: info, localizer: localizer) {
                                CLIInfoRow(label: localizer.text("Available models"), value: available)
                                if let families = GeminiDetailPresentation.familyText(for: info) {
                                    Text(families)
                                        .aiMeterFont(.caption)
                                        .foregroundStyle(.secondary)
                                        .fixedSize(horizontal: false, vertical: true)
                                }
                            }
                            if let version = snapshot.sourceVersion {
                                CLIInfoRow(label: localizer.text("CLI version"), value: version)
                            }
                        }
                        .padding(12)
                        .background(.white.opacity(0.045), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                        .overlay {
                            RoundedRectangle(cornerRadius: 14, style: .continuous)
                                .stroke(.white.opacity(0.08), lineWidth: 1)
                        }
                    }
                }
                Text(localizer.text("Updated %@", localizer.date(snapshot.fetchedAt)))
                    .aiMeterFont(.caption).foregroundStyle(.secondary)
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

private struct CLIInfoRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            Text(label).aiMeterFont(.caption).foregroundStyle(.secondary)
            Spacer(minLength: 8)
            Text(value)
                .aiMeterFont(.caption, weight: .medium)
                .multilineTextAlignment(.trailing)
                .textSelection(.enabled)
        }
    }
}
