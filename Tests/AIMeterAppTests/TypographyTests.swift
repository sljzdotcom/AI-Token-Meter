import AIMeterCore
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

    @Test("Semantic roles preserve the existing visual hierarchy")
    func semanticSizes() {
        #expect(AIMeterTextStyle.largeTitle.pointSize > AIMeterTextStyle.title2.pointSize)
        #expect(AIMeterTextStyle.title2.pointSize > AIMeterTextStyle.headline.pointSize)
        #expect(AIMeterTextStyle.headline.pointSize > AIMeterTextStyle.caption.pointSize)
        #expect(AIMeterTextStyle.caption.pointSize > AIMeterTextStyle.caption2.pointSize)
    }
}
