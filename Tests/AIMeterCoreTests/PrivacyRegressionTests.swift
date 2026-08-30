import Foundation
import Testing
@testable import AIMeterCore

@Suite("Privacy regression boundaries")
struct PrivacyRegressionTests {
    private let secret = "sk-private-regression-token-1234567890"

    @Test("Cached snapshots remove credentials from every display string")
    func cacheRedactsCredentials() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let cache = SnapshotCache(directoryURL: directory)
        let snapshot = credentialBearingSnapshot

        try cache.save([snapshot])

        let bytes = try Data(contentsOf: cache.fileURL)
        let serialized = String(decoding: bytes, as: UTF8.self)
        #expect(!serialized.contains(secret))
        #expect(!serialized.localizedCaseInsensitiveContains("Bearer \(secret)"))

        let restored = try #require(cache.load().first)
        #expect(restored.primaryMetric?.label.contains(secret) == false)
        #expect(restored.primaryMetric?.resetDescription?.contains(secret) == false)
        #expect(restored.sourceVersion?.contains(secret) == false)
        #expect(restored.statusMessage?.contains(secret) == false)
    }

    @Test("Presentation text never exposes an authorization value")
    func presentationRedactsCredentials() {
        let presentation = ProviderPresentation(snapshot: credentialBearingSnapshot)

        #expect(!presentation.detailText.contains(secret))
        #expect(presentation.statusText?.contains(secret) == false)
        #expect(presentation.statusText?.contains("[REDACTED]") == true)
    }

    @Test("Known authorization formats are redacted without altering normal guidance")
    func redactsKnownSensitiveFormats() {
        let raw = "Retry with Authorization: Bearer \(secret); then sign in required"
        let redacted = SensitiveTextRedactor.redact(raw)

        #expect(!redacted.contains(secret))
        #expect(redacted.contains("Bearer [REDACTED]"))
        #expect(redacted.contains("sign in required"))
    }

    private var credentialBearingSnapshot: UsageSnapshot {
        UsageSnapshot(
            provider: .deepSeek,
            primaryMetric: UsageMetric(
                label: "Available balance \(secret)",
                current: 12,
                limit: nil,
                unit: .cny,
                kind: .balance,
                resetDescription: "Authorization: Bearer \(secret)"
            ),
            sourceVersion: "Bearer \(secret)",
            collectionStatus: .cached,
            statusMessage: "Request failed with Bearer \(secret)"
        )
    }
}
