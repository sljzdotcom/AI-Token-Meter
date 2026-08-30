import Foundation
import Testing
@testable import AIMeterCore

@Suite("Snapshot cache", .serialized)
struct SnapshotCacheTests {
    @Test("Atomically persists and reloads non-sensitive snapshots")
    func persistsSnapshots() throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let cache = SnapshotCache(directoryURL: directory)
        let snapshot = makeSnapshot(fetchedAt: Date(timeIntervalSince1970: 1_000))

        try cache.save([snapshot])
        let loaded = try cache.load()

        #expect(loaded == [snapshot])
        let raw = try String(contentsOf: cache.fileURL, encoding: .utf8).lowercased()
        #expect(!raw.contains("apikey"))
        #expect(!raw.contains("authorization"))
        #expect(!raw.contains("bearer"))
        #expect(!raw.contains("token"))
    }

    @Test("Treats a corrupt cache as empty instead of failing refresh")
    func ignoresCorruptCache() throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let cache = SnapshotCache(directoryURL: directory)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try Data("not-json".utf8).write(to: cache.fileURL)

        #expect(try cache.load().isEmpty)
    }

    @Test("Preserves timestamps so callers can identify stale cached data")
    func preservesStaleness() throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let cache = SnapshotCache(directoryURL: directory)
        try cache.save([makeSnapshot(fetchedAt: Date(timeIntervalSince1970: 100))])

        let loaded = try #require(try cache.load().first)

        #expect(loaded.isStale(at: Date(timeIntervalSince1970: 401)))
    }

    private func temporaryDirectory() -> URL {
        FileManager.default.temporaryDirectory
            .appendingPathComponent("AI-Meter-CacheTests-\(UUID().uuidString)", isDirectory: true)
    }

    private func makeSnapshot(fetchedAt: Date) -> UsageSnapshot {
        UsageSnapshot(
            provider: .codex,
            primaryMetric: UsageMetric(
                id: UUID(uuidString: "00000000-0000-0000-0000-000000000001")!,
                label: "5h limit",
                current: 27,
                limit: 100,
                unit: .percent
            ),
            fetchedAt: fetchedAt,
            staleAfter: 300,
            sourceVersion: "test",
            collectionStatus: .fresh
        )
    }
}
