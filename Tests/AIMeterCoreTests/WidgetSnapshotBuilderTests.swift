import Foundation
import Testing
@testable import AIMeterCore

@Suite("Widget snapshot builder")
struct WidgetSnapshotBuilderTests {
    @Test("Keeps provider order and the same summary semantics as the app")
    func keepsProviderOrderAndPresentationSemantics() {
        let snapshots = [
            UsageSnapshot(
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
            ),
            UsageSnapshot(
                provider: .codex,
                primaryMetric: UsageMetric(label: "Session", current: 5, limit: 100, unit: .percent),
                secondaryMetric: UsageMetric(label: "Weekly", current: 31, limit: 100, unit: .percent)
            ),
            UsageSnapshot(
                provider: .claude,
                primaryMetric: UsageMetric(label: "Session", current: 18, limit: 100, unit: .percent)
            ),
        ]

        let envelope = WidgetSnapshotBuilder().build(
            snapshots: snapshots,
            generatedAt: Date(timeIntervalSince1970: 1_000)
        )

        #expect(envelope.version == 1)
        #expect(envelope.generatedAt == Date(timeIntervalSince1970: 1_000))
        #expect(envelope.providers.map(\.provider) == [.claude, .codex, .deepSeek])
        #expect(envelope.providers.map(\.valueText) == ["18%", "31%", "¥77.99"])
        #expect(envelope.providers.map(\.detailText) == ["Session", "Weekly", "Available balance"])
        #expect(envelope.providers[0].fraction == 0.18)
        #expect(envelope.providers[1].fraction == 0.31)
        #expect(abs((envelope.providers[2].fraction ?? 0) - 0.2201) < 0.0001)
    }

    @Test("Fills missing providers without losing available peers")
    func fillsMissingProviders() {
        let envelope = WidgetSnapshotBuilder().build(
            snapshots: [UsageSnapshot(
                provider: .codex,
                primaryMetric: UsageMetric(label: "Weekly", current: 9, limit: 100, unit: .percent)
            )],
            generatedAt: Date(timeIntervalSince1970: 100)
        )

        #expect(envelope.providers.map(\.provider) == [.claude, .codex, .deepSeek])
        #expect(envelope.providers[0].semantic == .unavailable)
        #expect(envelope.providers[0].valueText == "Unavailable")
        #expect(envelope.providers[1].valueText == "9%")
        #expect(envelope.providers[2].semantic == .unavailable)
    }

    @Test("Chooses the earliest future dated reset across Claude and Codex")
    func choosesEarliestFutureReset() {
        let now = Date(timeIntervalSince1970: 10_000)
        let envelope = WidgetSnapshotBuilder().build(
            snapshots: [
                UsageSnapshot(
                    provider: .claude,
                    primaryMetric: UsageMetric(
                        label: "Session",
                        current: 18,
                        limit: 100,
                        unit: .percent,
                        resetAt: now.addingTimeInterval(7_200),
                        resetDescription: "Resets later"
                    )
                ),
                UsageSnapshot(
                    provider: .codex,
                    primaryMetric: UsageMetric(
                        label: "5h limit",
                        current: 31,
                        limit: 100,
                        unit: .percent,
                        resetAt: now.addingTimeInterval(3_600),
                        resetDescription: "Resets first"
                    ),
                    secondaryMetric: UsageMetric(
                        label: "Expired",
                        current: 2,
                        limit: 100,
                        unit: .percent,
                        resetAt: now.addingTimeInterval(-60),
                        resetDescription: "Already reset"
                    )
                ),
            ],
            generatedAt: now
        )

        #expect(envelope.nextReset?.provider == .codex)
        #expect(envelope.nextReset?.label == "5h limit")
        #expect(envelope.nextReset?.text == "Resets first")
        #expect(envelope.nextReset?.resetAt == now.addingTimeInterval(3_600))
    }

    @Test("Falls back to sanitized textual reset guidance when no date is available")
    func fallsBackToTextualResetGuidance() {
        let secret = "sk-widget-reset-secret-123456"
        let envelope = WidgetSnapshotBuilder().build(
            snapshots: [
                UsageSnapshot(
                    provider: .codex,
                    primaryMetric: UsageMetric(
                        label: "Weekly",
                        current: 4,
                        limit: 100,
                        unit: .percent,
                        resetDescription: "Codex reset"
                    )
                ),
                UsageSnapshot(
                    provider: .claude,
                    primaryMetric: UsageMetric(
                        label: "Session",
                        current: 8,
                        limit: 100,
                        unit: .percent,
                        resetDescription: "Bearer \(secret) resets in 2h"
                    )
                ),
            ],
            generatedAt: Date(timeIntervalSince1970: 100)
        )

        #expect(envelope.nextReset?.provider == .claude)
        #expect(envelope.nextReset?.resetAt == nil)
        #expect(envelope.nextReset?.text == "Bearer [REDACTED] resets in 2h")
    }

    @Test("Keeps only the Codex reset credit count and nearest expiration")
    func minimizesResetCreditData() {
        let now = Date(timeIntervalSince1970: 50_000)
        let envelope = WidgetSnapshotBuilder().build(
            snapshots: [UsageSnapshot(
                provider: .codex,
                codexResetCredits: CodexResetCreditsSummary(
                    availableCount: 3,
                    credits: [
                        CodexResetCreditDisplay(title: "Later", expiresAt: now.addingTimeInterval(8_000)),
                        CodexResetCreditDisplay(title: "Sooner", expiresAt: now.addingTimeInterval(2_000)),
                        CodexResetCreditDisplay(title: "Unknown", expiresAt: nil),
                    ],
                    hasCompleteDetails: true
                )
            )],
            generatedAt: now
        )

        #expect(envelope.codexResetCredits?.availableCount == 3)
        #expect(envelope.codexResetCredits?.nearestExpiration == now.addingTimeInterval(2_000))
    }

    @Test("Maps fresh cached expired and unavailable states honestly")
    func mapsAvailabilityAndFreshness() {
        let now = Date(timeIntervalSince1970: 10_000)
        let snapshots = [
            UsageSnapshot(
                provider: .claude,
                primaryMetric: UsageMetric(label: "Session", current: 95, limit: 100, unit: .percent),
                fetchedAt: now,
                staleAfter: 300
            ),
            UsageSnapshot(
                provider: .codex,
                primaryMetric: UsageMetric(label: "Weekly", current: 40, limit: 100, unit: .percent),
                fetchedAt: now,
                staleAfter: 300,
                collectionStatus: .cached
            ),
            UsageSnapshot(
                provider: .deepSeek,
                availability: .unavailable,
                fetchedAt: now.addingTimeInterval(-600),
                staleAfter: 300,
                collectionStatus: .authenticationRequired
            ),
        ]

        let envelope = WidgetSnapshotBuilder().build(snapshots: snapshots, generatedAt: now)

        #expect(envelope.providers[0].semantic == .critical)
        #expect(envelope.providers[0].expiresAt == now.addingTimeInterval(300))
        #expect(envelope.providers[1].semantic == .stale)
        #expect(envelope.providers[2].semantic == .unavailable)
    }
}
