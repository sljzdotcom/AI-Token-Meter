import Foundation
import Testing
@testable import AIMeterCore

@Suite("Usage threshold evaluator")
struct ThresholdEvaluatorTests {
    @Test("Emits warning at 70 percent and critical at 90 percent")
    func emitsThresholdCrossings() {
        var evaluator = ThresholdEvaluator()

        #expect(evaluator.evaluate(snapshot(0.69)).isEmpty)
        #expect(evaluator.evaluate(snapshot(0.70)).map(\.level) == [.warning])
        #expect(evaluator.evaluate(snapshot(0.89)).isEmpty)
        #expect(evaluator.evaluate(snapshot(0.90)).map(\.level) == [.critical])
    }

    @Test("Crossing both levels in one refresh emits only the critical event")
    func emitsOnlyHighestCrossedLevel() {
        var evaluator = ThresholdEvaluator()

        #expect(evaluator.evaluate(snapshot(0.65)).isEmpty)
        let events = evaluator.evaluate(snapshot(0.93))

        #expect(events.count == 1)
        #expect(events.first?.level == .critical)
    }

    @Test("Duplicate refreshes do not repeat a notification")
    func deduplicatesRefreshes() {
        var evaluator = ThresholdEvaluator()

        #expect(evaluator.evaluate(snapshot(0.74)).count == 1)
        #expect(evaluator.evaluate(snapshot(0.76)).isEmpty)
        #expect(evaluator.evaluate(snapshot(0.76)).isEmpty)
    }

    @Test("A changed reset cycle allows thresholds to notify again")
    func resetsWhenCycleChanges() {
        var evaluator = ThresholdEvaluator()
        let firstReset = Date(timeIntervalSince1970: 1_000)
        let secondReset = Date(timeIntervalSince1970: 2_000)

        #expect(evaluator.evaluate(snapshot(0.75, resetAt: firstReset)).map(\.level) == [.warning])
        #expect(evaluator.evaluate(snapshot(0.76, resetAt: firstReset)).isEmpty)
        #expect(evaluator.evaluate(snapshot(0.75, resetAt: secondReset)).map(\.level) == [.warning])
    }

    @Test("Dropping below ten percent re-arms the warning threshold")
    func rearmsAfterUsageDrops() {
        var evaluator = ThresholdEvaluator()

        #expect(evaluator.evaluate(snapshot(0.75)).map(\.level) == [.warning])
        #expect(evaluator.evaluate(snapshot(0.05)).isEmpty)
        #expect(evaluator.evaluate(snapshot(0.72)).map(\.level) == [.warning])
    }

    @Test("A changing countdown description does not look like a new quota cycle")
    func ignoresChangingCountdownText() {
        var evaluator = ThresholdEvaluator()

        #expect(
            evaluator.evaluate(snapshot(0.75, resetDescription: "Resets in 51 min")).map(\.level)
                == [.warning]
        )
        #expect(evaluator.evaluate(snapshot(0.76, resetDescription: "Resets in 50 min")).isEmpty)
    }

    @Test("Balance-only metrics never trigger usage alerts")
    func ignoresUnboundedBalances() {
        var evaluator = ThresholdEvaluator()
        let balance = UsageMetric(
            label: "Available balance",
            current: 2,
            limit: nil,
            unit: .cny,
            kind: .balance
        )
        let snapshot = UsageSnapshot(provider: .deepSeek, primaryMetric: balance)

        #expect(evaluator.evaluate(snapshot).isEmpty)
    }

    private func snapshot(
        _ fraction: Double,
        resetAt: Date? = nil,
        resetDescription: String? = nil
    ) -> UsageSnapshot {
        UsageSnapshot(
            provider: .claude,
            primaryMetric: UsageMetric(
                label: "Current session",
                current: fraction * 100,
                limit: 100,
                unit: .percent,
                resetAt: resetAt,
                resetDescription: resetDescription
            )
        )
    }
}
