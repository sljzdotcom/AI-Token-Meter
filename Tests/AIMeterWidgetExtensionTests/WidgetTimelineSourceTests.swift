import AIMeterCore
import Foundation
import Testing
@testable import AIMeterWidgetExtension

@Suite("Widget timeline source")
struct WidgetTimelineSourceTests {
    @Test("Missing shared data produces all three unavailable providers")
    func missingData() {
        let now = Date(timeIntervalSince1970: 1_800_000_000)
        let source = WidgetTimelineSource(load: { nil }, now: { now })

        let entry = source.currentEntry()

        #expect(entry.date == now)
        #expect(entry.envelope.providers.map(\.provider) == [.claude, .codex, .deepSeek])
        #expect(entry.envelope.providers.allSatisfy { $0.semantic == .unavailable })
    }

    @Test("Expired data is marked stale without changing its displayed value")
    func expiredData() throws {
        let now = Date(timeIntervalSince1970: 1_800_000_000)
        let envelope = WidgetSnapshotEnvelope(
            generatedAt: now.addingTimeInterval(-600),
            providers: [
                WidgetProviderSnapshot(
                    provider: .claude,
                    valueText: "18%",
                    detailText: "Current session",
                    fraction: 0.18,
                    semantic: .normal,
                    fetchedAt: now.addingTimeInterval(-600),
                    expiresAt: now.addingTimeInterval(-300)
                ),
            ],
            nextReset: nil,
            codexResetCredits: nil
        )
        let source = WidgetTimelineSource(load: { envelope }, now: { now })

        let provider = try #require(source.currentEntry().envelope.providers.first)

        #expect(provider.semantic == .stale)
        #expect(provider.valueText == "18%")
        #expect(provider.fraction == 0.18)
    }

    @Test("Preview data never depends on a real account")
    func previewData() {
        let date = Date(timeIntervalSince1970: 1_800_000_000)

        let preview = WidgetTimelineSource.previewEnvelope(at: date)

        #expect(preview.providers.map(\.valueText) == ["18%", "31%", "¥77.99"])
        #expect(preview.codexResetCredits?.availableCount == 1)
    }
}
