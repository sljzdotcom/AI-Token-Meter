import AIMeterCore
import SwiftUI

struct MediumWidgetView: View {
    let providers: [WidgetProviderSnapshot]

    var body: some View {
        HStack(spacing: 10) {
            ForEach(providers, id: \.provider) { snapshot in
                VStack(alignment: .leading, spacing: 7) {
                    HStack(spacing: 6) {
                        WidgetProviderLogo(provider: snapshot.provider)
                            .frame(width: 18, height: 18)
                        Text(snapshot.provider.displayName)
                            .font(.system(size: 12, weight: .semibold))
                            .lineLimit(1)
                            .minimumScaleFactor(0.72)
                        Spacer(minLength: 0)
                        WidgetStatusIndicator(semantic: snapshot.semantic)
                    }
                    Spacer(minLength: 0)
                    Text(snapshot.valueText)
                        .font(.system(size: 23, weight: .bold, design: .rounded))
                        .minimumScaleFactor(0.65)
                        .lineLimit(1)
                    Text(snapshot.detailText)
                        .font(.system(size: 9, weight: .medium))
                        .foregroundStyle(.white.opacity(0.58))
                        .lineLimit(1)
                    WidgetProgressBar(snapshot: snapshot)
                }
                .padding(10)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                .background(Color.black.opacity(0.34), in: RoundedRectangle(cornerRadius: 15, style: .continuous))
                .accessibilityElement(children: .combine)
            }
        }
        .foregroundStyle(.white)
        .padding(12)
    }
}

struct WidgetProgressBar: View {
    let snapshot: WidgetProviderSnapshot

    var body: some View {
        GeometryReader { proxy in
            ZStack(alignment: .leading) {
                Capsule().fill(Color.white.opacity(0.12))
                if let fraction = snapshot.fraction {
                    Capsule()
                        .fill(LinearGradient(
                            colors: snapshot.semantic.accentColors(for: snapshot.provider),
                            startPoint: .leading,
                            endPoint: .trailing
                        ))
                        .frame(width: max(4, proxy.size.width * fraction))
                }
            }
        }
        .frame(height: 4)
    }
}
