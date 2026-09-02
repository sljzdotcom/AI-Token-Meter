import Foundation
import Testing
@testable import AIMeterApp

@MainActor
@Suite("Software update coordinator")
struct SoftwareUpdateCoordinatorTests {
    private let release = SoftwareUpdateRelease(
        version: "0.2.0",
        build: "4",
        publishedAt: Date(timeIntervalSince1970: 1_788_192_000),
        summary: "A secure update."
    )

    @Test("A manual check enters the checking state and calls the informational API")
    func manualCheck() {
        let engine = FakeSoftwareUpdateEngine()
        let coordinator = SoftwareUpdateCoordinator(
            engine: engine,
            currentVersion: "0.1.2",
            currentBuild: "3"
        )

        coordinator.checkForUpdates()

        #expect(coordinator.state == .checking)
        #expect(engine.informationCheckCount == 1)
        #expect(engine.presentUpdateCount == 0)
    }

    @Test("A busy coordinator ignores duplicate checks")
    func duplicateCheck() {
        let engine = FakeSoftwareUpdateEngine()
        let coordinator = makeCoordinator(engine: engine)

        coordinator.checkForUpdates()
        coordinator.checkForUpdates()

        #expect(engine.informationCheckCount == 1)
    }

    @Test("A discovered release enables installation and records the check time")
    func foundUpdate() {
        let engine = FakeSoftwareUpdateEngine()
        let now = Date(timeIntervalSince1970: 1_788_278_400)
        let coordinator = makeCoordinator(engine: engine, now: { now })

        coordinator.checkForUpdates()
        engine.send(.found(release))

        #expect(coordinator.state == .available(release))
        #expect(coordinator.lastCheckedAt == now)
        #expect(coordinator.canInstall)
    }

    @Test("No update reports an up-to-date state")
    func noUpdate() {
        let engine = FakeSoftwareUpdateEngine()
        let coordinator = makeCoordinator(engine: engine)

        coordinator.checkForUpdates()
        engine.send(.noUpdate)

        #expect(coordinator.state == .upToDate)
        #expect(coordinator.lastCheckedAt != nil)
    }

    @Test("Failures expose only fixed safe copy")
    func safeFailureCopy() {
        let engine = FakeSoftwareUpdateEngine()
        let coordinator = makeCoordinator(engine: engine)

        engine.send(.failed(.invalidSignature))

        #expect(coordinator.state == .failed("The update could not be verified."))
        #expect(coordinator.lastCheckedAt != nil)
    }

    @Test("Update Now is ignored until a release is available")
    func installWithoutRelease() {
        let engine = FakeSoftwareUpdateEngine()
        let coordinator = makeCoordinator(engine: engine)

        coordinator.installAvailableUpdate()

        #expect(coordinator.state == .idle)
        #expect(engine.presentUpdateCount == 0)
    }

    @Test("Update Now enters installation and presents Sparkle once")
    func installAvailableUpdate() {
        let engine = FakeSoftwareUpdateEngine()
        let coordinator = makeCoordinator(engine: engine)
        engine.send(.found(release))

        coordinator.installAvailableUpdate()
        coordinator.installAvailableUpdate()

        #expect(coordinator.state == .installing(release))
        #expect(engine.presentUpdateCount == 1)
    }

    @Test("A repeated discovery during installation does not re-enable the button")
    func foundDuringInstallation() {
        let engine = FakeSoftwareUpdateEngine()
        let coordinator = makeCoordinator(engine: engine)
        engine.send(.found(release))
        coordinator.installAvailableUpdate()

        engine.send(.found(release))

        #expect(coordinator.state == .installing(release))
        #expect(!coordinator.canInstall)
    }

    @Test("Stopping detaches the engine and ignores stale events")
    func stop() {
        let engine = FakeSoftwareUpdateEngine()
        let coordinator = makeCoordinator(engine: engine)
        let staleHandler = engine.eventHandler

        coordinator.stop()
        staleHandler?(.found(release))

        #expect(engine.stopCount == 1)
        #expect(engine.eventHandler == nil)
        #expect(coordinator.state == .idle)
    }

    @Test("An unavailable engine disables checks")
    func unavailableEngine() {
        let engine = FakeSoftwareUpdateEngine()
        engine.canCheckForUpdates = false
        let coordinator = makeCoordinator(engine: engine)

        coordinator.checkForUpdates()

        #expect(!coordinator.canCheck)
        #expect(engine.informationCheckCount == 0)
    }

    @Test("Configuration failures do not crash app startup")
    func startupFailure() {
        let engine = FakeSoftwareUpdateEngine()
        engine.startError = StubError.startFailed

        let coordinator = makeCoordinator(engine: engine)

        #expect(coordinator.state == .failed("Software updates are not configured correctly."))
    }

    private func makeCoordinator(
        engine: FakeSoftwareUpdateEngine,
        now: @escaping @MainActor () -> Date = Date.init
    ) -> SoftwareUpdateCoordinator {
        SoftwareUpdateCoordinator(
            engine: engine,
            currentVersion: "0.1.2",
            currentBuild: "3",
            now: now
        )
    }
}

@MainActor
private final class FakeSoftwareUpdateEngine: SoftwareUpdateEngine {
    var eventHandler: ((SoftwareUpdateEvent) -> Void)?
    var canCheckForUpdates = true
    var startError: Error?
    private(set) var informationCheckCount = 0
    private(set) var presentUpdateCount = 0
    private(set) var stopCount = 0

    func start() throws {
        if let startError {
            throw startError
        }
    }

    func checkForUpdateInformation() {
        informationCheckCount += 1
    }

    func presentAvailableUpdate() {
        presentUpdateCount += 1
    }

    func stop() {
        stopCount += 1
    }

    func send(_ event: SoftwareUpdateEvent) {
        eventHandler?(event)
    }
}

private enum StubError: Error {
    case startFailed
}
