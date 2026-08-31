import AIMeterCore
import AppKit
import Foundation
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("AI Meter visual system")
struct VisualSystemTests {
    @Test("Deep sea background is bundled at retina resolution")
    func floatingBackgroundResource() throws {
        let url = try #require(FloatingStripBackgroundAsset.resourceURL())
        let image = try #require(NSImage(contentsOf: url))

        #expect(image.size.width >= 324)
        #expect(image.size.height >= 1068)
    }

    @Test("Only the strip background mirrors with the attached edge")
    @MainActor
    func floatingBackgroundMirrorsWithEdge() throws {
        let background = try splitColorBackgroundImage()
        let right = try renderSurface(edge: .right, backgroundImage: background)
        let left = try renderSurface(edge: .left, backgroundImage: background)

        let rightLeading = try rgb(atX: 20, y: 178, in: right)
        let rightTrailing = try rgb(atX: 88, y: 178, in: right)
        let leftLeading = try rgb(atX: 20, y: 178, in: left)
        let leftTrailing = try rgb(atX: 88, y: 178, in: left)

        #expect(rightLeading.red > rightLeading.blue)
        #expect(rightTrailing.blue > rightTrailing.red)
        #expect(leftLeading.blue > leftLeading.red)
        #expect(leftTrailing.red > leftTrailing.blue)
    }

    @Test("Missing background keeps the glass fallback and exact shoulder mask")
    @MainActor
    func floatingBackgroundFallback() throws {
        let image = try renderSurface(edge: .right, backgroundImage: nil)

        #expect(try alpha(atX: 0, y: 178, in: image) > 0)
        #expect(try alpha(atX: 107, y: 8, in: image) == 0)
        #expect(try alpha(atX: 107, y: 348, in: image) == 0)
    }

    @Test("Provider logos use one optical calibration table")
    func providerLogoScales() {
        #expect(ProviderLogoStyle.opticalScale(for: .claude) > 1)
        #expect(ProviderLogoStyle.opticalScale(for: .codex) == 1)
        #expect(ProviderLogoStyle.opticalScale(for: .deepSeek) < 1)
    }

    @Test("Both compact shoulders use the approved inset bounds")
    func floatingStripBounds() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
        let expected = CGRect(x: 0, y: 16, width: 108, height: 324)

        #expect(FloatingStripShape(edge: .right).path(in: rect).boundingRect == expected)
        #expect(FloatingStripShape(edge: .left).path(in: rect).boundingRect == expected)
    }

    @Test("Compact shoulders match the approved short shelf and mirrored arc")
    func floatingStripCompactShoulders() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
        let right = FloatingStripShape(edge: .right).path(in: rect)
        let left = FloatingStripShape(edge: .left).path(in: rect)

        for point in [
            CGPoint(x: 40, y: 50),
            CGPoint(x: 8, y: 82),
            CGPoint(x: 1, y: 94),
        ] {
            #expect(right.contains(point))
            #expect(right.contains(CGPoint(x: point.x, y: rect.maxY - point.y)))
            #expect(left.contains(CGPoint(x: rect.maxX - point.x, y: point.y)))
        }
        for point in [
            CGPoint(x: 107, y: 8),
            CGPoint(x: 40, y: 30),
            CGPoint(x: 0, y: 82),
        ] {
            #expect(!right.contains(point))
            #expect(!right.contains(CGPoint(x: point.x, y: rect.maxY - point.y)))
            #expect(!left.contains(CGPoint(x: rect.maxX - point.x, y: point.y)))
        }
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

        for point in [(0, 178), (107, 20), (107, 178), (107, 336)] {
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

        #expect(try alpha(atX: 107, y: 8, in: image) == 0)
        #expect(try alpha(atX: 107, y: 348, in: image) == 0)
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

    @MainActor
    private func renderSurface(
        edge: FloatingStripEdge,
        backgroundImage: NSImage?
    ) throws -> CGImage {
        let renderer = ImageRenderer(content:
            FloatingStripSurface(edge: edge, backgroundImage: backgroundImage)
                .frame(width: 108, height: 356)
        )
        renderer.scale = 1
        return try #require(renderer.cgImage)
    }

    private func rgb(atX x: Int, y: Int, in image: CGImage) throws -> PixelRGB {
        let representation = NSBitmapImageRep(cgImage: image)
        let color = try #require(
            representation.colorAt(x: x, y: image.height - 1 - y)?
                .usingColorSpace(.deviceRGB)
        )
        return PixelRGB(
            red: UInt8((color.redComponent * 255).rounded()),
            green: UInt8((color.greenComponent * 255).rounded()),
            blue: UInt8((color.blueComponent * 255).rounded())
        )
    }

    private func splitColorBackgroundImage() throws -> NSImage {
        let width = 108
        let height = 356
        let representation = try #require(NSBitmapImageRep(
            bitmapDataPlanes: nil,
            pixelsWide: width,
            pixelsHigh: height,
            bitsPerSample: 8,
            samplesPerPixel: 4,
            hasAlpha: true,
            isPlanar: false,
            colorSpaceName: .deviceRGB,
            bytesPerRow: width * 4,
            bitsPerPixel: 32
        ))
        let bytes = try #require(representation.bitmapData)
        for y in 0..<height {
            for x in 0..<width {
                let offset = y * width * 4 + x * 4
                bytes[offset] = x < width / 2 ? 255 : 0
                bytes[offset + 1] = 0
                bytes[offset + 2] = x < width / 2 ? 0 : 255
                bytes[offset + 3] = 255
            }
        }

        let image = NSImage(size: NSSize(width: width, height: height))
        image.addRepresentation(representation)
        return image
    }

    private struct PixelRGB: Hashable {
        let red: UInt8
        let green: UInt8
        let blue: UInt8
    }
}
