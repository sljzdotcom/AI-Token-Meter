import AIMeterCore
import SwiftUI

struct FloatingStripView: View {
    @Bindable var model: AppModel
    let onSelectionChange: (UsageProvider?) -> Void

    @State private var selectedProvider: UsageProvider?

    var body: some View {
        VStack(spacing: 14) {
            ForEach(presentations, id: \.provider) { presentation in
                Button {
                    selectedProvider = selectedProvider == presentation.provider
                        ? nil
                        : presentation.provider
                    onSelectionChange(selectedProvider)
                } label: {
                    UsageRing(presentation: presentation, size: 58)
                        .scaleEffect(selectedProvider == presentation.provider ? 1.06 : 1)
                }
                .buttonStyle(.plain)
                .animation(.spring(response: 0.28, dampingFraction: 0.8), value: selectedProvider)
            }
        }
        .padding(.vertical, 18)
        .padding(.horizontal, 9)
        .background(
            RoundedRectangle(cornerRadius: 34, style: .continuous)
                .fill(Color.black.opacity(0.94))
                .shadow(color: .black.opacity(0.28), radius: 18, x: -5, y: 7)
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var presentations: [ProviderPresentation] {
        UsageProvider.allCases.map { provider in
            if let snapshot = model.snapshots.first(where: { $0.provider == provider }) {
                return ProviderPresentation(snapshot: snapshot)
            }
            return ProviderPresentation(snapshot: UsageSnapshot(
                provider: provider,
                availability: .unknown,
                collectionStatus: .refreshing
            ))
        }
    }
}

struct FloatingDetailView: View {
    @Bindable var model: AppModel
    let provider: UsageProvider

    var body: some View {
        if let snapshot = model.snapshots.first(where: { $0.provider == provider }) {
            let presentation = ProviderPresentation(snapshot: snapshot)
            VStack(alignment: .leading, spacing: 10) {
                HStack {
                    Label(presentation.title, systemImage: snapshot.provider.symbolName)
                        .font(.headline)
                    Spacer()
                    Text(presentation.valueText)
                        .font(.system(.title2, design: .rounded, weight: .bold))
                        .foregroundStyle(presentation.semantic.color)
                }
                Text(presentation.detailText)
                    .font(.caption)
                    .foregroundStyle(.white.opacity(0.7))
                if let secondary = snapshot.secondaryMetric,
                   let fraction = secondary.usedFraction {
                    VStack(alignment: .leading, spacing: 5) {
                        HStack {
                            Text(secondary.label)
                            Spacer()
                            Text("\(Int((fraction * 100).rounded()))%")
                        }
                        ProgressView(value: fraction)
                            .tint(presentation.semantic.color)
                    }
                    .font(.caption2)
                }
                if let status = presentation.statusText {
                    Text(status)
                        .font(.caption2)
                        .foregroundStyle(.white.opacity(0.6))
                }
            }
            .foregroundStyle(.white)
            .padding(16)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .background(
                RoundedRectangle(cornerRadius: 22, style: .continuous)
                    .fill(Color.black.opacity(0.94))
                    .shadow(color: .black.opacity(0.24), radius: 18, x: -4, y: 8)
            )
            .padding(5)
        }
    }
}
