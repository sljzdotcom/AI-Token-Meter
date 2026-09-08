import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("Gemini capability state")
@MainActor
struct GeminiAvailabilityTests {
    // Catch accidental launch/login or fake quota in initial and refreshed runtime state.
    @Test func unavailableAccountNeverOffersAuthenticationOrFabricatesProgress() async throws {
        let suite = "GeminiAvailability-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let model = AppModel(defaults: defaults, secretStore: GeminiTestSecretStore(), widgetSnapshotPublisher: nil,
                             isDemoMode: false, refreshOperation: { [] },
                             authenticationOpenOperation: { _ in Issue.record("Must not launch authentication") },
                             installationOpenOperation: { _ in Issue.record("Must not launch installation"); return false })
        for refreshed in [false, true] {
            if refreshed { await model.refresh() }
            let snapshot = try #require(model.snapshots.first { $0.provider == .gemini })
            #expect(snapshot.collectionStatus == .unavailable)
            #expect(snapshot.availability == .unavailable)
            #expect(snapshot.primaryMetric == nil)
            #expect(ProviderPresentation(snapshot: snapshot).ringFraction == nil)
        }
        let status = await model.checkServiceAccount(.gemini)
        #expect(status.connectionState == .unavailable)
        #expect(status.accountLabel == nil)
        #expect(status.checkedAt == nil)
        #expect(model.serviceAction(for: .gemini).title == "Check Status")
        #expect(model.beginSignIn(.gemini) == nil)
        #expect(model.beginCLIInstallation(.gemini) == nil)
    }
}

private struct GeminiTestSecretStore: SecretStore {
    func read() throws -> String? { nil }
    func save(_ secret: String) throws {}
    func delete() throws {}
}
