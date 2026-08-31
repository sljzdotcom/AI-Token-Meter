import AIMeterCore
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
    static let capsuleInsetRadius: CGFloat = 8
    static let panelPadding: CGFloat = 20

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

    static let detailGlass = LinearGradient(
        colors: [
            glassElevated.opacity(0.98),
            glassBase.opacity(0.99),
        ],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
}

extension UsageSemantic {
    var statusSymbolName: String? {
        switch self {
        case .normal: nil
        case .warning: "exclamationmark"
        case .critical: "exclamationmark.triangle.fill"
        case .stale: "clock.arrow.circlepath"
        case .unavailable: "questionmark.circle.fill"
        }
    }
}

private struct AIMeterGlassCardModifier: ViewModifier {
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Environment(\.accessibilityDifferentiateWithoutColor) private var differentiateWithoutColor
    @Environment(\.colorSchemeContrast) private var colorSchemeContrast

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
                                Color.white.opacity(
                                    differentiateWithoutColor || colorSchemeContrast == .increased
                                        ? 0.22 : 0.055
                                ),
                                lineWidth: 1
                            )
                    }
            }
    }
}

private struct AIMeterDetailSurfaceModifier: ViewModifier {
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Environment(\.accessibilityDifferentiateWithoutColor) private var differentiateWithoutColor
    @Environment(\.colorSchemeContrast) private var colorSchemeContrast

    func body(content: Content) -> some View {
        content
            .foregroundStyle(AIMeterVisualTheme.primaryText)
            .padding(AIMeterVisualTheme.panelPadding)
            .background {
                RoundedRectangle(
                    cornerRadius: AIMeterVisualTheme.panelCornerRadius,
                    style: .continuous
                )
                .fill(
                    reduceTransparency
                        ? AnyShapeStyle(AIMeterVisualTheme.glassBase)
                        : AnyShapeStyle(AIMeterVisualTheme.detailGlass)
                )
                .overlay {
                    if differentiateWithoutColor || colorSchemeContrast == .increased {
                        RoundedRectangle(
                            cornerRadius: AIMeterVisualTheme.panelCornerRadius,
                            style: .continuous
                        )
                        .stroke(Color.white.opacity(0.22), lineWidth: 1)
                    }
                }
                .shadow(color: .black.opacity(0.30), radius: 20, x: 0, y: 9)
            }
    }
}

extension View {
    func aiMeterGlassCard(
        cornerRadius: CGFloat = AIMeterVisualTheme.cardCornerRadius
    ) -> some View {
        modifier(AIMeterGlassCardModifier(cornerRadius: cornerRadius))
    }

    func aiMeterDetailSurface() -> some View {
        modifier(AIMeterDetailSurfaceModifier())
    }
}

struct AIMeterProgressBar: View {
    let fraction: Double
    let semantic: UsageSemantic

    var body: some View {
        GeometryReader { proxy in
            ZStack(alignment: .leading) {
                Capsule()
                    .fill(Color.white.opacity(0.10))
                Capsule()
                    .fill(fillStyle)
                    .frame(width: proxy.size.width * min(max(fraction, 0), 1))
            }
        }
        .frame(height: 5)
        .accessibilityHidden(true)
    }

    private var fillStyle: AnyShapeStyle {
        if semantic == .normal {
            return AnyShapeStyle(AIMeterVisualTheme.accentGradient)
        }
        return AnyShapeStyle(semantic.color)
    }
}
