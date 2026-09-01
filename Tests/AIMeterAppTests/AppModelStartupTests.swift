import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("App model startup")
struct AppModelStartupTests {
    @Test("Initialization never blocks on a Keychain read")
    @MainActor
    func initializationDoesNotReadSecret() {
        let secretStore = ReadCountingSecretStore()
        let suiteName = "AppModelStartupTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let model = AppModel(defaults: defaults, secretStore: secretStore)

        #expect(secretStore.readCount == 0)
        #expect(!model.apiKeyConfigured)
    }

    @Test("Display font defaults, updates, and persists without restarting")
    @MainActor
    func displayFontPreference() {
        let suiteName = "AppModelStartupTests.Font.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let secretStore = ReadCountingSecretStore()

        let model = AppModel(defaults: defaults, secretStore: secretStore)
        #expect(model.displayFontChoice == .system)

        model.setDisplayFontChoice(.antonio)
        #expect(model.displayFontChoice == .antonio)
        #expect(DisplayFontPreferenceStore(defaults: defaults).load() == .antonio)

        model.restoreDefaultDisplayFont()
        #expect(model.displayFontChoice == .system)
        #expect(DisplayFontPreferenceStore(defaults: defaults).load() == .system)
    }

    @Test("Demo startup publishes its initial Widget snapshot once")
    @MainActor
    func demoStartupPublishesWidgetSnapshot() {
        let context = makeContext("DemoWidget")
        defer { context.defaults.removePersistentDomain(forName: context.suiteName) }
        let recorder = AppModelWidgetRecorder()
        let model = AppModel(
            defaults: context.defaults,
            secretStore: ReadCountingSecretStore(),
            widgetSnapshotPublisher: recorder.publisher,
            isDemoMode: true
        )

        model.start()

        #expect(recorder.published.count == 1)
        #expect(recorder.published[0].providers.map(\.provider) == [.claude, .codex, .deepSeek])
        #expect(model.serviceAccounts.values.allSatisfy { $0.connectionState == .connected })
    }

    @Test("A completed refresh publishes exactly the displayed snapshots")
    @MainActor
    func refreshPublishesWidgetSnapshot() async {
        let context = makeContext("RefreshWidget")
        defer { context.defaults.removePersistentDomain(forName: context.suiteName) }
        let recorder = AppModelWidgetRecorder()
        let expected = UsageSnapshot(
            provider: .codex,
            primaryMetric: UsageMetric(label: "Weekly", current: 31, limit: 100, unit: .percent)
        )
        let model = AppModel(
            defaults: context.defaults,
            secretStore: ReadCountingSecretStore(),
            widgetSnapshotPublisher: recorder.publisher,
            isDemoMode: false,
            refreshOperation: { [expected] in [expected] }
        )

        await model.refresh()

        #expect(model.snapshots == [expected])
        #expect(recorder.published.count == 1)
        #expect(recorder.published[0].providers.map(\.provider) == [.claude, .codex, .deepSeek])
        #expect(
            recorder.published[0].providers.first(where: { $0.provider == .codex })?.valueText
                == "31%"
        )
    }

    @Test("Changing the DeepSeek balance baseline republishes the recalculated state")
    @MainActor
    func balanceBaselineRepublishesWidgetSnapshot() async throws {
        let context = makeContext("BaselineWidget")
        defer { context.defaults.removePersistentDomain(forName: context.suiteName) }
        let recorder = AppModelWidgetRecorder()
        let deepSeek = UsageSnapshot(
            provider: .deepSeek,
            primaryMetric: UsageMetric(
                label: "Balance",
                current: 77.99,
                limit: nil,
                unit: .cny,
                kind: .balance
            )
        )
        let model = AppModel(
            defaults: context.defaults,
            secretStore: ReadCountingSecretStore(),
            widgetSnapshotPublisher: recorder.publisher,
            isDemoMode: false,
            refreshOperation: { [deepSeek] in [deepSeek] }
        )
        await model.refresh()
        recorder.published.removeAll()

        model.setDeepSeekBalanceBaseline(200)

        let published = try #require(
            recorder.published.first?.providers.first(where: { $0.provider == .deepSeek })
        )
        #expect(published.valueText == "¥77.99")
        #expect(abs((published.fraction ?? 0) - 0.61005) < 0.000_001)
    }

    @Test("Widget publication failure never changes app refresh state")
    @MainActor
    func widgetFailureDoesNotAffectRefresh() async {
        let context = makeContext("WidgetFailure")
        defer { context.defaults.removePersistentDomain(forName: context.suiteName) }
        let expected = UsageSnapshot(provider: .claude)
        let publisher = WidgetSnapshotPublisher(
            save: { _ in throw WidgetPublishingFailureForAppModel.expected },
            reload: {}
        )
        let model = AppModel(
            defaults: context.defaults,
            secretStore: ReadCountingSecretStore(),
            widgetSnapshotPublisher: publisher,
            isDemoMode: false,
            refreshOperation: { [expected] in [expected] }
        )

        await model.refresh()

        #expect(model.snapshots == [expected])
        #expect(model.lastUpdatedAt != nil)
        #expect(!model.isRefreshing)
    }

    private func makeContext(_ suffix: String) -> (suiteName: String, defaults: UserDefaults) {
        let suiteName = "AppModelStartupTests.\(suffix).\(UUID().uuidString)"
        return (suiteName, UserDefaults(suiteName: suiteName)!)
    }
}

@MainActor
private final class AppModelWidgetRecorder {
    var published: [WidgetSnapshotEnvelope] = []

    var publisher: WidgetSnapshotPublisher {
        WidgetSnapshotPublisher(
            save: { [weak self] envelope in
                self?.published.append(envelope)
            },
            reload: {}
        )
    }
}

private enum WidgetPublishingFailureForAppModel: Error {
    case expected
}

private final class ReadCountingSecretStore: SecretStore, @unchecked Sendable {
    private let lock = NSLock()
    private var storedReadCount = 0

    var readCount: Int {
        lock.withLock { storedReadCount }
    }

    func read() throws -> String? {
        lock.withLock { storedReadCount += 1 }
        return "configured"
    }

    func save(_ secret: String) throws {}
    func delete() throws {}
}
