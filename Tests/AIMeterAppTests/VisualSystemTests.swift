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

    @Test("Floating island tapers from edge points through symmetric S curves")
    func floatingStripSCurveShoulders() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
        let right = FloatingStripShape(edge: .right).path(in: rect)
        let left = FloatingStripShape(edge: .left).path(in: rect)

        let insideTop = [
            CGPoint(x: 107, y: 20),
            CGPoint(x: 100, y: 40),
            CGPoint(x: 75, y: 58),
        ]
        let outsideTop = [
            CGPoint(x: 90, y: 1),
            CGPoint(x: 90, y: 20),
            CGPoint(x: 70, y: 40),
            CGPoint(x: 40, y: 58),
        ]

        for point in insideTop {
            #expect(right.contains(point))
            #expect(right.contains(CGPoint(x: point.x, y: rect.maxY - point.y)))
            #expect(left.contains(CGPoint(x: rect.maxX - point.x, y: point.y)))
        }
        for point in outsideTop {
            #expect(!right.contains(point))
            #expect(!right.contains(CGPoint(x: point.x, y: rect.maxY - point.y)))
            #expect(!left.contains(CGPoint(x: rect.maxX - point.x, y: point.y)))
        }

        let settlingOutside = [
            CGPoint(x: 0, y: 94),
            CGPoint(x: 0, y: 100),
        ]
        for point in settlingOutside {
            #expect(!right.contains(point))
            #expect(!right.contains(CGPoint(x: point.x, y: rect.maxY - point.y)))
            #expect(!left.contains(CGPoint(x: rect.maxX - point.x, y: point.y)))
        }

        #expect(right.contains(CGPoint(x: 8, y: 100)))
        #expect(right.contains(CGPoint(x: 1, y: 110)))

        #expect(right.contains(CGPoint(x: 106, y: 178)))
        #expect(left.contains(CGPoint(x: 2, y: 178)))
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

    @Test("Floating island surface paints the attached edge and body")
    @MainActor
    func floatingSurfacePaintsAttachedEdgeAndBody() throws {
        let renderer = ImageRenderer(content:
            FloatingStripSurface(edge: .right)
                .frame(width: 108, height: 356)
        )
        renderer.scale = 1
        let image = try #require(renderer.cgImage)

        for point in [(0, 178), (107, 1), (107, 178), (107, 354)] {
            #expect(try alpha(atX: point.0, y: point.1, in: image) > 0)
        }
    }

    @Test("Floating island has no shadow outside its visible shoulder")
    @MainActor
    func floatingSurfaceHasNoExteriorShadow() throws {
        let renderer = ImageRenderer(content:
            FloatingStripSurface(edge: .right)
                .frame(width: 108, height: 356)
        )
        renderer.scale = 1
        let image = try #require(renderer.cgImage)

        #expect(try alpha(atX: 0, y: 94, in: image) == 0)
    }

    @Test("Every non-normal usage state has a non-color symbol")
    func semanticSymbols() {
        #expect(UsageSemantic.normal.statusSymbolName == nil)
        for semantic in [UsageSemantic.warning, .critical, .stale, .unavailable] {
            #expect(semantic.statusSymbolName != nil)
        }
    }

    @Test("Each provider owns the approved unique brand palette")
    func providerBrandPalettes() {
        #expect(
            UsageProvider.claude.accentPalette
                == .init(startHex: 0xE8B96D, endHex: 0xD97757)
        )
        #expect(
            UsageProvider.codex.accentPalette
                == .init(startHex: 0xFF6FAE, endHex: 0xA96DFF)
        )
        #expect(
            UsageProvider.deepSeek.accentPalette
                == .init(startHex: 0x54EDC6, endHex: 0x7769FF)
        )
        #expect(Set(UsageProvider.allCases.map(\.accentPalette)).count == 3)
    }

    @Test("DeepSeek always keeps its balance palette while other providers use semantic overrides")
    func semanticAccentPrecedence() {
        for semantic in [
            UsageSemantic.normal,
            .warning,
            .critical,
            .stale,
            .unavailable,
        ] {
            #expect(semantic.accentRole(for: .deepSeek) == .provider(.deepSeek))
        }

        #expect(UsageSemantic.normal.accentRole(for: .claude) == .provider(.claude))
        #expect(UsageSemantic.normal.accentRole(for: .codex) == .provider(.codex))
        for semantic in [UsageSemantic.warning, .critical, .stale, .unavailable] {
            #expect(semantic.accentRole(for: .claude) == .semantic(semantic))
            #expect(semantic.accentRole(for: .codex) == .semantic(semantic))
        }
    }

    @Test("DeepSeek progress stays branded while other warning bars stay semantic")
    @MainActor
    func providerProgressBarColors() throws {
        let normalColors = try UsageProvider.allCases.map {
            try progressBarPixel(provider: $0, semantic: .normal)
        }
        #expect(Set(normalColors).count == 3)

        let claudeWarning = try progressBarPixel(provider: .claude, semantic: .warning)
        let codexWarning = try progressBarPixel(provider: .codex, semantic: .warning)
        let deepSeekWarning = try progressBarPixel(provider: .deepSeek, semantic: .warning)
        #expect(claudeWarning == codexWarning)
        #expect(deepSeekWarning != claudeWarning)

        let deepSeekNormal = try progressBarPixel(provider: .deepSeek, semantic: .normal)
        let deepSeekStale = try progressBarPixel(provider: .deepSeek, semantic: .stale)
        #expect(deepSeekStale == deepSeekNormal)
    }

    @MainActor
    private func progressBarPixel(
        provider: UsageProvider,
        semantic: UsageSemantic
    ) throws -> PixelRGB {
        let renderer = ImageRenderer(content:
            AIMeterProgressBar(
                provider: provider,
                fraction: 1,
                semantic: semantic
            )
            .frame(width: 120, height: 5)
        )
        renderer.scale = 1
        let image = try #require(renderer.cgImage)
        let data = try #require(image.dataProvider?.data)
        let bytes = try #require(CFDataGetBytePtr(data))
        let pixelOffset = 2 * image.bytesPerRow + 60 * image.bitsPerPixel / 8
        return PixelRGB(
            red: bytes[pixelOffset],
            green: bytes[pixelOffset + 1],
            blue: bytes[pixelOffset + 2]
        )
    }

    private func alpha(atX x: Int, y: Int, in image: CGImage) throws -> UInt8 {
        let data = try #require(image.dataProvider?.data)
        let bytes = CFDataGetBytePtr(data)
        let alphaIndex = y * image.bytesPerRow + x * image.bitsPerPixel / 8 + 3
        return try #require(bytes?[alphaIndex])
    }

    private struct PixelRGB: Hashable {
        let red: UInt8
        let green: UInt8
        let blue: UInt8
    }
}
