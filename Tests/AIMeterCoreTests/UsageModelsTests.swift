import Foundation
import Testing
@testable import AIMeterCore

@Suite("Usage domain models")
struct UsageModelsTests {
    @Test("A bounded metric reports a clamped used fraction")
    func boundedMetricReportsUsedFraction() {
        let metric = UsageMetric(
            label: "Current session",
            current: 73,
            limit: 100,
            unit: .percent
        )

        #expect(metric.usedFraction == 0.73)

        let overLimit = UsageMetric(
            label: "Current session",
            current: 120,
            limit: 100,
            unit: .percent
        )
        #expect(overLimit.usedFraction == 1)
    }

    @Test("A metric without a positive limit does not invent a percentage")
    func unboundedMetricHasNoUsedFraction() {
        let missingLimit = UsageMetric(
            label: "Balance",
            current: 42.5,
            limit: nil,
            unit: .cny
        )
        let zeroLimit = UsageMetric(
            label: "Balance",
            current: 42.5,
            limit: 0,
            unit: .cny
        )

        #expect(missingLimit.usedFraction == nil)
        #expect(zeroLimit.usedFraction == nil)
    }

    @Test("A snapshot becomes stale after its freshness interval")
    func snapshotDetectsStaleData() {
        let snapshot = UsageSnapshot(
            provider: .claude,
            primaryMetric: UsageMetric(
                label: "Current session",
                current: 73,
                limit: 100,
                unit: .percent
            ),
            fetchedAt: Date(timeIntervalSince1970: 100),
            staleAfter: 300
        )

        #expect(snapshot.isStale(at: Date(timeIntervalSince1970: 399)) == false)
        #expect(snapshot.isStale(at: Date(timeIntervalSince1970: 401)))
    }
}
