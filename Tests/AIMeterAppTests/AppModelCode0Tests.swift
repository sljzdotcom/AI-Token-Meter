import AIMeterCore
import Foundation
import Observation
import Testing
@testable import AIMeterApp

@Suite("Code0 AppModel isolated session composition", .serialized)
@MainActor
struct AppModelCode0Tests {
    @Test func initialCachedSessionIsPresentedWithoutAuthorization() throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let cached = code0AppSnapshot(2, cached: true)
        let session = AppCode0SessionDouble(snapshot: cached)
        let model = context.model(session: session)
        defer { model.stop() }
        #expect(model.snapshots.filter { $0.provider == .code0 } == [cached])
        #expect(model.code0SessionState == .paused)
        #expect(model.serviceAccounts[.code0]?.connectionState == .lastKnown)
        #expect(session.actions.isEmpty)
        #expect(!session.isAutomaticallySyncing)
    }

    @Test(arguments: [false, true])
    func lateOrdinaryRefreshUsesCurrentBOrNilIncludingPollution(disconnected: Bool) async throws {
        for polluted in [false, true] {
            let context = try Code0AppTestContext()
            defer { context.clean() }
            let gate = Code0AppResultGate<[UsageSnapshot]>()
            let session = AppCode0SessionDouble(snapshot: code0AppSnapshot(1, cached: true))
            let model = context.model(session: session, refresh: { await gate.wait() })
            defer { model.stop() }
            let task = Task { @MainActor in await model.refresh() }
            let started = await code0AppWait { await gate.isWaiting }
            let current = disconnected ? nil : code0AppSnapshot(8)
            session.emit(current)
            #expect(model.snapshots.filter { $0.provider == .code0 } == current.map { [$0] } ?? [])
            let old = code0AppSnapshot(1, cached: true)
            let ordinary = UsageSnapshot(provider: .claude, primaryMetric: UsageMetric(label: "Weekly", current: 31, limit: 100, unit: .percent))
            await gate.finish([ordinary] + (polluted ? [old, old] : []))
            await task.value
            #expect(started)
            #expect(model.snapshots.filter { $0.provider == .code0 } == current.map { [$0] } ?? [])
            #expect(model.snapshots.first { $0.provider == .claude }?.primaryMetric?.current == 31)
            #expect(session.actions.isEmpty)
        }
    }

    @Test(arguments: [false, true])
    func lastOrdinaryActorReadUsesCurrentSession(disconnected: Bool) async throws {
        for checkingAccounts in [false, true] {
            let context = try Code0AppTestContext()
            defer { context.clean() }
            let finalRead = Code0AppResultGate<Bool>()
            let session = AppCode0SessionDouble(snapshot: code0AppSnapshot(1, cached: true))
            let model = context.model(session: session,
                refresh: { [code0AppSnapshot(1)] },
                accounts: { _ in [.init(provider: .code0, connectionState: .connected, accountLabel: "stale-final-await")] },
                pauseReason: { _ = await finalRead.wait(); return nil })
            defer { model.stop() }
            let task = Task { @MainActor in
                if checkingAccounts { await model.refreshServiceAccounts() }
                else { await model.refresh() }
            }
            let reachedLastRead = await code0AppWait { await finalRead.isWaiting }
            let current = disconnected ? nil : code0AppSnapshot(9, cached: true)
            session.emit(current)
            await finalRead.finish(true)
            await task.value
            #expect(reachedLastRead)
            #expect(model.snapshots.filter { $0.provider == .code0 } == current.map { [$0] } ?? [])
            #expect(model.serviceAccounts[.code0]?.accountLabel == nil)
            #expect(session.actions.isEmpty)
        }
    }

    @Test func ordinaryCode0CannotCreateSnapshotWithoutSessionValue() async throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let session = AppCode0SessionDouble()
        let model = context.model(session: session, refresh: { [code0AppSnapshot(99)] })
        defer { model.stop() }
        await model.refresh()
        #expect(model.snapshots.allSatisfy { $0.provider != .code0 })
        #expect(session.actions.isEmpty)
    }

    @Test func snapshotCallbackIsSignalAndCannotOverrideCurrentNilWithStaleArgument() throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let session = AppCode0SessionDouble(snapshot: code0AppSnapshot(2, cached: true))
        let model = context.model(session: session)
        defer { model.stop() }
        session.snapshot = nil
        session.state = .disconnected
        let callback = try #require(session.onSnapshotChange, "Model must bind the independent snapshot publisher")
        callback(code0AppSnapshot(7))
        #expect(model.snapshots.allSatisfy { $0.provider != .code0 })
        #expect(session.actions.isEmpty)
    }

    @Test(arguments: [false, true])
    func allAccountResultsCannotOverwriteCurrentSessionOrPublishAccountFields(disconnected: Bool) async throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let gate = Code0AppResultGate<[ServiceAccountStatus]>()
        let session = AppCode0SessionDouble(snapshot: code0AppSnapshot(2, cached: true))
        let model = context.model(session: session, accounts: { _ in await gate.wait() })
        defer { model.stop() }
        let task = Task { @MainActor in await model.refreshServiceAccounts() }
        let started = await code0AppWait { await gate.isWaiting }
        let current = disconnected ? nil : code0AppSnapshot(8, cached: true)
        session.emit(current)
        await gate.finish([
            .init(provider: .claude, connectionState: .connected, accountLabel: "synthetic-four-service"),
            .init(provider: .code0, connectionState: .connected, accountLabel: "ordinary-account-must-not-leak"),
        ])
        await task.value
        #expect(started)
        #expect(model.snapshots.filter { $0.provider == .code0 } == current.map { [$0] } ?? [])
        #expect(model.serviceAccounts[.code0]?.accountLabel == nil)
        #expect(model.serviceAccounts[.code0]?.connectionState != .connected)
        #expect(model.serviceAccounts[.claude]?.accountLabel == "synthetic-four-service")
        #expect(session.actions.isEmpty)
    }

    @Test func singleCode0AccountCheckNeverInvokesOrdinaryReader() async throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let reads = Code0AppReadRecorder()
        let session = AppCode0SessionDouble(snapshot: code0AppSnapshot(3, cached: true))
        let model = context.model(session: session, accounts: { provider in
            await reads.record(provider)
            return [.init(provider: .code0, connectionState: .connected, accountLabel: "ordinary-pollution")]
        })
        defer { model.stop() }
        let status = await model.checkServiceAccount(.code0)
        #expect(await reads.providers.isEmpty)
        #expect(status.connectionState == .lastKnown)
        #expect(status.accountLabel == nil)
        #expect(session.actions.isEmpty)
    }

    @Test func explicitActionRoutesOnceAndDisplayOrPreferencesNeverAuthorize() throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let session = AppCode0SessionDouble(snapshot: code0AppSnapshot(2, cached: true))
        let model = context.model(session: session)
        defer { model.stop() }
        _ = model.presentations
        _ = model.code0SessionState
        var preferences = model.stripPreferences
        preferences.hiddenProviders.removeAll { $0 == .code0 }
        model.setStripPreferences(preferences)
        model.setAppLanguage(.simplifiedChinese)
        model.setDeepSeekBalanceBaseline(200)
        #expect(session.actions.isEmpty)
        for action in Code0ServiceAction.allCases {
            #expect(model.performCode0ServiceAction(action) == .started)
        }
        #expect(session.actions == Code0ServiceAction.allCases)
    }

    @Test func demoStartAndRefreshPreserveIndependentCacheWithoutFabricatingConnection() async throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let cached = code0AppSnapshot(4, cached: true)
        let session = AppCode0SessionDouble(snapshot: cached)
        let model = context.model(session: session, demo: true)
        defer { model.stop() }
        model.start()
        await model.refresh()
        await model.refreshServiceAccounts()
        #expect(model.snapshots.filter { $0.provider == .code0 } == [cached])
        #expect(model.serviceAccounts[.code0]?.connectionState == .lastKnown)
        for provider in [UsageProvider.claude, .codex, .deepSeek] {
            #expect(model.serviceAccounts[provider]?.connectionState == .connected)
        }
        #expect(session.actions.isEmpty)
    }

    @Test func normalStartAndStopDoNotAuthorizeAndStopCallsSession() async throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let session = AppCode0SessionDouble(snapshot: code0AppSnapshot(3, cached: true))
        let reads = Code0AppReadRecorder()
        let model = context.model(session: session, accounts: { provider in await reads.record(provider); return [] })
        model.start()
        let started = await code0AppWait { await reads.count > 0 && !model.isRefreshing }
        model.stop()
        #expect(started)
        #expect(session.actions.isEmpty)
        #expect(session.stops == 1)
        #expect(!session.isAutomaticallySyncing)
    }

    @Test func stopRevokesOldCallbackAndBothLateOrdinaryPublications() async throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let refreshGate = Code0AppResultGate<[UsageSnapshot]>()
        let accountGate = Code0AppResultGate<[ServiceAccountStatus]>()
        let current = code0AppSnapshot(8, cached: true)
        let session = AppCode0SessionDouble(snapshot: current)
        let model = context.model(session: session, refresh: { await refreshGate.wait() }, accounts: { _ in await accountGate.wait() })
        let refreshTask = Task { @MainActor in await model.refresh() }
        let accountTask = Task { @MainActor in await model.refreshServiceAccounts() }
        let refreshStarted = await code0AppWait { await refreshGate.isWaiting }
        let accountStarted = await code0AppWait { await accountGate.isWaiting }
        let oldCallback = session.onSnapshotChange
        #expect(model.snapshots.filter { $0.provider == .code0 } == [current])
        model.stop()
        let stopped = model.snapshots
        session.snapshot = code0AppSnapshot(99)
        oldCallback?(session.snapshot)
        await refreshGate.finish([code0AppSnapshot(1)])
        await accountGate.finish([.init(provider: .code0, connectionState: .connected, accountLabel: "late-account")])
        await refreshTask.value
        await accountTask.value
        #expect(refreshStarted && accountStarted)
        #expect(session.stops == 1)
        #expect(model.snapshots == stopped)
        #expect(model.serviceAccounts[.code0]?.accountLabel == nil)
    }
}

@MainActor
@Observable
final class AppCode0SessionDouble: Code0WebSessionProviding {
    var state: Code0SessionState
    var snapshot: UsageSnapshot?
    var failure: Code0SessionFailure?
    var isAutomaticallySyncing = false
    var onSnapshotChange: (@MainActor (UsageSnapshot?) -> Void)?
    private(set) var actions: [Code0ServiceAction] = []
    private(set) var stops = 0
    init(snapshot: UsageSnapshot? = nil, state: Code0SessionState? = nil) {
        self.snapshot = snapshot; self.state = state ?? (snapshot == nil ? .disconnected : .paused)
    }
    func connect() -> Code0SessionActionResult { actions.append(.connect); return .started }
    func loginCompleted() -> Code0SessionActionResult { actions.append(.loginCompleted); return .started }
    func syncNow() -> Code0SessionActionResult { actions.append(.syncNow); return .started }
    func disconnect() -> Code0SessionActionResult { actions.append(.disconnect); return .started }
    func processScheduledWork() {}
    func stop() { stops += 1; isAutomaticallySyncing = false }
    func emit(_ snapshot: UsageSnapshot?) {
        self.snapshot = snapshot; state = snapshot == nil ? .disconnected : .paused
        onSnapshotChange?(snapshot)
    }
}

@MainActor
struct Code0AppTestContext {
    let suite: String
    let defaults: UserDefaults
    let root: URL
    init() throws {
        let name = "Code0App.\(UUID())"
        suite = name
        defaults = try #require(UserDefaults(suiteName: name))
        root = FileManager.default.temporaryDirectory.appendingPathComponent(name)
    }
    func clean() { defaults.removePersistentDomain(forName: suite); try? FileManager.default.removeItem(at: root) }
    func model(session: AppCode0SessionDouble, demo: Bool = false,
        refresh: @escaping @Sendable () async -> [UsageSnapshot] = { [] },
        accounts: @escaping @Sendable (UsageProvider?) async -> [ServiceAccountStatus] = { _ in [] },
        pauseReason: @escaping @Sendable () async -> GeminiPauseReason? = { nil }) -> AppModel {
        AppModel(defaults: defaults, applicationSupportDirectoryURL: root, secretStore: Code0AppSecretStore(),
            systemActionPolicy: SystemActionPolicy(environment: [:], isXCTestBundle: true), widgetSnapshotPublisher: nil,
            isDemoMode: demo, code0WebSession: session, refreshOperation: refresh,
            geminiPauseReasonOperation: pauseReason, geminiResumeInvalidationOperation: {},
            serviceAccountRefreshOperation: accounts, authenticationOpenOperation: { _ in }, installationOpenOperation: { _ in false },
            cacheDirectoryURL: root)
    }
}

func code0AppSnapshot(_ value: Double, cached: Bool = false) -> UsageSnapshot {
    UsageSnapshot(provider: .code0,
        primaryMetric: UsageMetric(label: "Balance", current: value, limit: nil, unit: .siteCredits, kind: .balance),
        secondaryMetric: UsageMetric(label: "Cumulative usage", current: value / 2, limit: nil, unit: .siteCredits, kind: .cumulativeUsage),
        fetchedAt: Date(timeIntervalSince1970: 1_793_016_000 + value), sourceVersion: "website",
        collectionStatus: cached ? .cached : .fresh)
}

struct Code0AppSecretStore: SecretStore {
    func read() throws -> String? { nil }
    func save(_ secret: String) throws {}
    func delete() throws {}
}

actor Code0AppReadRecorder {
    private(set) var providers: [UsageProvider?] = []
    var count: Int { providers.count }
    func record(_ provider: UsageProvider?) { providers.append(provider) }
}

actor Code0AppResultGate<Value: Sendable> {
    private var released: Value?
    private var waiting: [CheckedContinuation<Value, Never>] = []
    var isWaiting: Bool { !waiting.isEmpty }
    func wait() async -> Value {
        if let released { return released }
        return await withCheckedContinuation { waiting.append($0) }
    }
    func finish(_ value: Value) {
        released = value
        let continuations = waiting; waiting.removeAll()
        for continuation in continuations { continuation.resume(returning: value) }
    }
}

@MainActor
func code0AppWait(_ predicate: @MainActor () async -> Bool) async -> Bool {
    let end = ContinuousClock.now.advanced(by: .seconds(1))
    while ContinuousClock.now < end {
        if await predicate() { return true }
        do { try await Task.sleep(for: .milliseconds(5)) } catch { return false }
    }
    return await predicate()
}
