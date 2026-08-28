import AIMeterCore
import SwiftUI

struct ProviderCard: View {
    let snapshot: UsageSnapshot

    private var presentation: ProviderPresentation {
        ProviderPresentation(snapshot: snapshot)
    }

    var body: some View {
        HStack(spacing: 14) {
            UsageRing(presentation: presentation, size: 54)
                .padding(5)
                .background(Color.black.opacity(0.88), in: Circle())

            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(presentation.title)
                        .font(.headline)
                    Spacer()
                    Text(presentation.valueText)
                        .font(.system(.title3, design: .rounded, weight: .semibold))
                        .foregroundStyle(presentation.semantic.color)
                }
                Text(presentation.detailText)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                if let secondary = snapshot.secondaryMetric {
                    metricLine(secondary)
                }
                if let status = presentation.statusText {
                    Text(status)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .padding(12)
        .background(.quaternary.opacity(0.45), in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }

    private func metricLine(_ metric: UsageMetric) -> some View {
        HStack(spacing: 4) {
            Text(metric.label)
            Spacer()
            if let fraction = metric.usedFraction {
                Text("\(Int((fraction * 100).rounded()))%")
            }
        }
        .font(.caption2)
        .foregroundStyle(.secondary)
    }
}
