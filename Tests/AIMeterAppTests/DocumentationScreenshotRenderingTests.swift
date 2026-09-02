import AIMeterCore
import AppKit
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("Documentation screenshot rendering")
struct DocumentationScreenshotRenderingTests {
    @Test("Renders a deterministic public Codex detail image")
    @MainActor
    func rendersPublicCodexDetail() throws {
        let view = CodexDetailView(snapshot: Self.codexSnapshot)
            .frame(width: 420, height: 560)
            .environment(\.colorScheme, .dark)
            .aiMeterFontScope(.content(.system))
        let renderer = ImageRenderer(content: view)
        renderer.scale = 2

        let image = try #require(renderer.nsImage)
        let tiff = try #require(image.tiffRepresentation)
        let bitmap = try #require(NSBitmapImageRep(data: tiff))
        let png = try #require(bitmap.representation(
            using: NSBitmapImageRep.FileType.png,
            properties: [:]
        ))

        #expect(png.count > 20_000)
        #expect(image.size.width == 420)
        #expect(image.size.height == 560)

        if let requestedDirectory = ProcessInfo.processInfo.environment["AI_METER_DOC_SCREENSHOT_DIR"] {
            let directory = URL(fileURLWithPath: requestedDirectory, isDirectory: true)
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            try png.write(
                to: directory.appendingPathComponent("provider-detail.png"),
                options: Data.WritingOptions.atomic
            )
        }
    }

    private static var codexSnapshot: UsageSnapshot {
        let reference = Date(timeIntervalSince1970: 1_788_316_200)
        return UsageSnapshot(
            provider: .codex,
            primaryMetric: UsageMetric(
                label: "Session",
                current: 23,
                limit: 100,
                unit: .percent,
                resetDescription: "Resets in 3h 42m"
            ),
            secondaryMetric: UsageMetric(
                label: "Weekly",
                current: 5,
                limit: 100,
                unit: .percent,
                resetDescription: "Resets Sep 6 · 08:00"
            ),
            availability: .available,
            fetchedAt: reference,
            staleAfter: 300,
            collectionStatus: .fresh,
            codexResetCredits: CodexResetCreditsSummary(
                availableCount: 1,
                credits: [
                    CodexResetCreditDisplay(
                        title: "Full usage reset",
                        expiresAt: reference.addingTimeInterval(21 * 86_400)
                    ),
                ],
                hasCompleteDetails: true
            ),
            codexLocalActivity: CodexLocalActivitySummary(
                tokenCount: 31_400_000_000,
                currentStreakDays: 54,
                longestSessionDuration: 6_720
            )
        )
    }
}
