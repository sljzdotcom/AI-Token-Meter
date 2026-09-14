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
        let cliInfo = AntigravityCLIInfo(
            currentModel: "Gemini 3.8 Flash (High)",
            availableModelCount: 4,
            modelFamilies: ["Gemini 3.8 Flash", "Gemini 3.7 Flash"]
        )
        let original = UsageSnapshot(provider: .gemini, primaryMetric: weekly, secondaryMetric: fiveHour, fetchedAt: Date(timeIntervalSince1970: 1234), sourceVersion: "1.1.28", geminiQuotaMetrics: [fiveHour, weekly], antigravityCLIInfo: cliInfo)
        try cache.save([original])
        let coordinator = RefreshCoordinator(collectors: [FailedGemini()], cache: cache)
        let result = try #require(await coordinator.refresh().first)
        #expect(result.collectionStatus == .cached)
        #expect(result.geminiQuotaMetrics == [fiveHour, weekly])
        #expect(result.fetchedAt == Date(timeIntervalSince1970: 1234))
        #expect(result.statusMessage == "Antigravity CLI authentication mode is not supported")
        #expect(result.antigravityCLIInfo == cliInfo)
        #expect(try cache.load().first?.geminiQuotaMetrics == [fiveHour, weekly])
        #expect(try cache.load().first?.antigravityCLIInfo == cliInfo)
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
        let legacy = UsageSnapshot(
            provider: .gemini,
            primaryMetric: metrics[2],
            secondaryMetric: metrics[0],
            geminiQuotaMetrics: metrics
        )
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        try encoder.encode(LegacyGeminiCacheEnvelope(version: 1, snapshots: [legacy]))
            .write(to: cache.fileURL)

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

    @Test(arguments: [
        "Claude Sonnet",
        "Gemini user@example.com",
        "Gemini /Users/example/.config",
        "Gemini sk-proj-secretvalue",
        "Gemini Claude/GPT",
        "Gemini 3.8 Flash 13800000000",
        "Gemini 3.8 .-",
    ])
    func cachedUnsafeModelInformationDoesNotRemainVisible(_ unsafeModel: String) throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("gemini-cache-\(UUID())")
        defer { try? FileManager.default.removeItem(at: root) }
        let cache = SnapshotCache(directoryURL: root)
        let reset = Date(timeIntervalSince1970: 2_000)
        let metrics = [
            UsageMetric(label: "Gemini · Five hour", current: 60, limit: 100, unit: .percent, resetAt: reset),
            UsageMetric(label: "Gemini · Weekly", current: 25, limit: 100, unit: .percent, resetAt: reset),
        ]
        let legacy = UsageSnapshot(
            provider: .gemini,
            primaryMetric: metrics[0],
            secondaryMetric: metrics[1],
            geminiQuotaMetrics: metrics,
            antigravityCLIInfo: AntigravityCLIInfo(
                currentModel: unsafeModel,
                availableModelCount: 1,
                modelFamilies: [unsafeModel]
            )
        )
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        try JSONEncoder().encode(LegacyGeminiCacheEnvelope(version: 1, snapshots: [legacy]))
            .write(to: cache.fileURL)

        #expect(try cache.load().first?.antigravityCLIInfo == nil)
    }
}

private struct LegacyGeminiCacheEnvelope: Encodable {
    let version: Int
    let snapshots: [UsageSnapshot]
}

private struct FailedGemini: UsageCollector {
    let provider = UsageProvider.gemini
    func collect() async throws -> UsageSnapshot { throw UsageCollectionError.geminiUnavailable("Antigravity CLI authentication mode is not supported") }
}
