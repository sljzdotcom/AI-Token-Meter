import Foundation
import Testing
@testable import AIMeterApp
@testable import AIMeterCore

@Suite("Code0 explicit connection lifecycle", .serialized)
@MainActor
struct Code0WebSessionTests {
    // A constructor, a timer tick, or a global refresh must not grant website access.
    @Test func disconnectedSessionAndScheduledWorkCreateNoWindow() {
        let fixture = SessionFixture()
        fixture.clock.advance(10_000)
        fixture.session.processScheduledWork()
        #expect(fixture.session.state == .disconnected)
        #expect(!fixture.session.isAutomaticallySyncing)
        #expect(fixture.driver.requests.isEmpty)
        #expect(fixture.driver.leaseAcquisitions == 0)
        #expect(fixture.store.saved.isEmpty)
    }

    // Catches reuse of the login document, pre-save publication, or early authorization.
    @Test func explicitLoginClosesBeforeFreshSyncAndDurablePublishPrecedesActive() throws {
        let fixture = SessionFixture()
        #expect(fixture.session.connect() == .started)
        let login = try #require(fixture.driver.requests.first)
        #expect(login.mode == .login)
        #expect(fixture.session.state == .connecting)
        #expect(fixture.driver.navigated.isEmpty)
        fixture.driver.emit(.policiesInstalled, for: login)
        #expect(fixture.driver.navigated == [login.instanceID])
        #expect(fixture.session.loginCompleted() == .started)
        #expect(fixture.session.state == .closing)
        #expect(fixture.driver.requests.count == 1)
        #expect(fixture.driver.closing == [login.instanceID])
        fixture.driver.release(login)
        let sync = try #require(fixture.driver.requests.last)
        #expect(sync.mode == .sync)
        #expect(sync.instanceID != login.instanceID)
        #expect(sync.documentID != login.documentID)
        #expect(sync.nonce != login.nonce)
        #expect(sync.connectionEpoch == login.connectionEpoch)
        #expect(fixture.driver.requests.count == 2)
        #expect(fixture.session.state == .syncing)
        fixture.driver.emit(.policiesInstalled, for: sync)
        fixture.deliverCapture(to: sync)
        #expect(fixture.session.snapshot?.primaryMetric?.current == 2.5)
        #expect(fixture.session.snapshot?.secondaryMetric?.current == 1.5)
        #expect(fixture.session.snapshot?.fetchedAt == fixture.clock.wallNow)
        #expect(fixture.session.state == .closing)
        #expect(!fixture.session.isAutomaticallySyncing)
        #expect(fixture.timeline.events == [
            "lease.acquire", "create.login", "navigate.login", "close.login", "released.login",
            "create.sync", "navigate.sync", "store.save", "publish.value", "close.sync",
        ])
        fixture.driver.release(sync)
        #expect(fixture.session.state == .active)
        #expect(fixture.session.isAutomaticallySyncing)
        #expect(fixture.driver.leaseReleases == 0)
    }

    // Missing serialization must not start a second login or sync window.
    @Test func busyActionsDoNotDuplicateOwnedWindow() throws {
        let fixture = SessionFixture()
        #expect(fixture.session.connect() == .started)
        let login = try #require(fixture.driver.requests.first)
        #expect(fixture.session.connect() == .busy)
        #expect(fixture.session.syncNow() == .busy)
        fixture.driver.emit(.policiesInstalled, for: login)
        #expect(fixture.session.loginCompleted() == .started)
        #expect(fixture.session.loginCompleted() == .busy)
        fixture.session.processScheduledWork()
        #expect(fixture.driver.requests.count == 1)
        fixture.driver.release(login)
        #expect(fixture.driver.requests.count == 2)
        #expect(fixture.session.syncNow() == .busy)
        #expect(fixture.driver.requests.count == 2)
    }

    // Manual and timer actions share the 300s lower bound, measured from attempt start.
    @Test func manualAndAutomaticSyncUseTheSame299And300SecondBoundary() throws {
        let fixture = SessionFixture()
        try fixture.activate(loginDelay: 200)
        fixture.clock.advance(299)
        #expect(fixture.session.syncNow() == .throttled(secondsRemaining: 1))
        fixture.session.processScheduledWork()
        #expect(fixture.driver.requests.count == 2)
        fixture.clock.advance(1)
        fixture.session.processScheduledWork()
        #expect(fixture.driver.requests.count == 3)
        #expect(fixture.driver.requests.last?.mode == .sync)
        #expect(fixture.session.state == .syncing)
        #expect(fixture.session.syncNow() == .busy)
        #expect(fixture.driver.requests.count == 3)
    }

    @Test func intervalRunsFromStartAndSleepDoesNotReplayMissedTicks() throws {
        let fixture = SessionFixture()
        let sync = try fixture.startSync()
        fixture.clock.advance(40)
        fixture.deliverCapture(to: sync)
        fixture.driver.release(sync)
        fixture.clock.advance(259)
        fixture.session.processScheduledWork()
        #expect(fixture.driver.requests.count == 2)
        fixture.clock.advance(1)
        #expect(fixture.session.syncNow() == .started)
        #expect(fixture.driver.requests.count == 3)
        let second = try #require(fixture.driver.requests.last)
        fixture.driver.emit(.policiesInstalled, for: second)
        fixture.deliverCapture(to: second)
        fixture.driver.release(second)
        fixture.clock.advance(3_000)
        fixture.session.processScheduledWork()
        fixture.session.processScheduledWork()
        #expect(fixture.driver.requests.count == 4)
        #expect(fixture.session.state == .syncing)
    }

    // A failed atomic write cannot publish the candidate or authorize another timer attempt.
    @Test func saveFailureKeepsOldValueAndTimestampAndPausesAfterRelease() throws {
        let fixture = SessionFixture()
        try fixture.activate()
        let old = fixture.session.snapshot
        fixture.clock.advance(300)
        #expect(fixture.session.syncNow() == .started)
        let sync = try #require(fixture.driver.requests.last)
        fixture.driver.emit(.policiesInstalled, for: sync)
        fixture.store.failSave = true
        fixture.deliverCapture(to: sync, quota: 5_000_000)
        #expect(fixture.session.snapshot == old)
        #expect(fixture.published.count == 1)
        #expect(!fixture.session.isAutomaticallySyncing)
        fixture.driver.release(sync)
        #expect(fixture.session.state == .paused)
        #expect(fixture.session.failure == .storageFailed)
        fixture.clock.advance(10_000)
        fixture.session.processScheduledWork()
        #expect(fixture.driver.requests.count == 3)
    }

    // close() is a request; release is independently required before new creation.
    @Test func missingCloseConfirmationBlocksCreationAndLateReleaseDoesNotAutoResume() throws {
        let fixture = SessionFixture()
        #expect(fixture.session.connect() == .started)
        let login = try #require(fixture.driver.requests.first)
        fixture.driver.emit(.policiesInstalled, for: login)
        #expect(fixture.session.loginCompleted() == .started)
        fixture.clock.advance(9)
        fixture.session.processScheduledWork()
        #expect(fixture.session.state == .closing)
        fixture.clock.advance(1)
        fixture.session.processScheduledWork()
        #expect(fixture.session.state == .cleanupPending)
        #expect(fixture.session.syncNow() == .busy)
        #expect(fixture.session.connect() == .busy)
        #expect(fixture.driver.requests.count == 1)
        fixture.driver.release(login)
        #expect(fixture.driver.requests.count == 1)
        #expect(!fixture.session.isAutomaticallySyncing)
    }

    @Test func successfulSaveWithoutReleaseDoesNotAuthorizeAutomaticSync() throws {
        let fixture = SessionFixture()
        let sync = try fixture.startSync()
        fixture.deliverCapture(to: sync)
        #expect(fixture.store.saved.count == 1)
        fixture.clock.advance(10)
        fixture.session.processScheduledWork()
        #expect(fixture.session.state == .cleanupPending)
        #expect(!fixture.session.isAutomaticallySyncing)
        fixture.clock.advance(1_000)
        fixture.session.processScheduledWork()
        #expect(fixture.driver.requests.count == 2)
    }

    @Test func loginAndHiddenAttemptTimeoutsPauseWithoutRetry() throws {
        let loginFixture = SessionFixture()
        #expect(loginFixture.session.connect() == .started)
        let login = try #require(loginFixture.driver.requests.first)
        loginFixture.clock.advance(599)
        loginFixture.session.processScheduledWork()
        #expect(loginFixture.driver.closing.isEmpty)
        loginFixture.clock.advance(1)
        loginFixture.session.processScheduledWork()
        #expect(loginFixture.driver.closing == [login.instanceID])
        loginFixture.driver.release(login)
        #expect(loginFixture.session.state == .paused)
        #expect(loginFixture.session.failure == .loginTimedOut)
        #expect(loginFixture.driver.requests.count == 1)

        let syncFixture = SessionFixture()
        let sync = try syncFixture.startSync()
        syncFixture.clock.advance(44)
        syncFixture.session.processScheduledWork()
        #expect(syncFixture.driver.closing.count == 1) // the released login only
        syncFixture.clock.advance(1)
        syncFixture.session.processScheduledWork()
        #expect(syncFixture.driver.closing.last == sync.instanceID)
        syncFixture.driver.release(sync)
        #expect(syncFixture.session.state == .paused)
        #expect(syncFixture.session.failure == .syncTimedOut)
        syncFixture.clock.advance(1_000)
        syncFixture.session.processScheduledWork()
        #expect(syncFixture.driver.requests.count == 2)
    }

    // Revocation occurs at Disconnect intent, before the asynchronous release callback.
    @Test func disconnectRejectsLateCaptureAndClearsOnlyOwnedDataAfterRelease() throws {
        let fixture = SessionFixture()
        try fixture.activate()
        fixture.clock.advance(300)
        #expect(fixture.session.syncNow() == .started)
        let sync = try #require(fixture.driver.requests.last)
        fixture.driver.emit(.policiesInstalled, for: sync)
        #expect(fixture.session.disconnect() == .started)
        #expect(fixture.session.snapshot == nil)
        fixture.deliverCapture(to: sync, quota: 5_000_000)
        #expect(fixture.store.saved.count == 1)
        #expect(fixture.driver.websiteClears == 0)
        #expect(fixture.session.connect() == .busy)
        fixture.driver.release(sync)
        #expect(fixture.driver.websiteClears == 1)
        #expect(fixture.store.removals == 1)
        #expect(fixture.store.cached == nil)
        #expect(fixture.driver.leaseReleases == 1)
        #expect(fixture.session.state == .disconnected)
        fixture.deliverCapture(to: sync)
        #expect(fixture.session.snapshot == nil)
        #expect(fixture.store.saved.count == 1)
    }

    @Test func websiteCleanupFailureKeepsLeaseAndBlocksReconnect() throws {
        let fixture = SessionFixture()
        try fixture.activate()
        fixture.driver.failWebsiteCleanup = true
        #expect(fixture.session.disconnect() == .started)
        #expect(fixture.session.state == .cleanupPending)
        #expect(fixture.driver.leaseReleases == 0)
        #expect(fixture.session.connect() == .busy)
        fixture.clock.advance(1_000)
        fixture.session.processScheduledWork()
        #expect(fixture.driver.requests.count == 2)
    }

    @Test func cachedRestartHasNoAutomaticAuthorizationButExplicitResumeCanValidate() throws {
        let clock = TestSessionClock()
        let store = MemoryCode0Store(timeline: SessionTimeline())
        store.cached = UsageSnapshot(
            provider: .code0,
            primaryMetric: UsageMetric(label: "Balance", current: 2.5, limit: nil, unit: .siteCredits, kind: .balance),
            secondaryMetric: UsageMetric(label: "Cumulative usage", current: 1.5, limit: nil, unit: .siteCredits, kind: .cumulativeUsage),
            fetchedAt: Date(timeIntervalSince1970: 1_793_016_000), staleAfter: 300,
            sourceVersion: "website", collectionStatus: .fresh
        )
        let driver = TestCode0Driver(timeline: SessionTimeline())
        let session = Code0WebSession(clock: clock, driver: driver, store: store)
        #expect(session.state == .paused)
        #expect(session.snapshot?.collectionStatus == .cached)
        #expect(session.snapshot?.primaryMetric?.current == 2.5)
        #expect(session.snapshot?.fetchedAt == Date(timeIntervalSince1970: 1_793_016_000))
        #expect(!session.isAutomaticallySyncing)
        clock.advance(10_000)
        session.processScheduledWork()
        #expect(driver.requests.isEmpty)
        #expect(driver.leaseAcquisitions == 0)
        #expect(session.syncNow() == .started)
        #expect(driver.requests.map(\.mode) == [.sync])
        #expect(!session.isAutomaticallySyncing)
    }

    // Transport failure retains old data; observed auth change immediately clears it.
    @Test func authenticationInvalidationClearsOldAccountWhileTransportFailureKeepsIt() throws {
        for authChanged in [false, true] {
            let fixture = SessionFixture()
            try fixture.activate()
            let old = fixture.session.snapshot
            fixture.clock.advance(300)
            #expect(fixture.session.syncNow() == .started)
            let sync = try #require(fixture.driver.requests.last)
            fixture.driver.emit(.policiesInstalled, for: sync)
            fixture.send(to: sync, sequence: 1, body: "\"kind\":\"ready\"")
            fixture.send(to: sync, sequence: 2, body: authChanged
                ? "\"kind\":\"invalidate\",\"reason\":\"authenticationChanged\""
                : "\"kind\":\"failure\",\"reason\":\"invalidResponse\"")
            #expect(fixture.session.snapshot == (authChanged ? nil : old))
            #expect(fixture.store.removals == (authChanged ? 1 : 0))
            fixture.driver.release(sync)
            #expect(fixture.session.state == .paused)
            #expect(!fixture.session.isAutomaticallySyncing)
            #expect(fixture.store.saved.count == 1)
        }
    }

    @Test func wrongNativeOriginCannotSaveOrPublishCapture() throws {
        let fixture = SessionFixture()
        let sync = try fixture.startSync()
        fixture.deliverCapture(to: sync, source: "https://example.invalid/")
        #expect(fixture.store.saved.isEmpty)
        #expect(fixture.session.snapshot == nil)
        fixture.driver.release(sync)
        #expect(fixture.session.state == .paused)
        #expect(!fixture.session.isAutomaticallySyncing)
    }

    @Test func profileBusyDoesNotCreateOrClearData() {
        let fixture = SessionFixture()
        fixture.driver.failLease = true
        #expect(fixture.session.connect() == .denied)
        #expect(fixture.session.failure == .profileBusy)
        #expect(fixture.driver.requests.isEmpty)
        #expect(fixture.driver.websiteClears == 0)
        #expect(fixture.store.removals == 0)
    }

    @Test func stopRevokesCallbacksWithoutDeletingCachedValuesOrWebsiteData() throws {
        let fixture = SessionFixture()
        try fixture.activate()
        fixture.clock.advance(300)
        #expect(fixture.session.syncNow() == .started)
        let sync = try #require(fixture.driver.requests.last)
        fixture.driver.emit(.policiesInstalled, for: sync)
        fixture.session.stop()
        fixture.deliverCapture(to: sync)
        #expect(fixture.store.saved.count == 1)
        #expect(!fixture.session.isAutomaticallySyncing)
        fixture.driver.release(sync)
        #expect(fixture.driver.leaseReleases == 1)
        #expect(fixture.store.removals == 0)
        #expect(fixture.driver.websiteClears == 0)
        fixture.clock.advance(1_000)
        fixture.session.processScheduledWork()
        #expect(fixture.driver.requests.count == 3)
    }
}

@MainActor
private final class SessionFixture {
    let timeline: SessionTimeline
    let clock: TestSessionClock
    let driver: TestCode0Driver
    let store: MemoryCode0Store
    let session: Code0WebSession
    var published: [UsageSnapshot?] = []

    init() {
        let timeline = SessionTimeline()
        let clock = TestSessionClock()
        let driver = TestCode0Driver(timeline: timeline)
        let store = MemoryCode0Store(timeline: timeline)
        self.timeline = timeline
        self.clock = clock
        self.driver = driver
        self.store = store
        self.session = Code0WebSession(clock: clock, driver: driver, store: store)
        session.onSnapshotChange = { [weak self] value in
            guard let self else { return }
            self.timeline.events.append(value == nil ? "publish.empty" : "publish.value")
            self.published.append(value)
        }
    }

    func startSync(loginDelay: Int = 0) throws -> Code0WindowRequest {
        try #require(session.connect() == .started, "Explicit Connect must start the owned login lifecycle")
        let login = try #require(driver.requests.first, "Connect must create one unloaded login instance")
        driver.emit(.policiesInstalled, for: login)
        clock.advance(loginDelay)
        try #require(session.loginCompleted() == .started)
        driver.release(login)
        let sync = try #require(driver.requests.last)
        try #require(sync.mode == .sync, "Login release must precede fresh hidden validation")
        driver.emit(.policiesInstalled, for: sync)
        return sync
    }

    func activate(loginDelay: Int = 0) throws {
        let sync = try startSync(loginDelay: loginDelay)
        deliverCapture(to: sync)
        driver.release(sync)
        try #require(session.state == .active)
    }

    func deliverCapture(to request: Code0WindowRequest, quota: Int = 1_250_000, source: String = "https://console.code0.ai/console") {
        send(to: request, sequence: 1, body: "\"kind\":\"ready\"", source: source)
        send(to: request, sequence: 2, body: "\"kind\":\"selfStart\",\"request\":1,\"revision\":0", source: source)
        send(to: request, sequence: 3, body: "\"kind\":\"capture\",\"request\":1,\"revision\":0,\"quota\":\(quota),\"usedQuota\":750000,\"divisor\":500000", source: source)
    }

    func send(to request: Code0WindowRequest, sequence: Int, body: String, source: String = "https://console.code0.ai/console") {
        let payload = Data("{\"version\":1,\"nonce\":\"\(request.nonce.uuidString)\",\"document\":\"\(request.documentID.uuidString)\",\"sequence\":\(sequence),\(body)}".utf8)
        driver.emit(.message(payload: payload, sourceURL: source, isMainFrame: true), for: request)
    }
}

@MainActor
private final class SessionTimeline { var events: [String] = [] }

@MainActor
private final class TestSessionClock: Code0SessionClock {
    private var instant = ContinuousClock.now
    private var elapsed: TimeInterval = 0
    var now: ContinuousClock.Instant { instant }
    var wallNow: Date { Date(timeIntervalSince1970: 1_793_016_000 + elapsed) }
    func advance(_ seconds: Int) {
        instant = instant.advanced(by: .seconds(seconds))
        elapsed += Double(seconds)
    }
}

@MainActor
private final class MemoryCode0Store: Code0SnapshotStoring {
    let timeline: SessionTimeline
    var cached: UsageSnapshot?
    var saved: [UsageSnapshot] = []
    var removals = 0
    var failSave = false
    init(timeline: SessionTimeline) { self.timeline = timeline }
    func load() throws -> UsageSnapshot? { cached }
    func save(_ snapshot: UsageSnapshot) throws {
        timeline.events.append("store.save")
        if failSave { throw Code0SnapshotStoreError.persistenceFailed }
        saved.append(snapshot)
        cached = snapshot
    }
    func remove() throws {
        timeline.events.append("store.remove")
        removals += 1
        cached = nil
    }
}

@MainActor
private final class TestCode0Driver: Code0WindowDriving {
    let timeline: SessionTimeline
    var requests: [Code0WindowRequest] = []
    var navigated: [UUID] = []
    var closing: [UUID] = []
    var leaseAcquisitions = 0
    var leaseReleases = 0
    var websiteClears = 0
    var failLease = false
    var failWebsiteCleanup = false
    private var owned: Set<UUID> = []
    private var handlers: [UUID: @MainActor (Code0WindowEvent) -> Void] = [:]
    init(timeline: SessionTimeline) { self.timeline = timeline }
    func acquireProfileLease() throws {
        leaseAcquisitions += 1
        if failLease { throw Code0SessionFailure.profileBusy }
        timeline.events.append("lease.acquire")
    }
    func releaseProfileLease() { leaseReleases += 1; timeline.events.append("lease.release") }
    func createUnloaded(_ request: Code0WindowRequest, events: @escaping @MainActor (Code0WindowEvent) -> Void) throws {
        // A real driver cannot create the next object while the prior one is retained.
        guard owned.isEmpty else { throw Code0SessionFailure.cleanupFailed }
        owned.insert(request.instanceID)
        handlers[request.instanceID] = events
        requests.append(request)
        timeline.events.append("create.\(request.mode.rawValue)")
    }
    func navigate(_ instanceID: UUID) throws {
        navigated.append(instanceID)
        if let request = requests.first(where: { $0.instanceID == instanceID }) {
            timeline.events.append("navigate.\(request.mode.rawValue)")
        }
    }
    func close(_ instanceID: UUID) {
        closing.append(instanceID)
        if let request = requests.first(where: { $0.instanceID == instanceID }) {
            timeline.events.append("close.\(request.mode.rawValue)")
        }
    }
    func clearWebsiteData(completion: @escaping @MainActor (Result<Void, Code0SessionFailure>) -> Void) {
        websiteClears += 1
        timeline.events.append("website.clear")
        completion(failWebsiteCleanup ? .failure(.cleanupFailed) : .success(()))
    }
    func emit(_ event: Code0WindowEvent, for request: Code0WindowRequest) { handlers[request.instanceID]?(event) }
    func release(_ request: Code0WindowRequest) {
        owned.remove(request.instanceID)
        timeline.events.append("released.\(request.mode.rawValue)")
        emit(.released, for: request)
    }
}
