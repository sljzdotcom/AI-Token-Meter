import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("Service accounts in Settings", .serialized)
@MainActor
struct ServiceAccountSettingsTests {
    @Test("Connected CLI accounts keep Sign in again available")
    func connectedAccountCanRelogin() async {
        let model = makeModel(
            accountRefresh: { provider in
                let statuses = [
                    ServiceAccountStatus(
                        provider: .claude,
                        connectionState: .connected,
                        accountLabel: "m@example.com",
                        accountDetail: "OAuth"
                    ),
                    ServiceAccountStatus(
                        provider: .codex,
                        connectionState: .connected,
                        accountLabel: "codex@example.com",
                        accountDetail: "ChatGPT · Pro"
                    ),
                    ServiceAccountStatus(provider: .deepSeek, connectionState: .signInRequired),
                ]
                return provider.map { selected in statuses.filter { $0.provider == selected } } ?? statuses
            }
        )

        await model.refreshServiceAccounts()

        #expect(model.serviceAccounts[.claude]?.accountLabel == "m@example.com")
        #expect(model.serviceAccounts[.codex]?.accountDetail == "ChatGPT · Pro")
        #expect(model.signInButtonTitle(for: .claude) == "Sign in again")
        #expect(model.signInButtonTitle(for: .codex) == "Sign in again")
    }

    @Test("A successful DeepSeek replacement updates identity and refreshes usage")
    func replacesDeepSeek() async {
        let usageRefreshes = ServiceAccountCounter()
        let model = makeModel(
            refreshOperation: { usageRefreshes.increment(); return [] },
            deepSeekReplace: { _ in
                ServiceAccountStatus(
                    provider: .deepSeek,
                    connectionState: .connected,
                    accountLabel: "API Key ••••ABCD"
                )
            }
        )

        let didReplace = await model.replaceDeepSeekAPIKey("new-key")

        #expect(didReplace)
        #expect(model.serviceAccounts[.deepSeek]?.accountLabel == "API Key ••••ABCD")
        #expect(model.apiKeyConfigured)
        #expect(!model.isReplacingDeepSeekAPIKey)
        #expect(usageRefreshes.value == 1)
        #expect(model.settingsMessageKind == .deepSeekCredential)
    }

    @Test("A rejected DeepSeek candidate remains in the field and keeps prior status")
    func rejectedDeepSeekCandidate() async {
        let oldStatus = ServiceAccountStatus(
            provider: .deepSeek,
            connectionState: .connected,
            accountLabel: "API Key ••••OLD1"
        )
        let model = makeModel(
            accountRefresh: { _ in [oldStatus] },
            deepSeekReplace: { _ in throw DeepSeekCredentialReplacementError.invalidKey }
        )
        await model.refreshServiceAccounts()

        let didReplace = await model.replaceDeepSeekAPIKey("bad-key")

        #expect(!didReplace)
        #expect(model.serviceAccounts[.deepSeek] == oldStatus)
        #expect(model.settingsMessage == "DeepSeek rejected this API Key. The existing Key was kept.")
    }

    @Test("CLI login opens once, polls, and refreshes usage after connection")
    func loginAndPoll() async {
        let loginRecorder = ServiceAccountLoginRecorder()
        let statusSequence = ServiceAccountStatusSequence([
            .init(provider: .claude, connectionState: .signInRequired),
            .init(
                provider: .claude,
                connectionState: .connected,
                accountLabel: "signed-in@example.com"
            ),
        ])
        let usageRefreshes = ServiceAccountCounter()
        let model = makeModel(
            refreshOperation: { usageRefreshes.increment(); return [] },
            accountRefresh: { _ in [await statusSequence.next()] },
            authenticationOpen: { loginRecorder.record($0) },
            pollAttempts: 2,
            pollSleep: { _ in }
        )

        let task = model.beginSignIn(.claude)
        await task?.value

        #expect(loginRecorder.providers == [.claude])
        #expect(model.serviceAccounts[.claude]?.accountLabel == "signed-in@example.com")
        #expect(model.settingsMessageKind == .claudeAuthentication)
        #expect(usageRefreshes.value == 1)
    }

    @Test("Re-login does not report success while the original connected identity is unchanged")
    func connectedReloginWaitsForChange() async {
        let connected = ServiceAccountStatus(
            provider: .codex,
            connectionState: .connected,
            accountLabel: "same@example.com",
            accountDetail: "ChatGPT · Pro"
        )
        let usageRefreshes = ServiceAccountCounter()
        let model = makeModel(
            refreshOperation: { usageRefreshes.increment(); return [] },
            accountRefresh: { _ in [connected] },
            authenticationOpen: { _ in },
            pollAttempts: 1,
            pollSleep: { _ in }
        )
        await model.refreshServiceAccounts()

        let task = model.beginSignIn(.codex)
        await task?.value

        #expect(model.settingsMessage == "Sign-in is still pending. Finish in Terminal, then choose Check Status.")
        #expect(usageRefreshes.value == 0)
    }

    @Test("Starting a second login check cancels the prior provider task")
    func cancelsPriorPolling() async {
        let model = makeModel(
            accountRefresh: { provider in [
                ServiceAccountStatus(provider: provider ?? .claude, connectionState: .signInRequired),
            ] },
            authenticationOpen: { _ in },
            pollAttempts: 3,
            pollSleep: { _ in try await Task.sleep(for: .seconds(60)) }
        )

        let first = model.beginSignIn(.claude)
        await Task.yield()
        let second = model.beginSignIn(.claude)
        #expect(first?.isCancelled == true)
        second?.cancel()
        await first?.value
        await second?.value
    }

    @Test("Initialization still performs no account or Keychain reads")
    func initializationRemainsNonBlocking() {
        let secretStore = ServiceAccountReadCountingStore()
        let accountRefreshes = ServiceAccountCounter()

        _ = makeModel(
            secretStore: secretStore,
            accountRefresh: { _ in accountRefreshes.increment(); return [] }
        )

        #expect(secretStore.readCount == 0)
        #expect(accountRefreshes.value == 0)
    }

    private func makeModel(
        secretStore: any SecretStore = ServiceAccountReadCountingStore(),
        refreshOperation: @escaping @Sendable () async -> [UsageSnapshot] = { [] },
        accountRefresh: @escaping @Sendable (UsageProvider?) async -> [ServiceAccountStatus] = { _ in [] },
        authenticationOpen: @escaping (UsageProvider) throws -> Void = { _ in },
        deepSeekReplace: @escaping @Sendable (String) async throws -> ServiceAccountStatus = { _ in
            ServiceAccountStatus(provider: .deepSeek, connectionState: .signInRequired)
        },
        pollAttempts: Int = 2,
        pollSleep: @escaping @Sendable (Duration) async throws -> Void = { _ in }
    ) -> AppModel {
        let suiteName = "ServiceAccountSettingsTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defaults.removePersistentDomain(forName: suiteName)
        return AppModel(
            defaults: defaults,
            secretStore: secretStore,
            widgetSnapshotPublisher: nil,
            isDemoMode: false,
            refreshOperation: refreshOperation,
            serviceAccountRefreshOperation: accountRefresh,
            authenticationOpenOperation: authenticationOpen,
            deepSeekReplaceOperation: deepSeekReplace,
            signInPollAttempts: pollAttempts,
            signInPollInterval: .milliseconds(1),
            signInSleep: pollSleep
        )
    }
}

private final class ServiceAccountCounter: @unchecked Sendable {
    private let lock = NSLock()
    private var count = 0
    func increment() { lock.withLock { count += 1 } }
    var value: Int { lock.withLock { count } }
}

@MainActor
private final class ServiceAccountLoginRecorder {
    private(set) var providers: [UsageProvider] = []
    func record(_ provider: UsageProvider) { providers.append(provider) }
}

private actor ServiceAccountStatusSequence {
    private var statuses: [ServiceAccountStatus]
    init(_ statuses: [ServiceAccountStatus]) { self.statuses = statuses }
    func next() -> ServiceAccountStatus {
        statuses.isEmpty
            ? ServiceAccountStatus(provider: .claude, connectionState: .signInRequired)
            : statuses.removeFirst()
    }
}

private final class ServiceAccountReadCountingStore: SecretStore, @unchecked Sendable {
    private let lock = NSLock()
    private var count = 0
    var readCount: Int { lock.withLock { count } }
    func read() throws -> String? { lock.withLock { count += 1 }; return nil }
    func save(_ secret: String) throws {}
    func delete() throws {}
}
