import Foundation
import Testing
@testable import AIMeterCore

@Suite("DeepSeek history store")
struct DeepSeekHistoryStoreTests {
    @Test("Persists only normalized display history")
    func persistsSafeHistory() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-history-\(UUID().uuidString)", isDirectory: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let store = DeepSeekHistoryStore(directoryURL: directory)
        let history = DeepSeekUsageHistory(
            days: [DeepSeekDailyUsage(date: Date(timeIntervalSince1970: 1_900_000_000), costCNY: 1.25, requestCount: 2, tokenCount: 3)],
            updatedAt: Date(timeIntervalSince1970: 1_900_000_100),
            statusMessage: "Synced"
        )

        try store.save(history)
        let data = try Data(contentsOf: store.fileURL)
        let payload = try #require(String(data: data, encoding: .utf8)).lowercased()

        #expect(try store.load() == history)
        #expect(!payload.contains("cookie"))
        #expect(!payload.contains("authorization"))
        #expect(!payload.contains("api_key"))
        #expect(!payload.contains("rawresponse"))
    }

    @Test("Missing history cache loads as nil")
    func missingCache() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-missing-history-\(UUID().uuidString)", isDirectory: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let store = DeepSeekHistoryStore(directoryURL: directory)

        #expect(try store.load() == nil)
    }
}
