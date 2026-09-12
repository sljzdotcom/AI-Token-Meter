import AIMeterCore
import SwiftUI

struct UsageRing: View {
    @Environment(\.locale) private var locale
    private var localizer: AppLocalizer { AppLocalizer(locale: locale) }
    let presentation: ProviderPresentation
    var size: CGFloat = 60
    var operation: ProviderOperationState = .idle
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.accessibilityDifferentiateWithoutColor) private var differentiateWithoutColor

    var body: some View {
        ZStack {
            Circle()
                .fill(AIMeterVisualTheme.glassBase.opacity(0.62))
                .overlay {
                    Circle()
                        .stroke(Color.white.opacity(0.13), lineWidth: size <= 48 ? 4 : 5)
                }
            if let ringFraction = presentation.ringFraction {
                Circle()
                    .trim(from: 0, to: ringFraction)
                    .stroke(
                        ringStyle,
                        style: StrokeStyle(lineWidth: size <= 48 ? 3.5 : 5, lineCap: .round)
                    )
                    .rotationEffect(.degrees(-90))
            }
            ProviderLogo(provider: presentation.provider, size: size * 0.44)
            if operation != .idle {
                TimelineView(.animation(minimumInterval: 1.0 / 30, paused: reduceMotion)) { context in
                    let seconds = context.date.timeIntervalSinceReferenceDate
                    Circle()
                        .trim(from: 0, to: operation == .refreshing ? 0.25 : 1)
                        .stroke(operation == .refreshing ? Color(red: 0.72, green: 0.85, blue: 1) : Color.orange,
                                style: StrokeStyle(lineWidth: 1.5, lineCap: .round))
                        .rotationEffect(.degrees(operation == .refreshing && !reduceMotion ? seconds / 1.1 * 360 : -90))
                        .opacity(operation == .waiting && !reduceMotion ? 0.55 + 0.3 * sin(seconds * 3) : 0.85)
                        .frame(width: size * 0.70, height: size * 0.70)
                }
                .accessibilityHidden(true)
            }
            if operation == .waiting {
                Image(systemName: "exclamationmark.circle.fill")
                    .font(.system(size: 10)).foregroundStyle(.orange)
                    .offset(x: size * 0.30, y: -size * 0.30)
                    .accessibilityHidden(true)
            }
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
        .accessibilityHint(localizer.text(operation == .refreshing ? "Refreshing" : operation == .waiting ? "Action required" : ""))
    }

    private var accessibilityLabel: String {
        ProviderDetailText.ringAccessibility(presentation, localizer: localizer)
    }

    private var ringStyle: AnyShapeStyle {
        presentation.semantic.accentStyle(for: presentation.provider)
    }
}
