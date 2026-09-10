import AIMeterCore
import SwiftUI

enum GeminiDetailPresentation {
    static func remainingText(for metric: UsageMetric) -> String {
        "\(Int(100 - metric.current))% remaining"
    }
}

struct GeminiDetailView: View {
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
                    Text(snapshot.collectionStatus == .cached ? "Last available quota" : "Official quota").aiMeterFont(.headline)
                    ForEach(metrics) { metric in
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Text(metric.label)
                                Spacer()
                                Text(GeminiDetailPresentation.remainingText(for: metric))
                                    .monospacedDigit()
                                    .foregroundStyle(accentStyle)
                            }
                            AIMeterProgressBar(
                                provider: .gemini,
                                fraction: metric.usedFraction ?? 0,
                                semantic: .normal
                            )
                            if let resetAt = metric.resetAt {
                                Text("Resets \(resetAt.formatted(date: .abbreviated, time: .shortened))")
                                    .aiMeterFont(.caption)
                                    .foregroundStyle(.secondary)
                            } else if let reset = metric.resetDescription {
                                Text(reset).aiMeterFont(.caption).foregroundStyle(.secondary)
                            }
                        }
                    }
                    Text("Source: Antigravity CLI \(snapshot.sourceVersion ?? "") · /usage")
                        .aiMeterFont(.caption).foregroundStyle(.secondary)
                    Text("Updated \(snapshot.fetchedAt.formatted(date: .abbreviated, time: .shortened))")
                        .aiMeterFont(.caption).foregroundStyle(.secondary)
                } else {
                    Text("Quota unavailable").aiMeterFont(.headline)
                }
                if let message = snapshot.statusMessage { Text(message).foregroundStyle(.secondary) }
                GeminiInstallationHelp(state: ServiceAccountStatus.fromGeminiSnapshot(snapshot).connectionState)
                HStack {
                    Link("Antigravity CLI installation guide", destination: GeminiInstallationGuide.url)
                    Spacer()
                    Button("Retry", action: onRetry)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .aiMeterDetailSurface()
    }
}
