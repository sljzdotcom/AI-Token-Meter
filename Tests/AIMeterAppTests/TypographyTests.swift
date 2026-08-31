import AIMeterCore
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("AI Meter typography")
struct TypographyTests {
    @Test("System is always available while custom choices use registered families")
    func availability() {
        let catalog = DisplayFontCatalog(availableFamilies: ["Antonio"])
        #expect(catalog.isAvailable(.system))
        #expect(catalog.isAvailable(.antonio))
        #expect(!catalog.isAvailable(.dinCondensed))
    }

    @Test("Unavailable saved custom fonts resolve safely to system")
    func fallback() {
        let catalog = DisplayFontCatalog(availableFamilies: [])
        #expect(AIMeterTypography.resolvedFamily(for: .antonio, catalog: catalog) == nil)
        #expect(AIMeterTypography.resolvedFamily(for: .dinCondensed, catalog: catalog) == nil)
    }

    @Test("Available choices resolve to the approved family names")
    func familyNames() {
        let catalog = DisplayFontCatalog(
            availableFamilies: ["Antonio", "DIN Condensed"]
        )
        #expect(AIMeterTypography.resolvedFamily(for: .antonio, catalog: catalog) == "Antonio")
        #expect(AIMeterTypography.resolvedFamily(for: .dinCondensed, catalog: catalog) == "DIN Condensed")
    }

    @Test("Settings always exposes three ordered choices and marks missing fonts")
    func settingsOptions() {
        let catalog = DisplayFontCatalog(availableFamilies: ["Antonio"])
        let options = DisplayFontSettingsPresentation.options(catalog: catalog)

        #expect(options.map(\.choice) == [.system, .antonio, .dinCondensed])
        #expect(options[0].isEnabled)
        #expect(options[1].isEnabled)
        #expect(!options[2].isEnabled)
        #expect(options[2].statusText == "Not installed")
    }

    @Test("Every semantic role preserves native system behavior and macOS custom base size")
    func semanticDescriptors() {
        let catalog = DisplayFontCatalog(
            availableFamilies: ["Antonio", "DIN Condensed"]
        )
        let expectedRoles: [(AIMeterTextStyle, CGFloat, Font.Weight?)] = [
            (.largeTitle, 26, nil),
            (.title, 22, nil),
            (.title2, 17, nil),
            (.title3, 15, nil),
            (.headline, 13, .semibold),
            (.subheadline, 11, nil),
            (.body, 13, nil),
            (.caption, 10, nil),
            (.caption2, 10, nil),
        ]

        for (style, expectedSize, expectedCustomWeight) in expectedRoles {
            let system = AIMeterTypography.resolvedDescriptor(
                token: .semantic(style),
                choice: .system,
                catalog: catalog,
                design: .default,
                weight: nil
            )
            guard case let .systemSemantic(resolvedStyle, _, resolvedWeight) = system else {
                Issue.record("System \(style) did not resolve as a native semantic font")
                continue
            }
            #expect(resolvedStyle == style)
            #expect(resolvedWeight == nil)

            for choice in [DisplayFontChoice.antonio, .dinCondensed] {
                let custom = AIMeterTypography.resolvedDescriptor(
                    token: .semantic(style),
                    choice: choice,
                    catalog: catalog,
                    design: .default,
                    weight: nil
                )
                guard case let .custom(_, size, relativeTo, resolvedWeight) = custom else {
                    Issue.record("\(choice) \(style) did not resolve as a custom font")
                    continue
                }
                #expect(size == expectedSize)
                #expect(relativeTo == style)
                #expect(resolvedWeight == expectedCustomWeight)
            }
        }
    }

    @Test("Explicit caller weight overrides semantic defaults for system and custom fonts")
    func explicitWeight() {
        let catalog = DisplayFontCatalog(availableFamilies: ["Antonio"])

        let system = AIMeterTypography.resolvedDescriptor(
            token: .semantic(.headline),
            choice: .system,
            catalog: catalog,
            design: .rounded,
            weight: .bold
        )
        guard case let .systemSemantic(style, design, weight) = system else {
            Issue.record("System headline did not resolve as a native semantic font")
            return
        }
        #expect(style == .headline)
        #expect(design == .rounded)
        #expect(weight == .bold)

        let custom = AIMeterTypography.resolvedDescriptor(
            token: .semantic(.headline),
            choice: .antonio,
            catalog: catalog,
            design: .rounded,
            weight: .bold
        )
        guard case let .custom(family, size, relativeTo, weight) = custom else {
            Issue.record("Antonio headline did not resolve as a custom font")
            return
        }
        #expect(family == "Antonio")
        #expect(size == 13)
        #expect(relativeTo == .headline)
        #expect(weight == .bold)
    }

    @Test("Unavailable custom fonts preserve native semantic fallback behavior")
    func unavailableDescriptorFallback() {
        let descriptor = AIMeterTypography.resolvedDescriptor(
            token: .semantic(.headline),
            choice: .antonio,
            catalog: DisplayFontCatalog(availableFamilies: []),
            design: .default,
            weight: nil
        )

        guard case let .systemSemantic(style, _, weight) = descriptor else {
            Issue.record("Unavailable Antonio did not resolve to a system semantic font")
            return
        }
        #expect(style == .headline)
        #expect(weight == nil)
    }

    @Test("Fixed fonts preserve size, relative role, and explicit weight")
    func fixedDescriptor() {
        let catalog = DisplayFontCatalog(availableFamilies: ["DIN Condensed"])
        let descriptor = AIMeterTypography.resolvedDescriptor(
            token: .fixed(size: 15, relativeTo: .body),
            choice: .dinCondensed,
            catalog: catalog,
            design: .default,
            weight: .semibold
        )

        guard case let .custom(family, size, relativeTo, weight) = descriptor else {
            Issue.record("Fixed DIN font did not resolve as a custom font")
            return
        }
        #expect(family == "DIN Condensed")
        #expect(size == 15)
        #expect(relativeTo == .body)
        #expect(weight == .semibold)
    }
}
