import Foundation
import Testing
@testable import AIMeterCore

@Suite("App presentation model")
struct AppPresentationTests {
    @Test("Formats bounded usage and assigns warning semantics")
    func formatsPercentageUsage() {
        let snapshot = UsageSnapshot(
            provider: .claude,
            primaryMetric: UsageMetric(
                label: "Current session",
                current: 73,
                limit: 100,
                unit: .percent
            )
        )

        let presentation = ProviderPresentation(snapshot: snapshot)

        #expect(presentation.title == "Claude")
        #expect(presentation.valueText == "73%")
        #expect(presentation.detailText == "Current session")
        #expect(presentation.semantic == .warning)
    }

    @Test("Formats balances without inventing a percentage")
    func formatsBalance() {
        let snapshot = UsageSnapshot(
            provider: .deepSeek,
            primaryMetric: UsageMetric(
                label: "Available balance",
                current: 61,
                limit: nil,
                unit: .cny,
                kind: .balance
            )
        )

        let presentation = ProviderPresentation(snapshot: snapshot)

        #expect(presentation.valueText == "¥61.00")
        #expect(presentation.fraction == nil)
        #expect(presentation.semantic == .normal)
    }

    @Test("Uses the same highest window for summary text and ring")
    func summarizesHighestWindowConsistently() {
        let snapshot = UsageSnapshot(
            provider: .claude,
            primaryMetric: UsageMetric(
                label: "Current session",
                current: 0,
                limit: 100,
                unit: .percent
            ),
            secondaryMetric: UsageMetric(
                label: "Weekly limit",
                current: 10,
                limit: 100,
                unit: .percent
            )
        )

        let presentation = ProviderPresentation(snapshot: snapshot)

        #expect(presentation.valueText == "10%")
        #expect(presentation.fraction == 0.10)
        #expect(presentation.ringFraction == 0.10)
    }

    @Test("Zero and unbounded metrics do not draw progress")
    func hidesFalseProgress() {
        let zero = ProviderPresentation(snapshot: usageSnapshot(provider: .codex, fraction: 0))
        let balance = ProviderPresentation(snapshot: UsageSnapshot(
            provider: .deepSeek,
            primaryMetric: UsageMetric(
                label: "Balance",
                current: 77.99,
                limit: nil,
                unit: .cny,
                kind: .balance
            )
        ))

        #expect(zero.ringFraction == nil)
        #expect(balance.ringFraction == nil)
    }

    @Test("DeepSeek ring reports balance depletion against its baseline")
    func deepSeekBalanceDepletion() {
        let snapshot = UsageSnapshot(
            provider: .deepSeek,
            primaryMetric: UsageMetric(
                label: "Available balance",
                current: 77.99,
                limit: nil,
                unit: .cny,
                kind: .balance
            ),
            secondaryMetric: UsageMetric(
                label: "Balance baseline",
                current: 22.01,
                limit: 100,
                unit: .cny,
                kind: .localBudget
            )
        )

        let presentation = ProviderPresentation(snapshot: snapshot)

        #expect(presentation.valueText == "¥77.99")
        #expect(abs((presentation.ringFraction ?? 0) - 0.2201) < 0.0001)
        #expect(presentation.semantic == .normal)
    }

    @Test("Turns account and cached states into actionable copy")
    func formatsNonFreshStates() {
        let authentication = ProviderPresentation(snapshot: UsageSnapshot(
            provider: .claude,
            availability: .unavailable,
            collectionStatus: .authenticationRequired,
            statusMessage: "Sign in required"
        ))
        let cached = ProviderPresentation(snapshot: UsageSnapshot(
            provider: .codex,
            primaryMetric: UsageMetric(label: "5h limit", current: 40, limit: 100, unit: .percent),
            collectionStatus: .cached,
            statusMessage: "Request timed out"
        ))

        #expect(authentication.valueText == "Sign in")
        #expect(authentication.semantic == .unavailable)
        #expect(cached.valueText == "40%")
        #expect(cached.semantic == .stale)
        #expect(cached.statusText == "Request timed out")
        #expect(cached.accessibilityStatusText == "Cached data. Request timed out")
        #expect(authentication.accessibilityStatusText == "Unavailable. Sign in required")
    }

    @Test("Formats Claude workspace setup as a one-time action")
    func formatsWorkspaceSetup() {
        let presentation = ProviderPresentation(snapshot: UsageSnapshot(
            provider: .claude,
            availability: .unavailable,
            collectionStatus: .setupRequired,
            statusMessage: "Approve the private usage workspace once"
        ))

        #expect(presentation.valueText == "Set up")
        #expect(presentation.detailText == "One-time Claude workspace approval")
        #expect(presentation.statusText == "Approve the private usage workspace once")
        #expect(presentation.semantic == .unavailable)
    }

    @Test("Menu bar summary reflects the highest bounded risk")
    func summarizesHighestRisk() {
        let snapshots = [
            usageSnapshot(provider: .claude, fraction: 0.22),
            usageSnapshot(provider: .codex, fraction: 0.92),
            usageSnapshot(provider: .deepSeek, fraction: 0.75),
        ]

        let summary = MenuBarSummary(snapshots: snapshots)

        #expect(summary.semantic == .critical)
        #expect(summary.valueText == "92%")
        #expect(summary.accessibilityLabel == "AI Meter, highest usage 92 percent")
    }

    @Test("Carries reset guidance for both quota windows")
    func formatsResetGuidance() {
        let snapshot = UsageSnapshot(
            provider: .claude,
            primaryMetric: UsageMetric(
                label: "Current session",
                current: 73,
                limit: 100,
                unit: .percent,
                resetDescription: "Resets in 51 min"
            ),
            secondaryMetric: UsageMetric(
                label: "All models",
                current: 7,
                limit: 100,
                unit: .percent,
                resetDescription: "Resets at midnight"
            )
        )

        let presentation = ProviderPresentation(snapshot: snapshot)

        #expect(presentation.primaryResetText == "Resets in 51 min")
        #expect(presentation.secondaryResetText == "Resets at midnight")
    }

    @Test("An officially unavailable account never appears healthy")
    func formatsUnavailableAccount() {
        let snapshot = UsageSnapshot(
            provider: .deepSeek,
            primaryMetric: UsageMetric(
                label: "Available balance",
                current: 0,
                limit: nil,
                unit: .cny,
                kind: .balance
            ),
            availability: .unavailable,
            collectionStatus: .fresh
        )

        let presentation = ProviderPresentation(snapshot: snapshot)

        #expect(presentation.semantic == .unavailable)
        #expect(presentation.statusText == "Account unavailable")
    }

    @Test("Formats the three Codex local activity values compactly")
    func formatsCodexLocalActivity() {
        let presentation = CodexLocalActivityPresentation(summary: CodexLocalActivitySummary(
            tokenCount: 31_400_000_000,
            currentStreakDays: 54,
            longestSessionDuration: 6_720
        ))

        #expect(presentation.tokenText == "31.4B")
        #expect(presentation.streakText == "54 days")
        #expect(presentation.longestSessionText == "1h 52m")
    }

    @Test("Orders reset credits and explains expiration by local calendar day")
    func presentsResetCredits() {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(secondsFromGMT: 0)!
        let now = Date(timeIntervalSince1970: 1_788_048_000)
        let summary = CodexResetCreditsSummary(
            availableCount: 4,
            credits: [
                CodexResetCreditDisplay(title: "Unknown", expiresAt: nil),
                CodexResetCreditDisplay(title: "Later", expiresAt: now.addingTimeInterval(3 * 86_400)),
                CodexResetCreditDisplay(title: "Today", expiresAt: now.addingTimeInterval(3_600)),
                CodexResetCreditDisplay(title: "Expired", expiresAt: now.addingTimeInterval(-86_400)),
            ],
            hasCompleteDetails: true
        )

        let presentation = CodexResetCreditsPresentation(
            summary: summary,
            now: now,
            calendar: calendar
        )

        #expect(presentation.availableText == "4 available")
        #expect(presentation.rows.map(\.title) == ["Expired", "Today", "Later", "Unknown"])
        #expect(presentation.rows.map(\.statusText) == [
            "Expired",
            "Expires today",
            "3 days remaining",
            "Expiration unavailable",
        ])
    }

    @Test("Marks incomplete reset credit details without inventing rows")
    func identifiesIncompleteResetCredits() {
        let summary = CodexResetCreditsSummary(
            availableCount: 2,
            credits: [CodexResetCreditDisplay(title: nil, expiresAt: nil)],
            hasCompleteDetails: false
        )

        let presentation = CodexResetCreditsPresentation(summary: summary)

        #expect(presentation.rows.count == 1)
        #expect(presentation.rows[0].title == "Usage reset")
        #expect(presentation.showsIncompleteDetails)
    }

    private func usageSnapshot(provider: UsageProvider, fraction: Double) -> UsageSnapshot {
        UsageSnapshot(
            provider: provider,
            primaryMetric: UsageMetric(
                label: "Usage",
                current: fraction * 100,
                limit: 100,
                unit: .percent
            )
        )
    }
}
