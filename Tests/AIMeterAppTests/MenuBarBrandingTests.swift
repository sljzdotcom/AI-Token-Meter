import AIMeterCore
import AppKit
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("Menu bar panel branding")
@MainActor
struct MenuBarBrandingTests {
    @Test("Application icon occupies a 32-point leading square in the real panel")
    func applicationIconLeadsPanelTitle() async throws {
        let suite = "MenuBarBranding-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let model = AppModel(
            defaults: defaults,
            secretStore: MenuBarBrandingSecretStore(),
            widgetSnapshotPublisher: nil,
            isDemoMode: true
        )
        let icon = solidIcon(color: .magenta)

        let bitmap = try await render(
            MenuBarPanel(model: model, brandIcon: icon),
            width: 380,
            height: 320
        )
        try save(bitmap)

        let magenta = try matchingPixelBounds(in: bitmap) { color in
            color.redComponent > 0.92
                && color.greenComponent < 0.08
                && color.blueComponent > 0.92
                && color.alphaComponent > 0.92
        }
        let bounds = try #require(magenta)
        #expect((62...64).contains(Int(bounds.width)))
        #expect((62...64).contains(Int(bounds.height)))
        #expect(bounds.maxX < 120)
    }

    private func solidIcon(color: NSColor) -> NSImage {
        let image = NSImage(size: NSSize(width: 16, height: 16))
        image.lockFocus()
        color.setFill()
        NSRect(x: 0, y: 0, width: 16, height: 16).fill()
        image.unlockFocus()
        return image
    }

    private func render<V: View>(
        _ view: V,
        width: Double,
        height: Double
    ) async throws -> NSBitmapImageRep {
        let root = view
            .frame(width: width, height: height, alignment: .topLeading)
            .environment(\.colorScheme, .dark)
        let host = NSHostingView(rootView: root)
        let frame = NSRect(x: 0, y: 0, width: width, height: height)
        let window = NSWindow(
            contentRect: frame,
            styleMask: .borderless,
            backing: .buffered,
            defer: false
        )
        window.isReleasedWhenClosed = false
        defer { window.close() }
        host.frame = frame
        window.contentView = host
        window.layoutIfNeeded()
        host.layoutSubtreeIfNeeded()
        try await Task.sleep(for: .milliseconds(25))
        host.layoutSubtreeIfNeeded()
        let bitmap = try #require(NSBitmapImageRep(
            bitmapDataPlanes: nil,
            pixelsWide: Int(width * 2),
            pixelsHigh: Int(height * 2),
            bitsPerSample: 8,
            samplesPerPixel: 4,
            hasAlpha: true,
            isPlanar: false,
            colorSpaceName: .deviceRGB,
            bytesPerRow: 0,
            bitsPerPixel: 0
        ))
        bitmap.size = NSSize(width: width, height: height)
        host.cacheDisplay(in: host.bounds, to: bitmap)
        return bitmap
    }

    private func matchingPixelBounds(
        in bitmap: NSBitmapImageRep,
        predicate: (NSColor) -> Bool
    ) throws -> CGRect? {
        var minX = bitmap.pixelsWide
        var minY = bitmap.pixelsHigh
        var maxX = -1
        var maxY = -1
        for y in 0..<bitmap.pixelsHigh {
            for x in 0..<bitmap.pixelsWide {
                guard let color = bitmap.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB),
                      predicate(color) else { continue }
                minX = min(minX, x)
                minY = min(minY, y)
                maxX = max(maxX, x)
                maxY = max(maxY, y)
            }
        }
        guard maxX >= minX, maxY >= minY else { return nil }
        return CGRect(
            x: minX,
            y: minY,
            width: maxX - minX + 1,
            height: maxY - minY + 1
        )
    }

    private func save(_ bitmap: NSBitmapImageRep) throws {
        guard let directoryPath = ProcessInfo.processInfo.environment[
            "AI_METER_DOC_SCREENSHOT_DIR"
        ] else { return }
        let directory = URL(fileURLWithPath: directoryPath, isDirectory: true)
        try FileManager.default.createDirectory(
            at: directory,
            withIntermediateDirectories: true
        )
        let png = try #require(bitmap.representation(using: .png, properties: [:]))
        try png.write(to: directory.appendingPathComponent("menu-panel-brand-header.png"))
    }
}

private struct MenuBarBrandingSecretStore: SecretStore {
    func read() throws -> String? { nil }
    func save(_ secret: String) throws {}
    func delete() throws {}
}
