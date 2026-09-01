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

    @Test("Accessibility labels identify official and local estimates")
    func accessibilityLabelsExposeDataScope() {
        let metric = UsageMetric(
            label: "Current session",
            current: 23,
            limit: 100,
            unit: .percent
        )

        #expect(
            ClaudeDetailPresentation.officialQuotaAccessibilityLabel(
                metric,
                resetText: "Resets in 3 hours"
            ) == "Official quota, Current session, 23 percent used, Resets in 3 hours"
        )
        #expect(
            ClaudeDetailPresentation.localStatAccessibilityLabel(title: "Sessions", value: "7")
                == "Local estimate, Sessions, 7"
        )
        #expect(
            ClaudeDetailPresentation.localActivityAccessibilityLabel(
                title: "Daily token activity",
                detail: "100 total tokens"
            ) == "Local estimate, Daily token activity, 100 total tokens"
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
        #expect(source.contains("officialQuotaAccessibilityLabel(metric, resetText: resetText)"))
        #expect(source.contains("localActivityAccessibilityLabel("))
    }

    @Test("Claude detail omits composition and model cards while retaining essentials")
    func removedCardsStayAbsent() throws {
        let projectRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let source = try String(contentsOf: projectRoot.appending(
            path: "Sources/AIMeterApp/Views/ClaudeDetailView.swift"
        ))

        #expect(!source.contains("tokenComposition(summary)"))
        #expect(!source.contains("modelBreakdown("))
        #expect(!source.contains("\"Token composition\""))
        #expect(!source.contains("\"Top models\""))
        #expect(source.contains("localStat(title: \"Sessions\""))
        #expect(source.contains("activityChart(summary)"))
        #expect(source.contains("Conversation content stays private."))
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
