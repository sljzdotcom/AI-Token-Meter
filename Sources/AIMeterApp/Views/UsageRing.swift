import AIMeterCore
import SwiftUI

struct UsageRing: View {
    let presentation: ProviderPresentation
    var size: CGFloat = 58

    var body: some View {
        ZStack {
            Circle()
                .stroke(Color.white.opacity(0.14), lineWidth: 5)
            if let ringFraction = presentation.ringFraction {
                Circle()
                    .trim(from: 0, to: ringFraction)
                    .stroke(
                        presentation.semantic.color,
                        style: StrokeStyle(lineWidth: 5, lineCap: .round)
                    )
                    .rotationEffect(.degrees(-90))
            }
            ProviderLogo(provider: presentation.provider, size: size * 0.43)
        }
        .frame(width: size, height: size)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(presentation.title), \(presentation.valueText), \(presentation.detailText)")
    }
}
