import AIMeterCore
import SwiftUI

struct UsageRing: View {
    let presentation: ProviderPresentation
    var size: CGFloat = 60
    @Environment(\.accessibilityDifferentiateWithoutColor) private var differentiateWithoutColor

    var body: some View {
        ZStack {
            Circle()
                .fill(AIMeterVisualTheme.glassBase.opacity(0.62))
                .overlay {
                    Circle()
                        .stroke(Color.white.opacity(0.13), lineWidth: 5)
                }
            if let ringFraction = presentation.ringFraction {
                Circle()
                    .trim(from: 0, to: ringFraction)
                    .stroke(
                        ringStyle,
                        style: StrokeStyle(lineWidth: 5, lineCap: .round)
                    )
                    .rotationEffect(.degrees(-90))
            }
            ProviderLogo(provider: presentation.provider, size: size * 0.44)
            if differentiateWithoutColor,
               let symbolName = presentation.semantic.statusSymbolName {
                Image(systemName: symbolName)
                    .font(.system(size: size * 0.15, weight: .bold))
                    .foregroundStyle(.white)
                    .padding(size * 0.055)
                    .background(Circle().fill(Color.black.opacity(0.82)))
                    .offset(x: size * 0.30, y: -size * 0.30)
                    .accessibilityHidden(true)
            }
        }
        .frame(width: size, height: size)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(accessibilityLabel)
    }

    private var accessibilityLabel: String {
        [
            presentation.title,
            presentation.valueText,
            presentation.detailText,
            presentation.accessibilityStatusText,
        ]
        .compactMap { $0 }
        .joined(separator: ", ")
    }

    private var ringStyle: AnyShapeStyle {
        if presentation.semantic == .normal {
            return AnyShapeStyle(AIMeterVisualTheme.accentGradient)
        }
        return AnyShapeStyle(presentation.semantic.color)
    }
}
