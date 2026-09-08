import AIMeterCore
import SwiftUI

struct GeminiDetailView: View {
    static let documentationURL = URL(string: "https://geminicli.com/docs/resources/quota-and-pricing/")!
    let snapshot: UsageSnapshot
    let onRetry: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack {
                ProviderLogo(provider: .gemini)
                Text("Gemini CLI").aiMeterFont(.title2, weight: .semibold)
            }
            Text("Quota unavailable").aiMeterFont(.headline)
            Text(snapshot.statusMessage ?? "Automatic quota collection is not available.")
                .foregroundStyle(.secondary)
            HStack {
                Link("Official quota documentation", destination: Self.documentationURL)
                Spacer()
                Button("Retry", action: onRetry)
            }
        }
        .padding(24)
    }
}
