import Foundation
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
}
