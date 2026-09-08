import AIMeterCore
import SwiftUI

struct GeminiDetailView: View {
    static let documentationURL = URL(string: "https://geminicli.com/docs/resources/quota-and-pricing/")!
    let snapshot: UsageSnapshot
    let onRetry: () -> Void

    var body: some View {
        ScrollView {
        VStack(alignment: .leading, spacing: 18) {
            HStack {
                ProviderLogo(provider: .gemini)
                Text("Gemini CLI").aiMeterFont(.title2, weight: .semibold)
            }
            if let metrics = snapshot.geminiQuotaMetrics, !metrics.isEmpty {
                Text(snapshot.collectionStatus == .cached ? "Last available quota" : "Official quota").aiMeterFont(.headline)
                ForEach(metrics) { metric in
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Text(metric.label)
                            Spacer()
                            Text("\(Int(metric.current))% used").monospacedDigit()
                        }
                        ProgressView(value: metric.current, total: 100)
                        if let reset = metric.resetDescription { Text(reset).aiMeterFont(.caption).foregroundStyle(.secondary) }
                    }
                }
                Text("Source: Gemini CLI \(snapshot.sourceVersion ?? "") · model tiers")
                    .aiMeterFont(.caption).foregroundStyle(.secondary)
                Text("Updated \(snapshot.fetchedAt.formatted(date: .abbreviated, time: .shortened))")
                    .aiMeterFont(.caption).foregroundStyle(.secondary)
            } else {
                Text("Quota unavailable").aiMeterFont(.headline)
            }
            if let message = snapshot.statusMessage { Text(message).foregroundStyle(.secondary) }
            HStack {
                Link("Official quota documentation", destination: Self.documentationURL)
                Spacer()
                Button("Retry", action: onRetry)
            }
        }
        .padding(24)
        }
    }
}
