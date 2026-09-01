import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("Widget snapshot publisher")
struct WidgetSnapshotPublisherTests {
    @Test("Stores a sanitized envelope before requesting a timeline reload")
    @MainActor
    func savesBeforeReload() throws {
        let recorder = WidgetPublishingRecorder()
        let publisher = WidgetSnapshotPublisher(
            save: { envelope in
                recorder.savedEnvelope = envelope
                recorder.events.append("save")
            },
            reload: {
                recorder.events.append("reload")
            }
        )

        publisher.publish([
            UsageSnapshot(
                provider: .claude,
                primaryMetric: UsageMetric(
                    label: "Current session",
                    current: 18,
                    limit: 100,
                    unit: .percent
                )
            ),
        ])

        #expect(recorder.events == ["save", "reload"])
        #expect(recorder.savedEnvelope?.providers.first?.valueText == "18%")
    }

    @Test("Does not request a reload when persistence fails")
    @MainActor
    func persistenceFailureStopsReload() {
        let recorder = WidgetPublishingRecorder()
        let publisher = WidgetSnapshotPublisher(
            save: { _ in throw WidgetPublishingFailure.expected },
            reload: { recorder.events.append("reload") },
            log: { message in recorder.messages.append(message) }
        )

        publisher.publish([UsageSnapshot(provider: .codex)])

        #expect(recorder.events.isEmpty)
        #expect(recorder.messages == ["Widget snapshot publication failed."])
    }

    @Test("Missing App Group configuration disables publishing quietly")
    @MainActor
    func missingAppGroupDisablesPublishing() {
        #expect(WidgetSnapshotPublisher.production(appGroupIdentifier: nil) == nil)
        #expect(WidgetSnapshotPublisher.production(appGroupIdentifier: "") == nil)
    }
}

@MainActor
private final class WidgetPublishingRecorder {
    var events: [String] = []
    var messages: [String] = []
    var savedEnvelope: WidgetSnapshotEnvelope?
}

private enum WidgetPublishingFailure: Error {
    case expected
}
