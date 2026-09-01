import AIMeterCore
import AppKit
import SwiftUI

enum DisplayFontFamilyCandidates {
    static func values(for choice: DisplayFontChoice) -> [String] {
        switch choice {
        case .system: []
        case .antonio: ["Antonio"]
        case .dinCondensed: ["DIN Condensed"]
        case .alimamaFangYuanTiVF: ["Alimama FangYuanTi VF"]
        case .firaCode: ["Fira Code", "Fira Code VF"]
        case .leigo: ["Leigo", "Leigo Regular"]
        case .menlo: ["Menlo"]
        case .alimamaDaoLiTi: ["Alimama DaoLiTi"]
        }
    }
}

struct DisplayFontCatalog: Sendable {
    let availableFamilies: Set<String>

    init(availableFamilies: Set<String>) {
        self.availableFamilies = availableFamilies
    }

    @MainActor static var live: Self {
        Self(availableFamilies: Set(NSFontManager.shared.availableFontFamilies))
    }

    func isAvailable(_ choice: DisplayFontChoice) -> Bool {
        choice == .system || resolvedFamily(choice) != nil
    }

    func resolvedFamily(_ choice: DisplayFontChoice) -> String? {
        DisplayFontFamilyCandidates.values(for: choice)
            .first(where: availableFamilies.contains)
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

struct AIMeterFontScopeConfiguration: Equatable, Sendable {
    let choice: DisplayFontChoice
    let pointOffset: CGFloat

    static let settings = Self(choice: .system, pointOffset: 0)

    static func content(_ choice: DisplayFontChoice) -> Self {
        Self(choice: choice, pointOffset: 1)
    }

    static func menuBarLabel(_ choice: DisplayFontChoice) -> Self {
        Self(choice: choice, pointOffset: 0)
    }
}

enum AIMeterTypography {
    static func resolvedFamily(
        for choice: DisplayFontChoice,
        catalog: DisplayFontCatalog
    ) -> String? {
        catalog.resolvedFamily(choice)
    }

    static func resolvedDescriptor(
        token: AIMeterFontToken,
        choice: DisplayFontChoice,
        catalog: DisplayFontCatalog,
        pointOffset: CGFloat = 0,
        design: Font.Design,
        weight: Font.Weight?
    ) -> AIMeterResolvedFontDescriptor {
        let style = token.relativeStyle
        let size = resolvedPointSize(token: token, pointOffset: pointOffset)

        if let family = resolvedFamily(for: choice, catalog: catalog) {
            return .custom(
                family: family,
                size: size,
                relativeTo: style,
                weight: weight ?? style.customDefaultWeight
            )
        }

        return switch token {
        case .semantic where pointOffset == 0:
            .systemSemantic(style: style, design: design, weight: weight)
        case .semantic:
            .systemFixed(size: size, design: design, weight: weight ?? style.customDefaultWeight)
        case .fixed:
            .systemFixed(size: size, design: design, weight: weight)
        }
    }

    static func resolvedPointSize(
        token: AIMeterFontToken,
        pointOffset: CGFloat
    ) -> CGFloat {
        max(1, token.pointSize + pointOffset)
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

private struct AIMeterFontPointOffsetKey: EnvironmentKey {
    static let defaultValue: CGFloat = 0
}

extension EnvironmentValues {
    var aiMeterDisplayFontChoice: DisplayFontChoice {
        get { self[AIMeterDisplayFontChoiceKey.self] }
        set { self[AIMeterDisplayFontChoiceKey.self] = newValue }
    }

    var aiMeterFontPointOffset: CGFloat {
        get { self[AIMeterFontPointOffsetKey.self] }
        set { self[AIMeterFontPointOffsetKey.self] = newValue }
    }
}

private struct AIMeterFontModifier: ViewModifier {
    @Environment(\.aiMeterDisplayFontChoice) private var choice
    @Environment(\.aiMeterFontPointOffset) private var pointOffset
    let token: AIMeterFontToken
    let design: Font.Design
    let weight: Font.Weight?

    func body(content: Content) -> some View {
        let descriptor = AIMeterTypography.resolvedDescriptor(
            token: token,
            choice: choice,
            catalog: .live,
            pointOffset: pointOffset,
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
    let configuration: AIMeterFontScopeConfiguration

    func body(content: Content) -> some View {
        let catalog = DisplayFontCatalog.live
        let descriptor = AIMeterTypography.resolvedDescriptor(
            token: .semantic(.body),
            choice: configuration.choice,
            catalog: catalog,
            pointOffset: configuration.pointOffset,
            design: .default,
            weight: nil
        )
        let scopedContent = content
            .environment(\.aiMeterDisplayFontChoice, configuration.choice)
            .environment(\.aiMeterFontPointOffset, configuration.pointOffset)

        switch descriptor {
        case .systemSemantic(let style, let design, let weight):
            scopedContent.font(Font.system(style.swiftUIStyle, design: design, weight: weight))
        case .systemFixed(let size, let design, let weight):
            scopedContent.font(Font.system(size: size, weight: weight, design: design))
        case .custom(let family, let size, let style, let weight):
            scopedContent
                .font(Font.custom(family, size: size, relativeTo: style.swiftUIStyle))
                .fontWeight(weight)
        }
    }
}

extension View {
    /// Pins an SF Symbol to its declared semantic baseline inside a content font scope.
    func aiMeterSymbolFont(_ style: AIMeterTextStyle) -> some View {
        font(.system(size: style.pointSize))
    }

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

    func aiMeterFontScope(_ configuration: AIMeterFontScopeConfiguration) -> some View {
        modifier(AIMeterFontScopeModifier(configuration: configuration))
    }

}
