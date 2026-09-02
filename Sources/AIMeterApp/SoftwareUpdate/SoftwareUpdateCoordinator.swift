import Foundation
import Observation

@MainActor
@Observable
final class SoftwareUpdateCoordinator {
    let currentVersion: String
    let currentBuild: String

    private(set) var state: SoftwareUpdateState = .idle
    private(set) var lastCheckedAt: Date?

    private let engine: any SoftwareUpdateEngine
    private let now: @MainActor () -> Date
    private var isStopped = false

    init(
        engine: any SoftwareUpdateEngine,
        currentVersion: String,
        currentBuild: String,
        now: @escaping @MainActor () -> Date = Date.init
    ) {
        self.engine = engine
        self.currentVersion = currentVersion
        self.currentBuild = currentBuild
        self.now = now

        engine.eventHandler = { [weak self] event in
            self?.receive(event)
        }

        do {
            try engine.start()
        } catch {
            state = .failed(SoftwareUpdateFailure.configuration.userMessage)
        }
    }

    var canCheck: Bool {
        !isStopped && state.canCheck && engine.canCheckForUpdates
    }

    var canInstall: Bool {
        !isStopped && state.canInstall
    }

    func checkForUpdates() {
        guard canCheck else { return }
        state = .checking
        engine.checkForUpdateInformation()
    }

    func installAvailableUpdate() {
        guard canInstall, case let .available(release) = state else { return }
        state = .installing(release)
        engine.presentAvailableUpdate()
    }

    func stop() {
        guard !isStopped else { return }
        isStopped = true
        engine.eventHandler = nil
        engine.stop()
    }

    private func receive(_ event: SoftwareUpdateEvent) {
        guard !isStopped else { return }

        switch event {
        case let .found(release):
            lastCheckedAt = now()
            if case let .installing(activeRelease) = state, activeRelease == release {
                return
            }
            state = .available(release)
        case .noUpdate:
            lastCheckedAt = now()
            state = .upToDate
        case let .downloadStarted(release),
             let .downloaded(release),
             let .extracting(release),
             let .installing(release):
            state = .installing(release)
        case let .cancelled(release):
            state = release.map(SoftwareUpdateState.available) ?? .idle
        case let .failed(failure):
            lastCheckedAt = now()
            state = .failed(failure.userMessage)
        }
    }
}
