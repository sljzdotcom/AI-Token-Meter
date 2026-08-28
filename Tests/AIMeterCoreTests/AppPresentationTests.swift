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
