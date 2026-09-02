import Foundation

enum SoftwareUpdateFailure: Equatable, Sendable {
    case offline
    case timedOut
    case invalidFeed
    case invalidSignature
    case permissionDenied
    case configuration
    case other

    var userMessage: String {
        switch self {
        case .offline:
            "You appear to be offline."
        case .timedOut:
            "The update check timed out."
        case .invalidFeed:
            "The update information is unavailable."
        case .invalidSignature:
            "The update could not be verified."
        case .permissionDenied:
            "The update could not be installed in Applications."
        case .configuration:
            "Software updates are not configured correctly."
        case .other:
            "The update check failed. Try again later."
        }
    }
}

enum SoftwareUpdateEvent: Equatable, Sendable {
    case found(SoftwareUpdateRelease)
    case noUpdate
    case downloadStarted(SoftwareUpdateRelease)
    case downloaded(SoftwareUpdateRelease)
    case extracting(SoftwareUpdateRelease)
    case installing(SoftwareUpdateRelease)
    case cancelled(SoftwareUpdateRelease?)
    case failed(SoftwareUpdateFailure)
}

@MainActor
protocol SoftwareUpdateEngine: AnyObject {
    var eventHandler: ((SoftwareUpdateEvent) -> Void)? { get set }
    var canCheckForUpdates: Bool { get }

    func start() throws
    func checkForUpdateInformation()
    func presentAvailableUpdate()
    func stop()
}
