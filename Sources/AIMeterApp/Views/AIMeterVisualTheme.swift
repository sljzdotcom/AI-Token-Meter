import SwiftUI

enum AIMeterVisualTheme {
    static let glassBase = Color(red: 0.027, green: 0.039, blue: 0.063)
    static let glassElevated = Color(red: 0.051, green: 0.071, blue: 0.114)
    static let cardSurface = Color(red: 0.090, green: 0.114, blue: 0.161)
    static let mintAccent = Color(red: 0.329, green: 0.929, blue: 0.776)
    static let violetAccent = Color(red: 0.467, green: 0.412, blue: 1.0)
    static let primaryText = Color.white.opacity(0.96)
    static let secondaryText = Color.white.opacity(0.68)
    static let tertiaryText = Color.white.opacity(0.46)

    static let panelCornerRadius: CGFloat = 25
    static let cardCornerRadius: CGFloat = 15

    static let accentGradient = LinearGradient(
        colors: [mintAccent, violetAccent],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )

    static let floatingGlass = LinearGradient(
        colors: [
            Color(red: 0.055, green: 0.069, blue: 0.110),
            glassBase,
            Color(red: 0.034, green: 0.043, blue: 0.075),
        ],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
}

private struct AIMeterGlassCardModifier: ViewModifier {
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Environment(\.accessibilityDifferentiateWithoutColor) private var differentiateWithoutColor

    let cornerRadius: CGFloat

    func body(content: Content) -> some View {
        content
            .background {
                RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                    .fill(
                        reduceTransparency
                            ? AIMeterVisualTheme.cardSurface
                            : AIMeterVisualTheme.cardSurface.opacity(0.78)
                    )
                    .overlay {
                        RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                            .stroke(
                                Color.white.opacity(differentiateWithoutColor ? 0.18 : 0.055),
                                lineWidth: 1
                            )
                    }
            }
    }
}

extension View {
    func aiMeterGlassCard(
        cornerRadius: CGFloat = AIMeterVisualTheme.cardCornerRadius
    ) -> some View {
        modifier(AIMeterGlassCardModifier(cornerRadius: cornerRadius))
    }
}
