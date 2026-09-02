import Testing
@testable import AIMeterCore

@Suite("AI Token Meter brand")
struct AppBrandTests {
    @Test("Exposes the approved visible product copy")
    func visibleCopy() {
        #expect(AppBrand.displayName == "AI Token Meter")
        #expect(AppBrand.subtitle == "Private AI usage monitor")
    }

    @Test("Formats the public author credit for About")
    func publicAuthorCredit() {
        #expect(AppBrand.author == "Miller")
        #expect(AppBrand.authorLine == "Author: Miller")
    }

    @Test("Formats a complete bundle version")
    func completeVersion() {
        #expect(
            AppBrand.versionText(info: [
                "CFBundleShortVersionString": "0.1.0",
                "CFBundleVersion": "1",
            ]) == "Version 0.1.0 (1)"
        )
    }

    @Test("Does not invent a missing bundle version")
    func missingVersion() {
        #expect(AppBrand.versionText(info: [:]) == "Version unavailable")
        #expect(
            AppBrand.versionText(info: ["CFBundleShortVersionString": "0.1.0"])
                == "Version unavailable"
        )
    }
}
