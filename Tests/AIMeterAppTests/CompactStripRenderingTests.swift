import AIMeterCore
import AppKit
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("Compact strip rendering")
struct CompactStripRenderingTests {
    @Test @MainActor func rendersBothDensitiesAndEdges() throws {
        for density in FloatingStripDensity.allCases {
            for edge in [FloatingStripEdge.left, .right] {
                let view = ZStack {
                    FloatingStripSurface(edge: edge, density: density, providerCount: 3)
                    VStack(spacing: density.spacing) {
                        ForEach([UsageProvider.claude, .codex, .deepSeek], id: \.self) { provider in
                            UsageRing(presentation: ProviderPresentation(snapshot: UsageSnapshot(
                                provider: provider,
                                primaryMetric: UsageMetric(label: "Usage", current: 25, limit: 100, unit: .percent),
                                collectionStatus: .fresh
                            )), size: density.ringSize)
                        }
                    }
                }.frame(width: density.width, height: density.baseHeight)
                    .environment(\.colorScheme, .dark)
                let renderer = ImageRenderer(content: view)
                renderer.scale = 2
                let image = try #require(renderer.nsImage)
                #expect(abs(image.size.width - density.width) < 0.01)
                #expect(abs(image.size.height - density.baseHeight) < 0.01)
                let tiff = try #require(image.tiffRepresentation)
                let bitmap = try #require(NSBitmapImageRep(data: tiff))
                // Transparent corners and opaque body validate a single surface mask.
                #expect(try #require(bitmap.colorAt(x: 0, y: 0)).alphaComponent < 0.01)
                #expect(try #require(bitmap.colorAt(x: Int(density.width), y: Int(density.baseHeight))).alphaComponent > 0.99)
                if let target = ProcessInfo.processInfo.environment["AI_METER_DOC_SCREENSHOT_DIR"] {
                    let url = URL(fileURLWithPath: target, isDirectory: true)
                    try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
                    let png = try #require(bitmap.representation(using: .png, properties: [:]))
                    try png.write(to: url.appendingPathComponent("strip-\(density.rawValue)-\(edge == .left ? "left" : "right").png"))
                }
            }
        }
    }
}
