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

/// RED checkpoint: all actions deny; no browser, lease, write, timer or automatic
/// authorization exists until isolated CI demonstrates the missing behavior.
@MainActor
@Observable
final class Code0WebSession: Code0WebSessionProviding {
    private(set) var state: Code0SessionState = .disconnected
    private(set) var snapshot: UsageSnapshot?
    private(set) var failure: Code0SessionFailure?
    var isAutomaticallySyncing: Bool { false }
    @ObservationIgnored var onSnapshotChange: (@MainActor (UsageSnapshot?) -> Void)?

    private let clock: any Code0SessionClock
    private let driver: any Code0WindowDriving
    private let store: any Code0SnapshotStoring

    init(clock: any Code0SessionClock, driver: any Code0WindowDriving, store: any Code0SnapshotStoring) {
        self.clock = clock
        self.driver = driver
        self.store = store
    }

    @discardableResult func connect() -> Code0SessionActionResult { .denied }
    @discardableResult func loginCompleted() -> Code0SessionActionResult { .denied }
    @discardableResult func syncNow() -> Code0SessionActionResult { .denied }
    @discardableResult func disconnect() -> Code0SessionActionResult { .denied }
    func processScheduledWork() {}
    func stop() {}
}
