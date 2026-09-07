import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("About brand links")
struct BrandLinksViewTests {
    @Test("A selected author link is handed to the injected browser boundary")
    func opensSelectedLink() {
        var opened: URL?
        let action = BrandLinkOpenAction { url in
            opened = url
            return true
        }

        let succeeded = action.open(AppBrand.authorLinks[0])

        #expect(succeeded)
        #expect(opened?.absoluteString == "https://twitter.com/MillerPanYue")
    }

    @Test("A rejected browser open remains observable to the view")
    func reportsRejectedOpen() {
        let action = BrandLinkOpenAction { _ in false }

        #expect(!action.open(AppBrand.authorLinks[1]))
    }
}
