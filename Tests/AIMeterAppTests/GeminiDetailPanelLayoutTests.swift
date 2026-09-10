import AIMeterCore
import AppKit
import Foundation
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("Gemini detail panel layout")
struct GeminiDetailPanelLayoutTests {
    @Test func allFourWindowsFitAndSmallScreensRemainScrollable() {
        #expect(GeminiDetailPanelLayout.height(tierCount: 4, availableHeight: 900) >= 530)
        #expect(GeminiDetailPanelLayout.height(tierCount: 4, availableHeight: 360) == 344)
        #expect(GeminiDetailPanelLayout.height(tierCount: 0, availableHeight: 900) == 280)
    }

    @Test func detailCardsShowRemainingWhileProgressUsesConsumedQuota() {
        let metric = UsageMetric(
            label: "Gemini · Five hour",
            current: 20,
            limit: 100,
            unit: .percent
        )

        #expect(GeminiDetailPresentation.remainingText(for: metric) == "80% remaining")
        #expect(metric.usedFraction == 0.2)
    }

    @Test("Antigravity detail keeps an opaque themed surface in every data state")
    @MainActor
    func detailSurfaceIsOpaqueAcrossStates() throws {
        for status in [CollectionStatus.fresh, .cached, .unavailable] {
            let snapshot = Self.snapshot(status: status)
            let view = GeminiDetailView(snapshot: snapshot, onRetry: {})
                .frame(width: 390, height: 520)
                .environment(\.colorScheme, .dark)
            let renderer = ImageRenderer(content: view)
            renderer.scale = 2

            let image = try #require(renderer.nsImage)
            let tiff = try #require(image.tiffRepresentation)
            let bitmap = try #require(NSBitmapImageRep(data: tiff))
            let scaleX = CGFloat(bitmap.pixelsWide) / image.size.width
            let scaleY = CGFloat(bitmap.pixelsHigh) / image.size.height
            func color(at point: CGPoint) throws -> NSColor {
                try #require(bitmap.colorAt(
                    x: Int((point.x * scaleX).rounded()),
                    y: Int((point.y * scaleY).rounded())
                ))
            }

            #expect(try color(at: CGPoint(x: 2, y: 2)).alphaComponent < 0.5)
            #expect(try color(at: CGPoint(x: 10, y: 260)).alphaComponent > 0.9)
            #expect(try color(at: CGPoint(x: 195, y: 500)).alphaComponent > 0.9)
        }
    }

    @Test("Approved Antigravity accent remains visible in every detail state")
    @MainActor
    func approvedAccentIsVisibleAcrossDetailStates() async throws {
        for status in [CollectionStatus.fresh, .cached, .unavailable] {
            let snapshot = Self.snapshot(status: status)
            let view = GeminiDetailView(snapshot: snapshot, onRetry: {})
            let bitmap = try await Self.render(view, width: 390, height: 520)
            try Self.save(bitmap, name: "antigravity-accent-\(status)")
            var accentPixels = 0
            for x in stride(from: 0, to: bitmap.pixelsWide, by: 2) {
                for y in stride(from: 0, to: bitmap.pixelsHigh, by: 2) {
                    guard let color = bitmap.colorAt(x: x, y: y)?.usingColorSpace(.sRGB) else { continue }
                    if abs(color.redComponent - 62.0 / 255.0) < 0.08,
                       abs(color.greenComponent - 214.0 / 255.0) < 0.08,
                       abs(color.blueComponent - 178.0 / 255.0) < 0.08,
                       color.alphaComponent > 0.6 {
                        accentPixels += 1
                    }
                }
            }

            #expect(accentPixels > 20, "\(status) rendered \(accentPixels) approved accent pixels")
        }
    }

    private static func snapshot(status: CollectionStatus) -> UsageSnapshot {
        let metrics = [
            UsageMetric(
                label: "Gemini · Five hour",
                current: 20,
                limit: 100,
                unit: .percent,
                resetDescription: "Resets in 2h"
            ),
            UsageMetric(
                label: "Gemini · Weekly",
                current: 40,
                limit: 100,
                unit: .percent,
                resetDescription: "Resets Friday"
            ),
        ]
        let retained = status == .unavailable ? nil : metrics
        return UsageSnapshot(
            provider: .gemini,
            primaryMetric: retained?.first,
            availability: status == .unavailable ? .unavailable : .available,
            fetchedAt: Date(timeIntervalSince1970: 1_788_316_200),
            sourceVersion: "1.1.28",
            collectionStatus: status,
            statusMessage: status == .unavailable ? "Quota unavailable" : nil,
            geminiQuotaMetrics: retained
        )
    }

    private static func save(_ bitmap: NSBitmapImageRep, name: String) throws {
        guard let path = ProcessInfo.processInfo.environment["AI_METER_DOC_SCREENSHOT_DIR"] else { return }
        let directory = URL(fileURLWithPath: path, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let png = try #require(bitmap.representation(using: .png, properties: [:]))
        try png.write(to: directory.appendingPathComponent("\(name).png"))
    }

    @MainActor
    private static func render<V: View>(
        _ view: V,
        width: Double,
        height: Double
    ) async throws -> NSBitmapImageRep {
        let host = NSHostingView(rootView: view
            .frame(width: width, height: height, alignment: .topLeading)
            .environment(\.colorScheme, .dark))
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
}
