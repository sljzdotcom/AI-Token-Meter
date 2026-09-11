import AIMeterCore
import AppKit
import Foundation
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("AI Meter visual system")
struct VisualSystemTests {
    @Test("Restore default is enabled only for custom selections")
    func restoreDefaultFontState() {
        #expect(!DisplayFontSettingsPresentation.canRestore(.system))
        #expect(DisplayFontSettingsPresentation.canRestore(.antonio))
        #expect(DisplayFontSettingsPresentation.canRestore(.dinCondensed))
    }

    @Test("Default deep sea background reuses one decoded image")
    @MainActor
    func floatingBackgroundCache() throws {
        let first = try #require(FloatingStripBackgroundAsset.defaultImage)
        let second = try #require(FloatingStripBackgroundAsset.defaultImage)

        #expect(first === second)
    }

    @Test("Cached AppKit images stay on the main actor")
    func floatingBackgroundCacheIsMainActorIsolated() throws {
        let projectRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let source = try String(contentsOf: projectRoot.appending(
            path: "Sources/AIMeterApp/Views/FloatingStripBackground.swift"
        ))

        #expect(source.contains("@MainActor static let defaultImage = load()"))
    }

    @Test("Deep sea background is bundled at retina resolution")
    func floatingBackgroundResource() throws {
        let url = try #require(FloatingStripBackgroundAsset.resourceURL())
        let image = try #require(NSImage(contentsOf: url))

        #expect(image.size.width >= 324)
        #expect(image.size.height >= 1068)
    }

    @Test("Background crop uses equal axis magnitudes and mirrors only X")
    func floatingBackgroundScale() {
        let right = FloatingStripBackgroundPresentation.scale(for: .right)
        let left = FloatingStripBackgroundPresentation.scale(for: .left)

        #expect(right.width == 1.22)
        #expect(right.height == 1.22)
        #expect(left.width == -right.width)
        #expect(left.height == right.height)
        #expect(abs(left.width) == abs(left.height))
    }

    @Test("Only the strip background mirrors with the attached edge")
    @MainActor
    func floatingBackgroundMirrorsWithEdge() throws {
        let background = try splitColorBackgroundImage()
        let right = try renderSurface(edge: .right, backgroundImage: background)
        let left = try renderSurface(edge: .left, backgroundImage: background)

        let rightLeading = try rgb(atX: 20, y: 202, in: right)
        let rightTrailing = try rgb(atX: 88, y: 202, in: right)
        let leftLeading = try rgb(atX: 20, y: 202, in: left)
        let leftTrailing = try rgb(atX: 88, y: 202, in: left)

        #expect(rightLeading.red > rightLeading.blue)
        #expect(rightTrailing.blue > rightTrailing.red)
        #expect(leftLeading.blue > leftLeading.red)
        #expect(leftTrailing.red > leftTrailing.blue)
    }

    @Test("Uniform crop removes black gutters from both visible shoulders")
    @MainActor
    func floatingBackgroundFillsShoulders() throws {
        let background = try blackGutterBlueCenterImage()

        for edge in [FloatingStripEdge.left, .right] {
            let rendered = try renderSurface(edge: edge, backgroundImage: background)
            let attachedX = edge == .right ? 107 : 0
            for y in [20, 336] {
                let color = try rgb(atX: attachedX, y: y, in: rendered)
                #expect(color.blue > 100)
                #expect(color.blue > color.red)
            }
        }
    }

    @Test("Missing background keeps the glass fallback and exact shoulder mask")
    @MainActor
    func floatingBackgroundFallback() throws {
        let image = try renderSurface(edge: .right, backgroundImage: nil)

        #expect(try alpha(atX: 0, y: 202, in: image) > 0)
        #expect(try alpha(atX: 107, y: 2, in: image) == 0)
        #expect(try alpha(atX: 107, y: 354, in: image) == 0)
    }

    @Test("Provider logos use one optical calibration table")
    func providerLogoScales() {
        #expect(ProviderLogoStyle.opticalScale(for: .claude) > 1)
        #expect(ProviderLogoStyle.opticalScale(for: .codex) == 1)
        #expect(ProviderLogoStyle.opticalScale(for: .deepSeek) < 1)
    }

    @Test("Comfortable contour ends with equal top and bottom insets")
    func floatingStripBounds() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 356)
        let inset = CGFloat(88) * 4 / 70

        for edge in [FloatingStripEdge.left, .right] {
            let bounds = FloatingStripShape(edge: edge).path(in: rect).boundingRect
            #expect(abs(bounds.minY - inset) < 0.001)
            #expect(abs(bounds.maxY - (rect.maxY - inset)) < 0.001)
            #expect(bounds.minX == 0)
            #expect(bounds.maxX == 108)
        }
    }

    @Test("Expanded contour uses the approved rounded shoulder landmarks")
    func approvedRoundedShoulderLandmarks() {
        let mini = FloatingStripContour.geometry(for: .mini)
        #expect(mini.start == CGPoint(x: 65, y: 4))
        #expect(mini.shoulderDepth == 70)
        #expect(mini.curves == [
            .init(control1: CGPoint(x: 63, y: 18), control2: CGPoint(x: 54, y: 29), end: CGPoint(x: 37, y: 30)),
            .init(control1: CGPoint(x: 18, y: 31), control2: CGPoint(x: 5, y: 42), end: CGPoint(x: 1, y: 58)),
            .init(control1: CGPoint(x: 0, y: 62), control2: CGPoint(x: 0, y: 66), end: CGPoint(x: 0, y: 70)),
        ])
        let compact = FloatingStripContour.geometry(for: .compact)
        #expect(compact.start == CGPoint(x: 78, y: 4))
        #expect(compact.shoulderDepth == 70)
        let comfortable = FloatingStripContour.geometry(for: .comfortable)
        #expect(comfortable.start == CGPoint(x: 108, y: CGFloat(88) * 4 / 70))
        #expect(comfortable.shoulderDepth == 88)
        #expect(comfortable.curves.last?.control2.x == 0)
        #expect(comfortable.curves.last?.end.x == 0)
    }

    @Test("Every expanded lower shoulder is the vertical mirror of its upper shoulder")
    func floatingStripShouldersMirrorVertically() {
        for density in FloatingStripDensity.allCases {
            let rect = CGRect(x: 0, y: 0, width: density.width,
                              height: density.height(providerCount: 4))
            let path = FloatingStripShape(edge: .right, density: density, providerCount: 4)
                .path(in: rect)
            var asymmetricSamples = 0
            for x in stride(from: 0.5, to: density.width, by: 2) {
                for y in stride(from: 0.5, to: rect.midY, by: 2) {
                    if path.contains(CGPoint(x: x, y: y))
                        != path.contains(CGPoint(x: x, y: rect.maxY - y)) {
                        asymmetricSamples += 1
                    }
                }
            }
            #expect(asymmetricSamples == 0,
                    "\(density.rawValue) has \(asymmetricSamples) asymmetric samples")
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

        for point in [(0, 178), (107, 20), (107, 178), (107, 330)] {
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

        #expect(try alpha(atX: 107, y: 2, in: image) == 0)
        #expect(try alpha(atX: 107, y: 354, in: image) == 0)
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
        #expect(Set(UsageProvider.allCases.map(\.accentPalette)).count == 4)
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
        #expect(Set(normalColors).count == 4)

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

    private func blackGutterBlueCenterImage() throws -> NSImage {
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
                let isGutter = y < 32 || y >= 324
                bytes[offset] = 0
                bytes[offset + 1] = 0
                bytes[offset + 2] = isGutter ? 0 : 255
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
