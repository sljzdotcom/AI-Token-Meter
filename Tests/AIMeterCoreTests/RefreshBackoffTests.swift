import Foundation
import Testing
@testable import AIMeterCore

struct RefreshBackoffTests {
    @Test func matchesSharedBehaviorContract() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let data = try Data(contentsOf: root.appendingPathComponent("contracts/fixtures/auxiliary/strip-behavior.json"))
        let json = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])
        let densities = try #require(json["densities"] as? [[String: Any]])
        for row in densities {
            let id = try #require(row["id"] as? String)
            let density = try #require(FloatingStripDensity(rawValue: id))
            #expect(density.width == (row["width"] as? NSNumber)?.doubleValue)
            #expect(density.baseHeight == (row["height"] as? NSNumber)?.doubleValue)
        }
        let contour = try #require(json["expandedContour"] as? [String: Any])
        #expect((contour["referenceWidth"] as? NSNumber)?.doubleValue == 65)
        #expect((contour["compactShoulderDepth"] as? NSNumber)?.doubleValue == 70)
        #expect((contour["comfortableShoulderDepth"] as? NSNumber)?.doubleValue == 88)
        #expect((contour["topStart"] as? [NSNumber])?.map(\.doubleValue) == [65, 4])
        #expect((contour["topCurves"] as? [[String: Any]])?.count == 3)
        let waits = try #require(json["backoff"] as? [[String: Any]])
        for row in waits {
            let id = try #require(row["kind"] as? String)
            let kind = try #require(RefreshFailureKind(rawValue: id))
            var state = RefreshBackoffState()
            state.record(kind, now: 100)
            #expect(state.nextEligibleAt - 100 == (row["first"] as? NSNumber)?.doubleValue)
            for _ in 0..<10 { state.record(kind, now: 100) }
            #expect(state.nextEligibleAt - 100 == (row["cap"] as? NSNumber)?.doubleValue)
        }
    }
    @Test func freshnessAndOperationAreIndependentOfQuota() {
        let snapshot = UsageSnapshot(provider: .codex, fetchedAt: Date(timeIntervalSince1970: 100),
                                     collectionStatus: .cached)
        #expect(ProviderDataState.freshness(snapshot, now: Date(timeIntervalSince1970: 280)) == "Cached · 3 min ago")
        #expect(ProviderOperationState.resolve(status: .cached, refreshing: true, needsAction: false) == .refreshing)
        #expect(ProviderOperationState.resolve(status: .authenticationRequired, refreshing: false, needsAction: false) == .waiting)
    }
    @Test func rateLimitSurvivesRestartAndManualRefresh() throws {
        var state = RefreshBackoffState()
        state.record(.rateLimited, now: 100, retryAfter: 90)
        let restored = try JSONDecoder().decode(RefreshBackoffState.self, from: JSONEncoder().encode(state))
        #expect(!restored.isEligible(now: 189, manual: true))
        #expect(restored.isEligible(now: 190, manual: false))
        state.record(.rateLimited, now: 200)
        #expect(state.nextEligibleAt == 320)
    }
    @Test func networkManualRetryAndClockRecovery() {
        var state = RefreshBackoffState()
        state.record(.network, now: 100)
        #expect(!state.isEligible(now: 110, manual: false))
        #expect(state.isEligible(now: 110, manual: true))
        state.normalizeClock(now: -100000)
        #expect(!state.isEligible(now: -100000, manual: false))
        #expect(state.isEligible(now: -99970, manual: false))
        state.record(.authentication, now: 200)
        #expect(!state.isEligible(now: 201, manual: true))
    }

    @Test func clockRollbackPreservesRateLimitWait() {
        var state = RefreshBackoffState()
        state.record(.rateLimited, now: 100, retryAfter: 90)
        state.normalizeClock(now: 99)
        #expect(!state.isEligible(now: 99, manual: true))
        #expect(state.nextEligibleAt == 189)
        #expect(state.isEligible(now: 189, manual: true))
    }
}
