import AIMeterCore
import Foundation
import Observation

enum Code0SessionState: Equatable, Sendable {
    case disconnected, connecting, syncing, closing, active, paused, cleanupPending
}

enum Code0SessionFailure: Error, Equatable, Sendable {
    case validationRequired
    case profileBusy
    case storageFailed
    case cleanupFailed
    case policyRejected
    case navigationFailed
    case invalidResponse
    case authenticationChanged
    case loginTimedOut
    case syncTimedOut
}

enum Code0SessionActionResult: Equatable, Sendable {
    case started
    case busy
    case throttled(secondsRemaining: Int)
    case denied
}

enum Code0WindowMode: String, Equatable, Sendable {
    case login, sync
}

/// Created by native for exactly one owned document. Page JSON cannot supply
/// the epoch, instance or validator configuration.
struct Code0WindowRequest: Equatable, Sendable {
    let instanceID: UUID
    let connectionEpoch: UUID
    let nonce: UUID
    let documentID: UUID
    let mode: Code0WindowMode
}

enum Code0WindowEvent: Sendable {
    /// All refusal delegates and the document-start bridge are installed.
    case policiesInstalled
    case message(payload: Data, sourceURL: String, isMainFrame: Bool)
    /// Proof of release of this owned instance, not a close/hide notification.
    case released
    case failed(Code0SessionFailure)
}

@MainActor
protocol Code0SessionClock {
    var now: ContinuousClock.Instant { get }
    var wallNow: Date { get }
}

/// Slow native operations report completion through events. createUnloaded must
/// not navigate; close must revoke callbacks/detach/stop before requesting release.
@MainActor
protocol Code0WindowDriving: AnyObject {
    func acquireProfileLease() throws
    func releaseProfileLease()
    func createUnloaded(_ request: Code0WindowRequest, events: @escaping @MainActor (Code0WindowEvent) -> Void) throws
    func navigate(_ instanceID: UUID) throws
    func close(_ instanceID: UUID)
    func clearWebsiteData(completion: @escaping @MainActor (Result<Void, Code0SessionFailure>) -> Void)
}

@MainActor
protocol Code0WebSessionProviding: AnyObject {
    var state: Code0SessionState { get }
    var snapshot: UsageSnapshot? { get }
    var failure: Code0SessionFailure? { get }
    var isAutomaticallySyncing: Bool { get }
    var onSnapshotChange: (@MainActor (UsageSnapshot?) -> Void)? { get set }
    @discardableResult func connect() -> Code0SessionActionResult
    @discardableResult func loginCompleted() -> Code0SessionActionResult
    @discardableResult func syncNow() -> Code0SessionActionResult
    @discardableResult func disconnect() -> Code0SessionActionResult
    func processScheduledWork()
    func stop()
}

/// Pure session ownership and scheduling. The driver supplies native release
/// proof; no timer, browser or filesystem implementation is constructed here.
@MainActor
@Observable
final class Code0WebSession: Code0WebSessionProviding {
    private(set) var state: Code0SessionState = .disconnected
    private(set) var snapshot: UsageSnapshot?
    private(set) var failure: Code0SessionFailure?
    private(set) var isAutomaticallySyncing = false
    @ObservationIgnored var onSnapshotChange: (@MainActor (UsageSnapshot?) -> Void)?

    private let clock: any Code0SessionClock
    private let driver: any Code0WindowDriving
    private let store: any Code0SnapshotStoring
    private var epoch = UUID()
    private var owned: Code0WindowRequest?
    private var validator: Code0BridgeValidator?
    private var policiesInstalled = false
    private var closeRequested = false
    private var leaseHeld = false
    private var hasConnection = false
    // A failed/invalid read does not prove the persistent account cache is absent.
    private var cacheMayExist = false
    private var stopped = false
    private var lastSyncStart: ContinuousClock.Instant?
    private var attemptDeadline: ContinuousClock.Instant?
    private var closeDeadline: ContinuousClock.Instant?
    private var closePlan: ClosePlan?
    private var cleanupID: UUID?
    @ObservationIgnored private var mutationDepth = 0
    @ObservationIgnored private var events: [PendingEvent] = []

    private enum ClosePlan: Equatable { case beginSync, activate, pause, disconnect, stop, cleanupFailure }
    private enum PendingEvent {
        case window(Code0WindowEvent, Code0WindowRequest)
        case cleanup(Result<Void, Code0SessionFailure>, UUID)
    }

    init(clock: any Code0SessionClock, driver: any Code0WindowDriving, store: any Code0SnapshotStoring) {
        self.clock = clock
        self.driver = driver
        self.store = store
        do {
            if let cached = try store.load() {
                cacheMayExist = true
                guard Self.validCache(cached) else {
                    state = .paused
                    failure = .storageFailed
                    return
                }
                snapshot = UsageSnapshot(
                    provider: .code0, primaryMetric: cached.primaryMetric, secondaryMetric: cached.secondaryMetric,
                    fetchedAt: cached.fetchedAt, staleAfter: 300, sourceVersion: "website", collectionStatus: .cached)
                hasConnection = true
                state = .paused
            }
        } catch {
            cacheMayExist = true
            state = .paused
            failure = .storageFailed
        }
    }

    @discardableResult
    func connect() -> Code0SessionActionResult {
        mutate {
            guard !stopped else { return .denied }
            guard !isBusy else { return .busy }
            state = .connecting
            failure = nil
            epoch = UUID()
            isAutomaticallySyncing = false
            guard acquireLease() else { return .denied }
            // A new explicit connection is an account boundary, unlike a retry.
            if cacheMayExist {
                snapshot = nil
                let currentEpoch = epoch
                do { try invalidateCache() }
                catch {
                    failCleanup(.storageFailed)
                    onSnapshotChange?(nil)
                    return .denied
                }
                onSnapshotChange?(nil)
                guard epoch == currentEpoch, !stopped, state == .connecting else { return .denied }
            }
            hasConnection = true
            lastSyncStart = nil
            beginWindow(.login)
            return .started
        }
    }

    @discardableResult
    func loginCompleted() -> Code0SessionActionResult {
        mutate {
            guard !stopped else { return .denied }
            guard state == .connecting, owned?.mode == .login, policiesInstalled else {
                return isBusy ? .busy : .denied
            }
            if let attemptDeadline, clock.now >= attemptDeadline {
                pause(.loginTimedOut)
                return .denied
            }
            beginClose(.beginSync)
            return .started
        }
    }

    @discardableResult
    func syncNow() -> Code0SessionActionResult {
        mutate {
            guard !stopped else { return .denied }
            guard !isBusy else { return .busy }
            guard hasConnection, state == .active || state == .paused else { return .denied }
            if let seconds = remainingInterval { return .throttled(secondsRemaining: seconds) }
            state = .syncing
            guard acquireLease() else { return .denied }
            failure = nil
            beginWindow(.sync)
            return .started
        }
    }

    @discardableResult
    func disconnect() -> Code0SessionActionResult {
        mutate {
            guard !stopped else { return .denied }
            guard cleanupID == nil, state != .cleanupPending, closePlan != .disconnect else { return .busy }
            guard hasConnection || leaseHeld || owned != nil else { return .denied }
            // A cache restored after process restart has no lease yet.
            state = .closing
            guard acquireLease() else { return .denied }
            epoch = UUID() // Revoke capture at intent, before close/release callbacks.
            validator = nil
            isAutomaticallySyncing = false
            failure = nil
            snapshot = nil
            closePlan = .disconnect
            state = .closing
            onSnapshotChange?(nil)
            beginClose(.disconnect)
            return .started
        }
    }

    func processScheduledWork() {
        mutate {
            if let closeDeadline, owned != nil, clock.now >= closeDeadline {
                self.closeDeadline = nil
                failure = .cleanupFailed
                state = .cleanupPending
                isAutomaticallySyncing = false
                if closePlan == .beginSync || closePlan == .activate { closePlan = .pause }
                return
            }
            guard !stopped else { return }
            if let attemptDeadline, clock.now >= attemptDeadline,
               state == .connecting || state == .syncing {
                pause(owned?.mode == .login ? .loginTimedOut : .syncTimedOut)
                return
            }
            if state == .active, isAutomaticallySyncing, owned == nil, remainingInterval == nil {
                failure = nil
                beginWindow(.sync)
            }
        }
    }

    func stop() {
        mutate {
            guard !stopped else { return }
            stopped = true
            epoch = UUID()
            validator = nil
            isAutomaticallySyncing = false
            // Stopping cannot turn an unfinished disconnect/cleanup into success.
            guard state != .cleanupPending, closePlan != .disconnect,
                  closePlan != .cleanupFailure else { return }
            beginClose(.stop)
        }
    }

    private var isBusy: Bool {
        owned != nil || cleanupID != nil || state == .connecting || state == .syncing ||
            state == .closing || state == .cleanupPending
    }

    private var remainingInterval: Int? {
        guard let lastSyncStart else { return nil }
        let deadline = lastSyncStart.advanced(by: .seconds(300))
        let now = clock.now
        guard now < deadline else { return nil }
        let parts = now.duration(to: deadline).components
        return Int(parts.seconds) + (parts.attoseconds > 0 ? 1 : 0)
    }

    private func acquireLease() -> Bool {
        guard !leaseHeld else { return true }
        do {
            try driver.acquireProfileLease()
            leaseHeld = true
            return true
        } catch {
            failure = .profileBusy
            state = .paused
            isAutomaticallySyncing = false
            return false
        }
    }

    private func beginWindow(_ mode: Code0WindowMode) {
        guard !stopped, leaseHeld, owned == nil, cleanupID == nil else { return }
        let request = Code0WindowRequest(instanceID: UUID(), connectionEpoch: epoch,
                                        nonce: UUID(), documentID: UUID(), mode: mode)
        owned = request
        policiesInstalled = false
        closeRequested = false
        closePlan = nil
        closeDeadline = nil
        isAutomaticallySyncing = false
        state = mode == .login ? .connecting : .syncing
        let startedAt = clock.now
        attemptDeadline = startedAt.advanced(by: .seconds(mode == .login ? 600 : 45))
        if mode == .sync {
            lastSyncStart = startedAt
            validator = Code0BridgeValidator(connectionEpoch: epoch, attemptID: request.instanceID,
                                             nonce: request.nonce, documentID: request.documentID)
        } else { validator = nil }
        do {
            try driver.createUnloaded(request) { [weak self] event in
                self?.enqueue(.window(event, request))
            }
        } catch {
            // A throwing creation may still retain a partially-created object.
            // Only its release callback can prove it is safe to create again.
            pause(.navigationFailed)
        }
    }

    private func handle(_ event: Code0WindowEvent, request: Code0WindowRequest) {
        guard owned?.instanceID == request.instanceID else { return }
        if case .released = event {
            // Deadlines remain authoritative even if a timer tick was delayed.
            if let closeDeadline, clock.now >= closeDeadline {
                failure = .cleanupFailed
                if closePlan == .beginSync || closePlan == .activate { closePlan = .pause }
            }
            finishRelease()
            return
        }
        guard request.connectionEpoch == epoch, !stopped else { return }
        if let attemptDeadline, clock.now >= attemptDeadline,
           state == .connecting || state == .syncing {
            pause(request.mode == .login ? .loginTimedOut : .syncTimedOut)
            return
        }
        switch event {
        case .policiesInstalled:
            guard !policiesInstalled, state == .connecting || state == .syncing else { return }
            policiesInstalled = true
            do { try driver.navigate(request.instanceID) }
            catch {
                if request.connectionEpoch == epoch, !stopped,
                   state == .connecting || state == .syncing { pause(.navigationFailed) }
            }
        case .message(let payload, let sourceURL, let isMainFrame):
            guard state == .syncing, policiesInstalled, var current = validator else { return }
            let event = current.receive(payload, context: Code0BridgeContext(
                connectionEpoch: request.connectionEpoch, attemptID: request.instanceID,
                sourceURL: sourceURL, isMainFrame: isMainFrame))
            validator = current
            switch event {
            case .control: break
            case .capture(let capture): accept(capture, request: request)
            case .invalidated:
                snapshot = nil
                validator = nil
                isAutomaticallySyncing = false
                failure = .authenticationChanged
                let currentEpoch = epoch
                do { try invalidateCache() }
                catch {
                    if epoch == currentEpoch {
                        failure = .storageFailed
                        beginClose(.cleanupFailure)
                    }
                    onSnapshotChange?(nil)
                    return
                }
                onSnapshotChange?(nil)
                if epoch == currentEpoch, !stopped, state == .syncing { beginClose(.pause) }
            case .failure, .rejected: pause(.invalidResponse)
            }
        case .failed(let reason):
            if state == .closing {
                if closePlan == .beginSync || closePlan == .activate {
                    closePlan = .pause
                    failure = reason
                }
                if reason == .cleanupFailed {
                    failure = .cleanupFailed
                    state = .cleanupPending
                }
                return
            }
            guard state == .connecting || state == .syncing else { return }
            pause(reason)
        case .released: break // handled before the revoked-epoch guard
        }
    }

    private func accept(_ capture: Code0WebsiteCapture, request: Code0WindowRequest) {
        do {
            let candidate = try capture.makeSnapshot(fetchedAt: clock.wallNow)
            cacheMayExist = true
            try store.save(candidate)
            guard owned?.instanceID == request.instanceID, epoch == request.connectionEpoch,
                  !stopped, state == .syncing else { return }
            snapshot = candidate
            validator = nil
            prepareClose(.activate)
            onSnapshotChange?(candidate)
            // An observer may synchronously Disconnect/stop while publishing.
            guard owned?.instanceID == request.instanceID, epoch == request.connectionEpoch,
                  !stopped, closePlan == .activate else { return }
            requestClose()
        } catch {
            if owned?.instanceID == request.instanceID, epoch == request.connectionEpoch,
               !stopped, state == .syncing { pause(.storageFailed) }
        }
    }

    private func pause(_ reason: Code0SessionFailure) {
        failure = reason
        validator = nil
        isAutomaticallySyncing = false
        beginClose(.pause)
    }

    private func prepareClose(_ plan: ClosePlan) {
        validator = nil
        isAutomaticallySyncing = false
        attemptDeadline = nil
        closePlan = plan
        state = .closing
        if owned != nil, closeDeadline == nil { closeDeadline = clock.now.advanced(by: .seconds(10)) }
    }

    private func beginClose(_ plan: ClosePlan) {
        prepareClose(plan)
        if owned != nil { requestClose() }
        else { finishClose(plan) }
    }

    private func requestClose() {
        guard let request = owned, !closeRequested else { return }
        closeRequested = true
        driver.close(request.instanceID)
    }

    private func finishRelease() {
        var plan = closePlan ?? .pause
        if stopped, plan != .disconnect, plan != .cleanupFailure { plan = .stop }
        if closePlan == nil { failure = .navigationFailed }
        owned = nil
        validator = nil
        policiesInstalled = false
        closeRequested = false
        attemptDeadline = nil
        closeDeadline = nil
        closePlan = nil
        finishClose(plan)
    }

    private func finishClose(_ plan: ClosePlan) {
        switch plan {
        case .beginSync:
            if !stopped { beginWindow(.sync) }
            else { finishClose(.stop) }
        case .activate:
            guard !stopped else { finishClose(.stop); return }
            state = .active
            failure = nil
            isAutomaticallySyncing = true
        case .pause:
            state = .paused
        case .cleanupFailure:
            failCleanup(.storageFailed)
        case .disconnect:
            beginDataCleanup()
        case .stop:
            state = snapshot == nil ? .disconnected : .paused
            releaseLease()
        }
    }

    private func beginDataCleanup() {
        guard owned == nil, cleanupID == nil, leaseHeld else { failCleanup(.cleanupFailed); return }
        let id = UUID()
        cleanupID = id
        closePlan = .disconnect
        state = .closing
        do { try invalidateCache() }
        catch { cleanupID = nil; failCleanup(.storageFailed); return }
        driver.clearWebsiteData { [weak self] result in self?.enqueue(.cleanup(result, id)) }
    }

    private func finishDataCleanup(_ result: Result<Void, Code0SessionFailure>, id: UUID) {
        guard cleanupID == id else { return }
        cleanupID = nil
        switch result {
        case .failure: failCleanup(.cleanupFailed)
        case .success:
            closePlan = nil
            hasConnection = false
            lastSyncStart = nil
            snapshot = nil
            failure = nil
            state = .disconnected
            releaseLease()
        }
    }

    private func failCleanup(_ reason: Code0SessionFailure) {
        failure = reason
        validator = nil
        isAutomaticallySyncing = false
        state = .cleanupPending
    }

    private func invalidateCache() throws {
        try store.remove()
        cacheMayExist = false
    }

    private func releaseLease() {
        guard leaseHeld else { return }
        leaseHeld = false
        driver.releaseProfileLease()
    }

    /// Driver callbacks may fire synchronously inside create/navigate/close.
    /// Queue them until the current state transition has installed its identity.
    private func mutate<T>(_ operation: () -> T) -> T {
        mutationDepth += 1
        defer { mutationDepth -= 1; drainEvents() }
        return operation()
    }

    private func enqueue(_ event: PendingEvent) {
        events.append(event)
        drainEvents()
    }

    private func drainEvents() {
        guard mutationDepth == 0 else { return }
        while !events.isEmpty {
            let event = events.removeFirst()
            mutationDepth += 1
            switch event {
            case .window(let event, let request): handle(event, request: request)
            case .cleanup(let result, let id): finishDataCleanup(result, id: id)
            }
            mutationDepth -= 1
        }
    }

    private static func validCache(_ snapshot: UsageSnapshot) -> Bool {
        func metric(_ metric: UsageMetric?, label: String, kind: UsageMetricKind) -> Bool {
            guard let metric else { return false }
            return metric.label == label && metric.unit == .siteCredits && metric.kind == kind &&
                metric.current.isFinite && metric.current >= 0 && metric.limit == nil &&
                metric.resetAt == nil && metric.resetDescription == nil
        }
        return snapshot.provider == .code0 && snapshot.fetchedAt.timeIntervalSince1970.isFinite &&
            metric(snapshot.primaryMetric, label: "Balance", kind: .balance) &&
            metric(snapshot.secondaryMetric, label: "Cumulative usage", kind: .cumulativeUsage)
    }
}
