import AIMeterCore
import AppKit
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("Floating strip rendered background")
@MainActor
struct FloatingStripRenderingTests {
    @Test("Expanded top remains uninterrupted glass on both edges and densities")
    func expandedTopHasNoDecoration() async throws {
        let suite = "FloatingStripRendering-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let model = AppModel(defaults: defaults, secretStore: RenderingSecretStore(),
                             widgetSnapshotPublisher: nil, isDemoMode: true)
        for count in 1...4 {
            for density in FloatingStripDensity.allCases {
                var preferences = FloatingStripPreferences()
                preferences.density = density
                preferences.hiddenProviders = Array(UsageProvider.allCases.dropFirst(count))
                model.setStripPreferences(preferences)
                for edge in [FloatingStripEdge.left, .right] {
                    let strip = FloatingStripView(model: model, session: FloatingDetailSession(),
                        displayState: FloatingStripDisplayState(resolvedEdge: edge),
                        onProviderTap: { _ in }, onAccessibilityMove: { _ in })
                    let actual = try await render(strip, width: density.width, height: density.height(providerCount: count))
                    let surface = try await render(FloatingStripSurface(edge: edge, density: density, providerCount: count),
                                             width: density.width, height: density.height(providerCount: count))
                    let name = "expanded-\(count)-\(density.rawValue)-\(edge == .left ? "left" : "right")"
                    try save(actual, name: name)
                    try save(surface, name: "\(name)-surface")
                    #expect(actual.pixelsWide == Int(density.width))
                    #expect(actual.pixelsHigh == Int(density.height(providerCount: count)))
                    // Compare a clear top band, above all Provider buttons, with the real glass.
                    // A reintroduced horizontal decoration changes these pixels; logos cannot.
                    var changedPixels = 0
                    let top = density == .compact ? 32 : 40
                    for y in top..<(top + 10) {
                        for x in (Int(density.width / 2) - 12)..<(Int(density.width / 2) + 12) {
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
                    let reference = try await render(ZStack {
                        FloatingStripSurface(edge: edge, density: density, providerCount: count)
                        VStack(spacing: density.spacing) {
                            ForEach(Array(UsageProvider.allCases.prefix(count)), id: \.self) { provider in
                                ProviderLogo(provider: provider, size: density.ringSize * 0.44)
                                    .frame(width: density.ringSize, height: density.ringSize)
                            }
                        }
                    }, width: density.width, height: density.height(providerCount: count))
                    let rect = CGRect(x: 0, y: 0, width: density.width, height: density.height(providerCount: count))
                    let frames = FloatingStripContentLayout.providerFrames(in: rect, density: density, count: count)
                    for frame in frames {
                        #expect(rect.contains(frame))
                        // Bright inner-logo pixels must occupy the same coordinates on both edges.
                        var brightLogoPixels = 0
                        var mismatchedLogoPixels = 0
                        for y in Int(frame.midY - 7)..<Int(frame.midY + 7) {
                            for x in Int(frame.midX - 7)..<Int(frame.midX + 7) {
                                let expected = try #require(reference.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB))
                                if min(expected.redComponent, expected.greenComponent, expected.blueComponent) > 0.92 {
                                    brightLogoPixels += 1
                                    let pixel = try #require(actual.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB))
                                    if min(pixel.redComponent, pixel.greenComponent, pixel.blueComponent) < 0.85 { mismatchedLogoPixels += 1 }
                                }
                            }
                        }
                        #expect(brightLogoPixels > 5)
                        #expect(mismatchedLogoPixels == 0, "Logo orientation changed: \(name)")
                    }
                    let foldedState = FloatingStripDisplayState(resolvedEdge: edge)
                    foldedState.isFolded = true
                    let folded = FloatingStripView(model: model, session: FloatingDetailSession(),
                        displayState: foldedState, onProviderTap: { _ in }, onAccessibilityMove: { _ in })
                    let frame = FloatingStripLayout.foldedFrame(
                        from: CGRect(x: 0, y: 0, width: density.width, height: density.height(providerCount: count)), edge: edge)
                    try save(await render(folded, width: frame.width, height: frame.height),
                             name: "folded-\(density.rawValue)-\(edge == .left ? "left" : "right")")
                }
            }
        }
    }

    private func render<V: View>(_ view: V, width: Double, height: Double) async throws -> NSBitmapImageRep {
        let host = NSHostingView(rootView: view.frame(width: width, height: height)
            .environment(\.colorScheme, .dark))
        let frame = NSRect(x: 0, y: 0, width: width, height: height)
        let window = NSWindow(contentRect: frame, styleMask: .borderless, backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.close() }
        host.frame = frame
        window.contentView = host
        window.layoutIfNeeded()
        host.layoutSubtreeIfNeeded()
        try await Task.sleep(for: .milliseconds(25))
        host.layoutSubtreeIfNeeded()
        let bitmap = try #require(NSBitmapImageRep(bitmapDataPlanes: nil,
            pixelsWide: Int(width), pixelsHigh: Int(height), bitsPerSample: 8,
            samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
            colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0))
        bitmap.size = NSSize(width: width, height: height)
        host.cacheDisplay(in: host.bounds, to: bitmap)
        return bitmap
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
