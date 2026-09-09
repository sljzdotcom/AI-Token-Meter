import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("Gemini capability state")
@MainActor
struct GeminiAvailabilityTests {
    @Test("Gemini setup help uses the supported release and an installation page")
    func setupHelpTargetsSupportedRelease() {
        #expect(GeminiInstallationGuide.url.absoluteString == "https://geminicli.com/docs/get-started/installation/")
        #expect(GeminiInstallationGuide.installCommand == "npm install -g @google/gemini-cli@0.58.0")
        #expect(GeminiInstallationGuide.instructions(for: .notInstalled) == [
            "Requires Node.js 20 or later.",
            "npm install -g @google/gemini-cli@0.58.0",
            "Run gemini and choose Sign in with Google.",
            "Return to AI Token Meter and choose Retry.",
        ])
        #expect(GeminiInstallationGuide.instructions(for: .signInRequired) == [
            "Run gemini and choose Sign in with Google.",
            "Return to AI Token Meter and choose Retry.",
        ])
        #expect(GeminiInstallationGuide.instructions(for: .connected).isEmpty)
    }

    @Test func refreshPublishesRealGeminiQuotaAndAccountState() async throws {
        let suite = "GeminiRefresh-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let metrics = [UsageMetric(label: "Pro", current: 25, limit: 100, unit: .percent), UsageMetric(label: "Flash", current: 60, limit: 100, unit: .percent)]
        let sample = UsageSnapshot(provider: .gemini, primaryMetric: metrics[1], secondaryMetric: metrics[0], sourceVersion: "0.58.0", geminiQuotaMetrics: metrics)
        let model = AppModel(defaults: defaults, secretStore: GeminiTestSecretStore(), widgetSnapshotPublisher: nil,
                             isDemoMode: false, refreshOperation: { [sample] })
        await model.refresh()
        #expect(model.snapshots.first?.geminiQuotaMetrics?.map(\.current) == [25, 60])
        #expect(model.serviceAccounts[.gemini]?.connectionState == .connected)
        #expect(model.serviceAccounts[.gemini]?.accountLabel == nil)
        let checked = await model.checkServiceAccount(.gemini)
        #expect(checked.connectionState == .connected)
        #expect(model.beginSignIn(.gemini) == nil)
        #expect(model.beginCLIInstallation(.gemini) == nil)
    }

    @Test(arguments: [CollectionStatus.cached, .authenticationRequired, .notInstalled, .unavailable])
    func failedRefreshDoesNotLeaveSettingsConnected(_ status: CollectionStatus) async throws {
        let suite = "GeminiFailure-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let metric = UsageMetric(label: "Pro", current: 25, limit: 100, unit: .percent)
        let previous = Date(timeIntervalSince1970: 1234)
        let sample = UsageSnapshot(provider: .gemini, primaryMetric: status == .cached ? metric : nil,
            availability: status == .cached ? .available : .unavailable, fetchedAt: previous,
            collectionStatus: status, statusMessage: "Latest Gemini check failed",
            geminiQuotaMetrics: status == .cached ? [metric] : nil)
        let model = AppModel(defaults: defaults, secretStore: GeminiTestSecretStore(), widgetSnapshotPublisher: nil,
                             isDemoMode: false, refreshOperation: { [sample] })
        await model.refresh()
        let state = model.serviceAccounts[.gemini]
        #expect(state?.connectionState == (status == .authenticationRequired ? .signInRequired : status == .notInstalled ? .notInstalled : .unavailable))
        #expect(state?.accountDetail == "Latest Gemini check failed")
        #expect(state?.accountLabel == nil)
        #expect(model.snapshots.first?.fetchedAt == previous)
        #expect(model.snapshots.first?.primaryMetric?.current == (status == .cached ? 25 : nil))
    }

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
