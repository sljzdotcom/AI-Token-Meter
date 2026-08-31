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

struct DisplayFontOption: Equatable, Identifiable {
    let choice: DisplayFontChoice
    let isEnabled: Bool
    let statusText: String?

    var id: DisplayFontChoice { choice }
}

enum DisplayFontSettingsPresentation {
    static func options(catalog: DisplayFontCatalog) -> [DisplayFontOption] {
        DisplayFontChoice.allCases.map { choice in
            let enabled = catalog.isAvailable(choice)
            return DisplayFontOption(
                choice: choice,
                isEnabled: enabled,
                statusText: enabled ? nil : "Not installed"
            )
        }
    }

    @MainActor static func liveOptions() -> [DisplayFontOption] {
        options(catalog: .live)
    }

    static func canRestore(_ choice: DisplayFontChoice) -> Bool {
        choice != .system
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
        case .largeTitle: 26
        case .title: 22
        case .title2: 17
        case .title3: 15
        case .headline: 13
        case .subheadline: 11
        case .body: 13
        case .caption, .caption2: 10
        }
    }

    var customDefaultWeight: Font.Weight? {
        switch self {
        case .headline: .semibold
        default: nil
        }
    }
}

enum AIMeterResolvedFontDescriptor {
    case systemSemantic(
        style: AIMeterTextStyle,
        design: Font.Design,
        weight: Font.Weight?
    )
    case systemFixed(size: CGFloat, design: Font.Design, weight: Font.Weight?)
    case custom(
        family: String,
        size: CGFloat,
        relativeTo: AIMeterTextStyle,
        weight: Font.Weight?
    )
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

    static func resolvedDescriptor(
        token: AIMeterFontToken,
        choice: DisplayFontChoice,
        catalog: DisplayFontCatalog,
        design: Font.Design,
        weight: Font.Weight?
    ) -> AIMeterResolvedFontDescriptor {
        let style = token.relativeStyle

        if let family = resolvedFamily(for: choice, catalog: catalog) {
            return .custom(
                family: family,
                size: token.pointSize,
                relativeTo: style,
                weight: weight ?? style.customDefaultWeight
            )
        }

        return switch token {
        case .semantic:
            .systemSemantic(style: style, design: design, weight: weight)
        case .fixed(let size, _):
            .systemFixed(size: size, design: design, weight: weight)
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
        let descriptor = AIMeterTypography.resolvedDescriptor(
            token: token,
            choice: choice,
            catalog: .live,
            design: design,
            weight: weight
        )

        switch descriptor {
        case .systemSemantic(let style, let design, let weight):
            content.font(Font.system(style.swiftUIStyle, design: design, weight: weight))
        case .systemFixed(let size, let design, let weight):
            content.font(Font.system(size: size, weight: weight, design: design))
        case .custom(let family, let size, let style, let weight):
            content
                .font(Font.custom(family, size: size, relativeTo: style.swiftUIStyle))
                .fontWeight(weight)
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

private struct AIMeterFontPreviewModifier: ViewModifier {
    let choice: DisplayFontChoice

    func body(content: Content) -> some View {
        let catalog = DisplayFontCatalog.live

        if let family = AIMeterTypography.resolvedFamily(for: choice, catalog: catalog) {
            content.font(Font.custom(
                family,
                size: AIMeterTextStyle.body.pointSize,
                relativeTo: .body
            ))
        } else {
            content.font(.system(.body))
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

    func aiMeterFontPreview(_ choice: DisplayFontChoice) -> some View {
        modifier(AIMeterFontPreviewModifier(choice: choice))
    }
}
