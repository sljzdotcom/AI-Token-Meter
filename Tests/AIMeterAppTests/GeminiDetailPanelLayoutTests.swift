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

    @Test func detailCardsShowRemainingWhileProgressUsesConsumedQuota() throws {
        let projectRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let source = try String(
            contentsOf: projectRoot.appendingPathComponent("Sources/AIMeterApp/Views/GeminiDetailView.swift"),
            encoding: .utf8
        )
        #expect(source.contains("% remaining"))
        #expect(source.contains("ProgressView(value: metric.current, total: 100)"))
        #expect(!source.contains("% used"))
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
}
