import Foundation
import Testing
@testable import AIMeterCore

@Suite("Gemini cached quota")
struct GeminiCacheTests {
    @Test func failurePreservesAllTiersAndOriginalSampleTime() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("gemini-cache-\(UUID())")
        defer { try? FileManager.default.removeItem(at: root) }
        let cache = SnapshotCache(directoryURL: root)
        let reset = Date(timeIntervalSince1970: 2_000)
        let fiveHour = UsageMetric(label: "Gemini · Five hour", current: 10, limit: 100, unit: .percent, resetAt: reset)
        let weekly = UsageMetric(label: "Gemini · Weekly", current: 20, limit: 100, unit: .percent, resetAt: reset)
        let original = UsageSnapshot(provider: .gemini, primaryMetric: weekly, secondaryMetric: fiveHour, fetchedAt: Date(timeIntervalSince1970: 1234), sourceVersion: "1.1.28", geminiQuotaMetrics: [fiveHour, weekly])
        try cache.save([original])
        let coordinator = RefreshCoordinator(collectors: [FailedGemini()], cache: cache)
        let result = try #require(await coordinator.refresh().first)
        #expect(result.collectionStatus == .cached)
        #expect(result.geminiQuotaMetrics == [fiveHour, weekly])
        #expect(result.fetchedAt == Date(timeIntervalSince1970: 1234))
        #expect(result.statusMessage == "Antigravity CLI authentication mode is not supported")
        #expect(try cache.load().first?.geminiQuotaMetrics == [fiveHour, weekly])
    }

    @Test func legacyFourWindowCacheKeepsOnlyGeminiAndRecomputesTheSummary() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("gemini-cache-\(UUID())")
        defer { try? FileManager.default.removeItem(at: root) }
        let cache = SnapshotCache(directoryURL: root)
        let reset = Date(timeIntervalSince1970: 2_000)
        let metrics = [
            UsageMetric(label: "Gemini · Five hour", current: 60, limit: 100, unit: .percent, resetAt: reset),
            UsageMetric(label: "Gemini · Weekly", current: 25, limit: 100, unit: .percent, resetAt: reset),
            UsageMetric(label: "Claude/GPT · Five hour", current: 80, limit: 100, unit: .percent, resetAt: reset),
            UsageMetric(label: "Claude/GPT · Weekly", current: 20, limit: 100, unit: .percent, resetAt: reset),
        ]
        try cache.save([UsageSnapshot(
            provider: .gemini,
            primaryMetric: metrics[2],
            secondaryMetric: metrics[0],
            geminiQuotaMetrics: metrics
        )])

        let restored = try #require(cache.load().first)

        #expect(restored.geminiQuotaMetrics?.map(\.label) == ["Gemini · Five hour", "Gemini · Weekly"])
        #expect(restored.primaryMetric?.label == "Gemini · Five hour")
        #expect(restored.secondaryMetric?.label == "Gemini · Weekly")
    }

    @Test func incompleteLegacyGeminiCacheDoesNotRemainVisible() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("gemini-cache-\(UUID())")
        defer { try? FileManager.default.removeItem(at: root) }
        let cache = SnapshotCache(directoryURL: root)
        let other = UsageMetric(label: "Claude/GPT · Five hour", current: 80, limit: 100, unit: .percent)
        try cache.save([UsageSnapshot(provider: .gemini, primaryMetric: other, geminiQuotaMetrics: [other])])

        let restored = try #require(cache.load().first)

        #expect(restored.geminiQuotaMetrics?.isEmpty != false)
        #expect(restored.primaryMetric == nil)
        #expect(restored.secondaryMetric == nil)
    }
}
private struct FailedGemini: UsageCollector {
    let provider = UsageProvider.gemini
    func collect() async throws -> UsageSnapshot { throw UsageCollectionError.geminiUnavailable("Antigravity CLI authentication mode is not supported") }
}
