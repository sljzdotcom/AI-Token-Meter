import AIMeterCore
import SwiftUI

struct UsageRing: View {
    let presentation: ProviderPresentation
    var size: CGFloat = 60

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
        }
        .frame(width: size, height: size)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(presentation.title), \(presentation.valueText), \(presentation.detailText)")
    }

    private var ringStyle: AnyShapeStyle {
        if presentation.semantic == .normal {
            return AnyShapeStyle(AIMeterVisualTheme.accentGradient)
        }
        return AnyShapeStyle(presentation.semantic.color)
    }
}
