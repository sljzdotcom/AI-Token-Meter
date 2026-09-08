import Foundation
import Testing
@testable import AIMeterCore

@Suite("Gemini cached quota")
struct GeminiCacheTests {
    @Test func failurePreservesAllTiersAndOriginalSampleTime() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("gemini-cache-\(UUID())")
        defer { try? FileManager.default.removeItem(at: root) }
        let cache = SnapshotCache(directoryURL: root)
        let metric = UsageMetric(label: "Flash Lite", current: 20, limit: 100, unit: .percent)
        let original = UsageSnapshot(provider: .gemini, primaryMetric: metric, fetchedAt: Date(timeIntervalSince1970: 1234), sourceVersion: "0.58.0", geminiQuotaMetrics: [metric])
        try cache.save([original])
        let coordinator = RefreshCoordinator(collectors: [FailedGemini()], cache: cache)
        let result = try #require(await coordinator.refresh().first)
        #expect(result.collectionStatus == .cached)
        #expect(result.geminiQuotaMetrics == [metric])
        #expect(result.fetchedAt == Date(timeIntervalSince1970: 1234))
        #expect(result.statusMessage == "Gemini CLI authentication mode is not supported")
        #expect(try cache.load().first?.geminiQuotaMetrics == [metric])
    }
}
private struct FailedGemini: UsageCollector {
    let provider = UsageProvider.gemini
    func collect() async throws -> UsageSnapshot { throw UsageCollectionError.geminiUnavailable("Gemini CLI authentication mode is not supported") }
}
