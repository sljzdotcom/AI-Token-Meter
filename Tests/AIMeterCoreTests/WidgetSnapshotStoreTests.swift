import Foundation
import Testing
@testable import AIMeterCore

@Suite("Widget snapshot store")
struct WidgetSnapshotStoreTests {
    @Test("Atomically round-trips only version one envelopes")
    func roundTripsVersionOne() throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let store = WidgetSnapshotStore(directoryURL: directory)
        let expected = envelope()

        try store.save(expected)
        #expect(try store.load() == expected)

        try Data(#"{"version":2,"generatedAt":0,"providers":[]}"#.utf8)
            .write(to: store.fileURL, options: .atomic)
        #expect(try store.load() == nil)
    }

    @Test("Missing or corrupt data loads as an empty state")
    func handlesMissingAndCorruptData() throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let store = WidgetSnapshotStore(directoryURL: directory)

        #expect(try store.load() == nil)

        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try Data("not-json".utf8).write(to: store.fileURL, options: .atomic)
        #expect(try store.load() == nil)
    }

    @Test("Persisted Widget JSON excludes credentials and personal identifiers")
    func persistedJSONIsPrivacySafe() throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let store = WidgetSnapshotStore(directoryURL: directory)
        let secret = "sk-widget-private-123456789"
        let snapshots = [UsageSnapshot(
            provider: .codex,
            primaryMetric: UsageMetric(
                label: "person@example.com 13800000000 Cookie: session=abcdef123456",
                current: 22,
                limit: 100,
                unit: .percent,
                resetDescription: "Authorization: Bearer \(secret)"
            ),
            sourceVersion: "reset-credit-id=opaque-998877",
            collectionStatus: .cached,
            statusMessage: secret,
            codexResetCredits: CodexResetCreditsSummary(
                availableCount: 1,
                credits: [CodexResetCreditDisplay(title: "redeem-id opaque-998877", expiresAt: nil)],
                hasCompleteDetails: true
            )
        )]
        let envelope = WidgetSnapshotBuilder().build(
            snapshots: snapshots,
            generatedAt: Date(timeIntervalSince1970: 100)
        )

        try store.save(envelope)

        let serialized = String(decoding: try Data(contentsOf: store.fileURL), as: UTF8.self)
        #expect(!serialized.contains(secret))
        #expect(!serialized.contains("person@example.com"))
        #expect(!serialized.contains("13800000000"))
        #expect(!serialized.contains("abcdef123456"))
        #expect(!serialized.contains("opaque-998877"))
        #expect(serialized.contains("[REDACTED]"))
    }

    private func temporaryDirectory() -> URL {
        FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-token-widget-\(UUID().uuidString)", isDirectory: true)
    }

    private func envelope() -> WidgetSnapshotEnvelope {
        WidgetSnapshotEnvelope(
            version: 1,
            generatedAt: Date(timeIntervalSince1970: 123),
            providers: [WidgetProviderSnapshot(
                provider: .claude,
                valueText: "18%",
                detailText: "Session",
                fraction: 0.18,
                semantic: .normal,
                fetchedAt: Date(timeIntervalSince1970: 120),
                expiresAt: Date(timeIntervalSince1970: 420)
            )],
            nextReset: nil,
            codexResetCredits: nil
        )
    }
}
