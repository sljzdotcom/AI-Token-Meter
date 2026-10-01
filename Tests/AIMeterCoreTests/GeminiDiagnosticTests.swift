import Foundation
import Testing
@testable import AIMeterCore

@Suite("Antigravity diagnostic privacy")
struct GeminiDiagnosticTests {
    @Test func storesOnlyBoundedAllowlistedRecords() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let file = directory.appendingPathComponent("diagnostics.json")
        let store = GeminiDiagnosticStore(fileURL: file)

        for index in 0..<25 {
            await store.record(GeminiDiagnosticRecord(
                recordedAt: Date(timeIntervalSince1970: TimeInterval(index)),
                stage: .usage,
                category: .timedOut,
                durationMilliseconds: 1_000 + index,
                outputTruncated: index == 24
            ))
        }

        let records = await store.records()
        let data = try Data(contentsOf: file)
        let root = try #require(JSONSerialization.jsonObject(with: data) as? [[String: Any]])
        #expect(records.count == 20)
        #expect(records.first?.recordedAt == Date(timeIntervalSince1970: 5))
        #expect(records.last?.outputTruncated == true)
        #expect(data.count <= 16 * 1_024)
        #expect(Set(root[0].keys) == ["recordedAt", "stage", "category", "durationMilliseconds", "outputTruncated"])
    }

    @Test func summaryReportsUnknownLegacyPauseWithoutInventingAStage() async {
        let store = GeminiDiagnosticStore(fileURL: nil)

        let summary = await store.summary(pauseReason: .unknown, lastQuotaAt: Date(timeIntervalSince1970: 1_234))

        #expect(summary.contains("refreshPaused=true reason=unknown"))
        #expect(summary.contains("lastQuotaAt=1970-01-01T00:20:34Z"))
        #expect(summary.contains("stage=unknown"))
        #expect(!summary.contains("stdout"))
        #expect(!summary.contains("environment="))
    }

    @Test func failureTimestampIsNotReportedAsLastQuotaWhenSnapshotHasNoQuota() {
        let failureTime = Date(timeIntervalSince1970: 1_234)
        let failure = UsageSnapshot(provider: .gemini, availability: .unavailable,
            fetchedAt: failureTime, collectionStatus: .unavailable,
            statusMessage: "Timed out", geminiQuotaMetrics: nil)
        let cached = UsageSnapshot(provider: .gemini, availability: .available,
            fetchedAt: failureTime, collectionStatus: .cached,
            geminiQuotaMetrics: [UsageMetric(label: "Pro", current: 25, limit: 100, unit: .percent)])

        #expect(failure.geminiQuotaFetchedAt == nil)
        #expect(cached.geminiQuotaFetchedAt == failureTime)
    }

    @Test func onlyFixedDiagnosticFieldsCanBeSerialized() throws {
        let record = GeminiDiagnosticRecord(
            recordedAt: Date(timeIntervalSince1970: 1),
            stage: .version,
            category: .environmentRejected,
            durationMilliseconds: 17,
            outputTruncated: false
        )
        let data = try JSONEncoder().encode(record)
        let object = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])

        #expect(Set(object.keys) == ["recordedAt", "stage", "category", "durationMilliseconds", "outputTruncated"])
        #expect(String(decoding: data, as: UTF8.self).contains("environmentRejected"))
    }
}
