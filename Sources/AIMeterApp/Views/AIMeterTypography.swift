import AIMeterCore
import AppKit
import SwiftUI

struct DisplayFontCatalog: Sendable {
    let availableFamilies: Set<String>

    init(availableFamilies: Set<String>) {
        self.availableFamilies = availableFamilies
    }

    @MainActor static var live: Self {
        Self(availableFamilies: Set(NSFontManager.shared.availableFontFamilies))
    }

    func isAvailable(_ choice: DisplayFontChoice) -> Bool {
        switch choice {
        case .system: true
        case .antonio: availableFamilies.contains("Antonio")
        case .dinCondensed: availableFamilies.contains("DIN Condensed")
        }
    }
}

enum AIMeterTextStyle: CaseIterable, Sendable {
    case largeTitle, title, title2, title3, headline, subheadline, body, caption, caption2

    var swiftUIStyle: Font.TextStyle {
        switch self {
        case .largeTitle: .largeTitle
        case .title: .title
        case .title2: .title2
        case .title3: .title3
        case .headline: .headline
        case .subheadline: .subheadline
        case .body: .body
        case .caption: .caption
        case .caption2: .caption2
        }
    }

    var pointSize: CGFloat {
        switch self {
        case .largeTitle: 34
        case .title: 28
        case .title2: 22
        case .title3: 20
        case .headline: 17
        case .subheadline: 15
        case .body: 13
        case .caption: 12
        case .caption2: 11
        }
    }

    var defaultWeight: Font.Weight? {
        switch self {
        case .headline: .semibold
        default: nil
        }
    }
}

enum AIMeterTypography {
    static func resolvedFamily(
        for choice: DisplayFontChoice,
        catalog: DisplayFontCatalog
    ) -> String? {
        guard catalog.isAvailable(choice) else { return nil }
        return switch choice {
        case .system: nil
        case .antonio: "Antonio"
        case .dinCondensed: "DIN Condensed"
        }
    }
}

enum AIMeterFontToken: Sendable {
    case semantic(AIMeterTextStyle)
    case fixed(size: CGFloat, relativeTo: AIMeterTextStyle)

    var pointSize: CGFloat {
        switch self {
        case .semantic(let style): style.pointSize
        case .fixed(let size, _): size
        }
    }

    var relativeStyle: AIMeterTextStyle {
        switch self {
        case .semantic(let style): style
        case .fixed(_, let style): style
        }
    }
}

private struct AIMeterDisplayFontChoiceKey: EnvironmentKey {
    static let defaultValue: DisplayFontChoice = .system
}

extension EnvironmentValues {
    var aiMeterDisplayFontChoice: DisplayFontChoice {
        get { self[AIMeterDisplayFontChoiceKey.self] }
        set { self[AIMeterDisplayFontChoiceKey.self] = newValue }
    }
}

private struct AIMeterFontModifier: ViewModifier {
    @Environment(\.aiMeterDisplayFontChoice) private var choice
    let token: AIMeterFontToken
    let design: Font.Design
    let weight: Font.Weight?

    func body(content: Content) -> some View {
        let style = token.relativeStyle
        let resolvedWeight = weight ?? style.defaultWeight
        let catalog = DisplayFontCatalog.live

        if let family = AIMeterTypography.resolvedFamily(for: choice, catalog: catalog) {
            content.font(Font.custom(family, size: token.pointSize, relativeTo: style.swiftUIStyle))
                .fontWeight(resolvedWeight)
        } else {
            content.font(systemFont(style: style, weight: resolvedWeight))
        }
    }

    private func systemFont(style: AIMeterTextStyle, weight: Font.Weight?) -> Font {
        switch token {
        case .semantic:
            Font.system(style.swiftUIStyle, design: design, weight: weight)
        case .fixed(let size, _):
            Font.system(size: size, weight: weight, design: design)
        }
    }
}

private struct AIMeterFontScopeModifier: ViewModifier {
    let choice: DisplayFontChoice

    func body(content: Content) -> some View {
        let catalog = DisplayFontCatalog.live

        if let family = AIMeterTypography.resolvedFamily(for: choice, catalog: catalog) {
            content
                .environment(\.aiMeterDisplayFontChoice, choice)
                .font(Font.custom(
                    family,
                    size: AIMeterTextStyle.body.pointSize,
                    relativeTo: .body
                ))
        } else {
            content
                .environment(\.aiMeterDisplayFontChoice, choice)
                .font(.system(.body))
        }
    }
}

extension View {
    func aiMeterFont(
        _ style: AIMeterTextStyle,
        design: Font.Design = .default,
        weight: Font.Weight? = nil
    ) -> some View {
        modifier(AIMeterFontModifier(token: .semantic(style), design: design, weight: weight))
    }

    func aiMeterFont(
        size: CGFloat,
        relativeTo style: AIMeterTextStyle,
        design: Font.Design = .default,
        weight: Font.Weight? = nil
    ) -> some View {
        modifier(AIMeterFontModifier(
            token: .fixed(size: size, relativeTo: style),
            design: design,
            weight: weight
        ))
    }

    func aiMeterFontScope(_ choice: DisplayFontChoice) -> some View {
        modifier(AIMeterFontScopeModifier(choice: choice))
    }
}
