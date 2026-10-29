import AIMeterCore
import Foundation
import Observation
import Testing
@testable import AIMeterApp

@Suite("Code0 App cold composition and lifecycle adapter", .serialized)
@MainActor
struct Code0AppRuntimeTests {
    @Test func productionFactoryIsColdAndUsesOnlyTheProvidedAnchor() throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let cold = makeModel(context, production: true)
        #expect(cold.code0SessionState == .disconnected)
        #expect(!cold.isCode0AutomaticallySyncing)
        let coldLocation = try Code0StorageLocation(applicationSupportRoot: context.root)
        #expect(!FileManager.default.fileExists(atPath: coldLocation.directoryURL.path))
        #expect(!FileManager.default.fileExists(atPath: coldLocation.lockURL.path))
        cold.stop()

        let location = try Code0StorageLocation(applicationSupportRoot: context.root)
        let writer = Code0ProfileLease(location: location)
        defer { writer.release() }
        try writer.acquire()
        let store = Code0SnapshotStore(profile: writer)
        let prior = code0AppSnapshot(7, cached: true)
        try store.save(prior)
        writer.release()
        let restored = makeModel(context, production: true)
        defer { restored.stop() }
        #expect(restored.code0SessionState == .paused)
        #expect(!restored.isCode0AutomaticallySyncing)
        #expect(restored.snapshots.first { $0.provider == .code0 }?.primaryMetric?.current == 7)
        #expect(restored.snapshots.first { $0.provider == .code0 }?.fetchedAt == prior.fetchedAt)
        #expect(restored.serviceAccounts[.code0]?.connectionState == .lastKnown)
        // The independently opened real OS lease must still be available: cold
        // cache construction must not acquire it or authorize a browser.
        try writer.acquire()
        #expect(writer.isHeld)
    }

    @Test(arguments: [false, true])
    func testAndDemoDefaultsDoNotReadPersonalOrProvidedWebsiteCache(demo: Bool) throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let profile = Code0ProfileLease(location: try Code0StorageLocation(applicationSupportRoot: context.root))
        defer { profile.release() }
        try profile.acquire()
        try Code0SnapshotStore(profile: profile).save(code0AppSnapshot(7, cached: true))
        profile.release()
        let model = makeModel(context, production: demo, demo: demo)
        defer { model.stop() }
        #expect(model.code0SessionState == .disconnected)
        #expect(model.snapshots.allSatisfy { $0.provider != .code0 })
        #expect(!model.isCode0AutomaticallySyncing)
    }

    @Test func unsafeFactoryAnchorFailsUnavailableWithoutFallback() throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let sentinel = Data("ordinary-file-anchor".utf8)
        try sentinel.write(to: context.root)
        let model = makeModel(context, production: true)
        defer { model.stop() }
        #expect(model.code0StatusKey == "Code0 connection could not be prepared.")
        #expect(model.serviceAccounts[.code0]?.connectionState == .unavailable)
        #expect(model.snapshots.allSatisfy { $0.provider != .code0 })
        #expect(try Data(contentsOf: context.root) == sentinel)
    }

    @Test func independentTickDoesNotStartOnDisplayAndCancelledWaitCannotWorkAfterStop() async throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let sleeper = Code0RuntimeTickGate()
        let session = Code0RuntimeSession(snapshot: code0AppSnapshot(2, cached: true))
        let model = makeModel(context, session: session, sleep: { _ in await sleeper.wait() })
        _ = model.snapshots; _ = model.code0SessionState; _ = model.serviceAccounts
        #expect(await sleeper.entries == 0)
        #expect(session.actions.isEmpty)
        #expect(session.ticks == 0)
        do {
            #expect(model.performCode0ServiceAction(.connect) == .started)
            let firstWait = await code0AppWait { await sleeper.entries == 1 }
            await sleeper.releaseAll()
            let nextWait = await code0AppWait { await sleeper.entries == 2 }
            #expect(firstWait && nextWait)
            try #require(nextWait)
            #expect(session.ticks == 1)
            let actualTickTask = try #require(model.currentCode0TickTaskForTesting)
            model.stop()
            let stoppedTicks = session.ticks
            await sleeper.releaseAll()
            var joinedTickTask = false
            let waiter = Task { @MainActor in
                await actualTickTask.value
                joinedTickTask = true
            }
            let completedTickTask = await code0AppWait { joinedTickTask }
            waiter.cancel()
            try #require(completedTickTask)
            let exitedSleep = await code0AppWait { await sleeper.exits == 2 }
            #expect(exitedSleep)
            #expect(session.stops == 1)
            #expect(session.ticks == stoppedTicks)
            #expect(session.actions == [.connect])
        } catch {
            model.stop()
            await sleeper.releaseAll()
            throw error
        }
        model.stop()
        await sleeper.releaseAll()
    }

    @Test func restartRebindsColdSessionAndOldCallbackCannotPublish() throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let old = Code0RuntimeSession(snapshot: code0AppSnapshot(2, cached: true))
        let next = Code0RuntimeSession(snapshot: code0AppSnapshot(8, cached: true))
        var constructions = 0
        let model = makeModel(context, demo: true, factory: {
            constructions += 1
            return constructions == 1 ? old : next
        })
        defer { model.stop() }
        let callback = old.onSnapshotChange
        model.stop()
        old.emit(code0AppSnapshot(99))
        #expect(model.snapshots.first { $0.provider == .code0 }?.primaryMetric?.current == 2)
        model.start()
        #expect(constructions == 2)
        #expect(model.code0SessionState == .paused)
        #expect(!model.isCode0AutomaticallySyncing)
        callback?(code0AppSnapshot(99))
        #expect(model.snapshots.first { $0.provider == .code0 }?.primaryMetric?.current == 8)
        #expect(old.stops == 1)
        #expect(next.actions.isEmpty)
    }

    @Test func unconfirmedCloseBlocksReplacementUntilTheOldSessionIsSafe() async throws {
        let context = try Code0AppTestContext()
        defer { context.clean() }
        let sleeper = Code0RuntimeTickGate()
        let old = Code0RuntimeSession(snapshot: code0AppSnapshot(2, cached: true))
        old.stayClosingOnStop = true
        let next = Code0RuntimeSession(snapshot: code0AppSnapshot(8, cached: true))
        var constructions = 0
        let model = makeModel(context, demo: true, factory: {
            constructions += 1; return constructions == 1 ? old : next
        }, sleep: { _ in await sleeper.wait() })
        model.stop()
        let waitingForCleanup = await code0AppWait { await sleeper.entries == 1 }
        model.start()
        #expect(waitingForCleanup)
        #expect(constructions == 1)
        #expect(!model.canPerformCode0Action(.connect))
        #expect(model.performCode0ServiceAction(.connect) == .denied)
        old.state = .paused // This double represents the already-tested release boundary.
        model.start()
        #expect(constructions == 2)
        await sleeper.releaseAll()
        model.stop()
    }

    private func makeModel(_ context: Code0AppTestContext, production: Bool = false, demo: Bool = false,
        session: (any Code0WebSessionProviding)? = nil,
        factory: (@MainActor () throws -> any Code0WebSessionProviding)? = nil,
        sleep: @escaping @Sendable (Duration) async throws -> Void = { try await Task.sleep(for: $0) }) -> AppModel {
        AppModel(defaults: context.defaults, applicationSupportDirectoryURL: context.root, secretStore: Code0AppSecretStore(),
            systemActionPolicy: SystemActionPolicy(environment: [:], isXCTestBundle: !production), widgetSnapshotPublisher: nil,
            isDemoMode: demo, code0WebSession: session, code0SessionFactory: factory, code0TickSleep: sleep,
            refreshOperation: { [] }, geminiPauseReasonOperation: { nil }, geminiResumeInvalidationOperation: {},
            serviceAccountRefreshOperation: { _ in [] }, cacheDirectoryURL: context.root)
    }
}

@MainActor
@Observable
private final class Code0RuntimeSession: Code0WebSessionProviding {
    var state: Code0SessionState = .paused
    var snapshot: UsageSnapshot?
    var failure: Code0SessionFailure?
    var isAutomaticallySyncing = false
    var onSnapshotChange: (@MainActor (UsageSnapshot?) -> Void)?
    var stayClosingOnStop = false
    private(set) var actions: [Code0ServiceAction] = []
    private(set) var ticks = 0
    private(set) var stops = 0
    init(snapshot: UsageSnapshot?) { self.snapshot = snapshot }
    func connect() -> Code0SessionActionResult { actions.append(.connect); state = .connecting; return .started }
    func loginCompleted() -> Code0SessionActionResult { actions.append(.loginCompleted); return .started }
    func syncNow() -> Code0SessionActionResult { actions.append(.syncNow); return .started }
    func disconnect() -> Code0SessionActionResult { actions.append(.disconnect); return .started }
    func processScheduledWork() { ticks += 1 }
    func stop() { stops += 1; isAutomaticallySyncing = false; state = stayClosingOnStop ? .closing : .paused }
    func emit(_ value: UsageSnapshot?) { snapshot = value; onSnapshotChange?(value) }
}

private actor Code0RuntimeTickGate {
    private(set) var entries = 0
    private(set) var exits = 0
    private var waiting: [CheckedContinuation<Void, Never>] = []
    func wait() async {
        entries += 1
        await withCheckedContinuation { waiting.append($0) }
        exits += 1
    }
    func releaseAll() {
        let continuations = waiting; waiting.removeAll()
        for continuation in continuations { continuation.resume() }
    }
}
