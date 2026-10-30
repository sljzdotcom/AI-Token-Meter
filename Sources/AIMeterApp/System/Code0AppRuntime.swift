import AIMeterCore
import Foundation

@MainActor
struct Code0SystemClock: Code0SessionClock {
    var now: ContinuousClock.Instant { ContinuousClock.now }
    var wallNow: Date { Date() }
}

/// Cold composition only: load the dedicated numeric cache without taking the
/// lease, creating a website store/window, or authorizing this run.
@MainActor
enum Code0AppRuntime {
    static func makeSession(applicationSupportAnchor: URL?) throws -> any Code0WebSessionProviding {
        let location: Code0StorageLocation
        if let applicationSupportAnchor { location = try Code0StorageLocation(applicationSupportRoot: applicationSupportAnchor) }
        else { location = try Code0StorageLocation.applicationSupport() }
        let profile = Code0ProfileLease(location: location)
        let store = Code0SnapshotStore(profile: profile)
        let driver = Code0NativeWebDriver(profile: profile)
        return Code0WebSession(clock: Code0SystemClock(), driver: driver, store: store)
    }
}
