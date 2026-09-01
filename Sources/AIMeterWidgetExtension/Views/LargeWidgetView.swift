import AIMeterCore
import SwiftUI

struct LargeWidgetView: View {
    let envelope: WidgetSnapshotEnvelope

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("AI Token Meter")
                    .font(.system(size: 15, weight: .bold))
                Spacer()
                Text("Updated \(envelope.generatedAt, style: .relative)")
                    .font(.system(size: 9, weight: .medium))
                    .foregroundStyle(.white.opacity(0.5))
            }
            HStack(spacing: 12) {
                VStack(spacing: 9) {
                    ForEach(envelope.providers, id: \.provider) { snapshot in
                        LargeProviderRow(snapshot: snapshot)
                    }
                }
                VStack(spacing: 9) {
                    NextResetCard(summary: envelope.nextReset)
                    ResetCreditsCard(summary: envelope.codexResetCredits)
                }
                .frame(width: 125)
            }
        }
        .foregroundStyle(.white)
        .padding(16)
    }
}

private struct LargeProviderRow: View {
    let snapshot: WidgetProviderSnapshot

    var body: some View {
        HStack(spacing: 10) {
            ZStack {
                WidgetProgressRing(snapshot: snapshot, lineWidth: 4)
                WidgetProviderLogo(provider: snapshot.provider)
                    .frame(width: 18, height: 18)
            }
            .frame(width: 42, height: 42)
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(snapshot.provider.name)
                        .font(.system(size: 12, weight: .semibold))
                    WidgetStatusIndicator(semantic: snapshot.semantic)
                    Spacer()
                    Text(snapshot.valueText)
                        .font(.system(size: 17, weight: .bold, design: .rounded))
                }
                Text(snapshot.detailText)
                    .font(.system(size: 9, weight: .medium))
                    .foregroundStyle(.white.opacity(0.56))
                    .lineLimit(1)
                WidgetProgressBar(snapshot: snapshot)
            }
        }
        .padding(9)
        .frame(maxWidth: .infinity)
        .background(Color.black.opacity(0.34), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
        .accessibilityElement(children: .combine)
    }
}

private struct NextResetCard: View {
    let summary: WidgetResetSummary?

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            Label("Next reset", systemImage: "clock.arrow.circlepath")
                .font(.system(size: 10, weight: .semibold))
            Spacer(minLength: 0)
            Text(summary?.provider.name ?? "No reset")
                .font(.system(size: 15, weight: .bold))
            Text(summary?.text ?? "Open the app to refresh")
                .font(.system(size: 9, weight: .medium))
                .foregroundStyle(.white.opacity(0.58))
                .lineLimit(2)
        }
        .padding(10)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
        .background(Color.black.opacity(0.34), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
    }
}

private struct ResetCreditsCard: View {
    let summary: WidgetResetCreditsSummary?

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            Label("Reset credits", systemImage: "arrow.counterclockwise.circle")
                .font(.system(size: 10, weight: .semibold))
            Spacer(minLength: 0)
            Text("\(summary?.availableCount ?? 0) available")
                .font(.system(size: 15, weight: .bold))
            if let expiration = summary?.nearestExpiration {
                Text("Nearest expires \(expiration, style: .date)")
                    .font(.system(size: 9, weight: .medium))
                    .foregroundStyle(.white.opacity(0.58))
                    .lineLimit(2)
            } else {
                Text("No expiry available")
                    .font(.system(size: 9, weight: .medium))
                    .foregroundStyle(.white.opacity(0.58))
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
        .background(Color.black.opacity(0.34), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
    }
}
