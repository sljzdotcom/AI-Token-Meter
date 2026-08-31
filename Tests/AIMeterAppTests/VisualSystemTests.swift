import AIMeterCore
import Foundation
import SwiftUI
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

    @Test("Floating island uses the approved reverse semicircle shoulders")
    func floatingStripReverseSemicircleShoulders() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
        let right = FloatingStripShape(edge: .right).path(in: rect)
        let left = FloatingStripShape(edge: .left).path(in: rect)

        #expect(right.contains(CGPoint(x: 70, y: 8)))
        #expect(!right.contains(CGPoint(x: 45, y: 45)))
        #expect(right.contains(CGPoint(x: 70, y: 348)))
        #expect(!right.contains(CGPoint(x: 45, y: 311)))

        #expect(left.contains(CGPoint(x: 38, y: 8)))
        #expect(!left.contains(CGPoint(x: 63, y: 45)))
        #expect(left.contains(CGPoint(x: 38, y: 348)))
        #expect(!left.contains(CGPoint(x: 63, y: 311)))
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

    @Test("Floating island surface paints every window edge")
    @MainActor
    func floatingSurfaceIsOpaqueAtWindowEdges() throws {
        let renderer = ImageRenderer(content:
            FloatingStripSurface(edge: .right)
                .frame(width: 108, height: 356)
        )
        renderer.scale = 1
        let image = try #require(renderer.cgImage)

        for point in [(0, 178), (107, 178), (54, 0), (54, 355)] {
            #expect(try alpha(atX: point.0, y: point.1, in: image) > 0)
        }
    }

    @Test("Every non-normal usage state has a non-color symbol")
    func semanticSymbols() {
        #expect(UsageSemantic.normal.statusSymbolName == nil)
        for semantic in [UsageSemantic.warning, .critical, .stale, .unavailable] {
            #expect(semantic.statusSymbolName != nil)
        }
    }

    private func alpha(atX x: Int, y: Int, in image: CGImage) throws -> UInt8 {
        let data = try #require(image.dataProvider?.data)
        let bytes = CFDataGetBytePtr(data)
        let alphaIndex = y * image.bytesPerRow + x * image.bitsPerPixel / 8 + 3
        return try #require(bytes?[alphaIndex])
    }
}
