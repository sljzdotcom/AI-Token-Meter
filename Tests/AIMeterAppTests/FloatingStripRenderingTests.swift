import AIMeterCore
import AppKit
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("Floating strip rendered background")
@MainActor
struct FloatingStripRenderingTests {
    @Test("Expanded top remains uninterrupted glass on both edges and densities")
    func expandedTopHasNoDecoration() throws {
        let suite = "FloatingStripRendering-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let model = AppModel(defaults: defaults, secretStore: RenderingSecretStore(),
                             widgetSnapshotPublisher: nil, isDemoMode: true)
        for density in FloatingStripDensity.allCases {
            var preferences = FloatingStripPreferences()
            preferences.density = density
            model.setStripPreferences(preferences)
            for edge in [FloatingStripEdge.left, .right] {
                let strip = FloatingStripView(model: model, session: FloatingDetailSession(),
                    displayState: FloatingStripDisplayState(resolvedEdge: edge),
                    onProviderTap: { _ in }, onAccessibilityMove: { _ in })
                let actual = try render(strip, width: density.width, height: density.baseHeight)
                let surface = try render(FloatingStripSurface(edge: edge, density: density, providerCount: 3),
                                         width: density.width, height: density.baseHeight)
                let name = "expanded-\(density.rawValue)-\(edge == .left ? "left" : "right")"
                try save(actual, name: name)
                try save(surface, name: "\(name)-surface")
                #expect(actual.pixelsWide == Int(density.width * 2))
                #expect(actual.pixelsHigh == Int(density.baseHeight * 2))
                // Compare a clear top band, above all Provider buttons, with the real glass.
                // A reintroduced horizontal decoration changes these pixels; logos cannot.
                var changedPixels = 0
                let top = density == .compact ? 32 : 40
                for y in (top * 2)..<((top + 10) * 2) {
                    for x in (Int(density.width) - 24)..<(Int(density.width) + 24) {
                        let a = try #require(actual.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB))
                        let b = try #require(surface.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB))
                        if max(abs(a.redComponent - b.redComponent),
                               abs(a.greenComponent - b.greenComponent),
                               abs(a.blueComponent - b.blueComponent)) > 2.0 / 255.0 {
                            changedPixels += 1
                        }
                    }
                }
                #expect(changedPixels == 0, "\(name): top glass has \(changedPixels) decorated pixels")
                let foldedState = FloatingStripDisplayState(resolvedEdge: edge)
                foldedState.isFolded = true
                let folded = FloatingStripView(model: model, session: FloatingDetailSession(),
                    displayState: foldedState, onProviderTap: { _ in }, onAccessibilityMove: { _ in })
                let frame = FloatingStripLayout.foldedFrame(
                    from: CGRect(x: 0, y: 0, width: density.width, height: density.baseHeight), edge: edge)
                try save(render(folded, width: frame.width, height: frame.height),
                         name: "folded-\(density.rawValue)-\(edge == .left ? "left" : "right")")
            }
        }
    }

    private func render<V: View>(_ view: V, width: Double, height: Double) throws -> NSBitmapImageRep {
        let renderer = ImageRenderer(content: view.frame(width: width, height: height)
            .environment(\.colorScheme, .dark))
        renderer.scale = 2
        let image = try #require(renderer.nsImage)
        let tiff = try #require(image.tiffRepresentation)
        return try #require(NSBitmapImageRep(data: tiff))
    }

    private func save(_ bitmap: NSBitmapImageRep, name: String) throws {
        guard let path = ProcessInfo.processInfo.environment["AI_METER_DOC_SCREENSHOT_DIR"] else { return }
        let directory = URL(fileURLWithPath: path, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let png = try #require(bitmap.representation(using: .png, properties: [:]))
        try png.write(to: directory.appendingPathComponent("\(name).png"))
    }
}

private struct RenderingSecretStore: SecretStore {
    func read() throws -> String? { nil }
    func save(_ secret: String) throws {}
    func delete() throws {}
}
