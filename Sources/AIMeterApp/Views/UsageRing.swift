import AIMeterCore
import SwiftUI

struct UsageRing: View {
    let presentation: ProviderPresentation
    var size: CGFloat = 58

    var body: some View {
        ZStack {
            Circle()
                .stroke(Color.white.opacity(0.14), lineWidth: 5)
            Circle()
                .trim(from: 0, to: max(presentation.fraction ?? 0.04, 0.04))
                .stroke(
                    presentation.semantic.color,
                    style: StrokeStyle(lineWidth: 5, lineCap: .round)
                )
                .rotationEffect(.degrees(-90))
            VStack(spacing: 1) {
                Image(systemName: presentation.provider.symbolName)
                    .font(.system(size: 11, weight: .semibold))
                Text(presentation.valueText)
                    .font(.system(size: 10, weight: .semibold, design: .rounded))
                    .minimumScaleFactor(0.65)
                    .lineLimit(1)
            }
            .foregroundStyle(.white)
            .padding(7)
        }
        .frame(width: size, height: size)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(presentation.title), \(presentation.valueText), \(presentation.detailText)")
    }
}
