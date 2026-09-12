import AIMeterCore
import AppKit
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("Provider logo rendering")
@MainActor
struct ProviderLogoTests {
    @Test("Settings tint resolves to the primary foreground color in light appearance")
    func settingsTintUsesPrimaryForegroundInLightAppearance() throws {
        for provider in UsageProvider.allCases {
            let bitmap = try render(
                ProviderLogo(provider: provider, size: 18, tint: .primary),
                colorScheme: .light
            )

            let opaquePixels = try opaquePixels(in: bitmap)
            #expect(!opaquePixels.isEmpty, "\(provider.rawValue) did not render")
            #expect(
                opaquePixels.allSatisfy { luminance(of: $0) < 0.2 },
                "\(provider.rawValue) did not resolve .primary to a dark light-mode foreground"
            )
        }
    }

    @Test("Tint leaves the existing 18pt logo alpha masks unchanged")
    func tintKeepsExistingLogoGeometry() throws {
        for provider in UsageProvider.allCases {
            let existing = try render(ProviderLogo(provider: provider, size: 18), colorScheme: .dark)
            let tinted = try render(ProviderLogo(provider: provider, size: 18, tint: .white), colorScheme: .dark)

            #expect(existing.pixelsWide == 36)
            #expect(existing.pixelsHigh == 36)
            #expect(try alphaFootprint(in: existing) == alphaFootprint(in: tinted),
                    "\(provider.rawValue) changed its resource or optical correction when tinted")
            #expect(try opaquePixels(in: existing).allSatisfy { luminance(of: $0) > 0.8 },
                    "\(provider.rawValue) no longer defaults to the existing white rendering")
        }
    }

    private func render<V: View>(_ view: V, colorScheme: ColorScheme) throws -> NSBitmapImageRep {
        let size = NSSize(width: 18, height: 18)
        let host = NSHostingView(rootView: view
            .frame(width: size.width, height: size.height)
            .environment(\.colorScheme, colorScheme))
        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: size),
            styleMask: .borderless,
            backing: .buffered,
            defer: false
        )
        window.isReleasedWhenClosed = false
        defer { window.close() }
        window.contentView = host
        host.frame = NSRect(origin: .zero, size: size)
        window.layoutIfNeeded()
        host.layoutSubtreeIfNeeded()

        let bitmap = try #require(NSBitmapImageRep(
            bitmapDataPlanes: nil,
            pixelsWide: 36,
            pixelsHigh: 36,
            bitsPerSample: 8,
            samplesPerPixel: 4,
            hasAlpha: true,
            isPlanar: false,
            colorSpaceName: .deviceRGB,
            bytesPerRow: 0,
            bitsPerPixel: 0
        ))
        bitmap.size = size
        host.cacheDisplay(in: host.bounds, to: bitmap)
        return bitmap
    }

    private func opaquePixels(in bitmap: NSBitmapImageRep) throws -> [NSColor] {
        try (0..<bitmap.pixelsHigh).flatMap { y in
            try (0..<bitmap.pixelsWide).compactMap { x in
                let color = try #require(bitmap.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB))
                return color.alphaComponent > 0.6 ? color : nil
            }
        }
    }

    private func alphaFootprint(in bitmap: NSBitmapImageRep) throws -> [Bool] {
        try (0..<bitmap.pixelsHigh).flatMap { y in
            try (0..<bitmap.pixelsWide).map { x in
                let color = try #require(bitmap.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB))
                return color.alphaComponent > 0.1
            }
        }
    }

    private func luminance(of color: NSColor) -> CGFloat {
        0.2126 * color.redComponent + 0.7152 * color.greenComponent + 0.0722 * color.blueComponent
    }
}
