import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("AI Meter visual system")
struct VisualSystemTests {
    @Test("Provider logos use one optical calibration table")
    func providerLogoScales() {
        #expect(ProviderLogoStyle.opticalScale(for: .claude) > 1)
        #expect(ProviderLogoStyle.opticalScale(for: .codex) == 1)
        #expect(ProviderLogoStyle.opticalScale(for: .deepSeek) < 1)
    }

    @Test("Both island orientations fill the complete window without transparent edge gaps")
    func floatingStripBounds() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 356)

        #expect(FloatingStripShape(edge: .right).path(in: rect).boundingRect == rect)
        #expect(FloatingStripShape(edge: .left).path(in: rect).boundingRect == rect)
    }

    @Test("Panel, card, and capsule geometry forms a strict hierarchy")
    func geometryHierarchy() {
        #expect(AIMeterVisualTheme.panelCornerRadius > AIMeterVisualTheme.cardCornerRadius)
        #expect(AIMeterVisualTheme.cardCornerRadius > AIMeterVisualTheme.capsuleInsetRadius)
        #expect(AIMeterVisualTheme.panelPadding == 20)
    }

    @Test("App bundle declares the generated meter icon")
    func appIconConfiguration() throws {
        let projectRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let plistURL = projectRoot.appending(
            path: "Sources/AIMeterApp/Resources/Info.plist"
        )
        let data = try Data(contentsOf: plistURL)
        let plist = try #require(
            PropertyListSerialization.propertyList(from: data, format: nil)
                as? [String: Any]
        )

        #expect(plist["CFBundleIconFile"] as? String == "AppIcon")
    }
}
