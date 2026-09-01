import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("Claude detail presentation")
struct ClaudeDetailPresentationTests {
    @Test("Zero local activity uses an explicit empty state")
    func zeroActivityIsExplicit() {
        let summary = makeSummary(days: [
            ClaudeDailyActivity(date: .distantPast, inputTokens: 0, outputTokens: 0, cacheTokens: 0),
        ])

        #expect(!ClaudeDetailPresentation.hasDailyActivity(summary))
        #expect(ClaudeDetailPresentation.localActivityEmptyTitle == "No local Claude Code activity")
    }

    @Test("Top model rows include stable percentage shares")
    func topModelsIncludeShares() {
        let summary = makeSummary(
            days: [ClaudeDailyActivity(date: .distantPast, inputTokens: 100, outputTokens: 0, cacheTokens: 0)],
            models: [
                ClaudeModelActivity(modelID: "claude-sonnet-5", tokenCount: 75),
                ClaudeModelActivity(modelID: "claude-haiku-4-5", tokenCount: 25),
            ]
        )

        let rows = ClaudeDetailPresentation.topModelRows(summary)

        #expect(rows.map(\.sharePercent) == [75, 25])
    }

    @Test("Accessibility labels identify official and local estimates")
    func accessibilityLabelsExposeDataScope() {
        let metric = UsageMetric(
            label: "Current session",
            current: 23,
            limit: 100,
            unit: .percent
        )

        #expect(
            ClaudeDetailPresentation.officialQuotaAccessibilityLabel(metric)
                == "Official quota, Current session, 23 percent used"
        )
        #expect(
            ClaudeDetailPresentation.localStatAccessibilityLabel(title: "Sessions", value: "7")
                == "Local estimate, Sessions, 7"
        )
    }

    @Test("Only the local activity region is inside the scroll view")
    func pinnedOfficialQuotaSourceContract() throws {
        let projectRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let source = try String(contentsOf: projectRoot.appending(
            path: "Sources/AIMeterApp/Views/ClaudeDetailView.swift"
        ))
        let scroll = try #require(source.range(of: "ScrollView {"))
        let header = try #require(source.range(of: "header\n"))
        let official = try #require(source.range(of: "officialQuotaSection\n"))
        let local = try #require(source.range(of: "localActivitySection\n"))

        #expect(header.lowerBound < scroll.lowerBound)
        #expect(official.lowerBound < scroll.lowerBound)
        #expect(local.lowerBound > scroll.lowerBound)
    }

    private func makeSummary(
        days: [ClaudeDailyActivity],
        models: [ClaudeModelActivity] = []
    ) -> ClaudeLocalActivitySummary {
        ClaudeLocalActivitySummary(
            days: days,
            sessionCount: 0,
            activeDayCount: 0,
            models: models,
            updatedAt: .distantPast
        )
    }
}
