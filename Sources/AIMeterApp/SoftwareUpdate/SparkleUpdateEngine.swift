import Foundation
import Sparkle

@MainActor
final class SparkleUpdateEngine: NSObject, SoftwareUpdateEngine, SPUUpdaterDelegate {
    var eventHandler: ((SoftwareUpdateEvent) -> Void)?

    private var controller: SPUStandardUpdaterController?
    private var currentRelease: SoftwareUpdateRelease?

    var canCheckForUpdates: Bool {
        controller?.updater.canCheckForUpdates ?? false
    }

    func start() throws {
        guard controller == nil else { return }
        controller = SPUStandardUpdaterController(
            startingUpdater: true,
            updaterDelegate: self,
            userDriverDelegate: nil
        )
    }

    func checkForUpdateInformation() {
        controller?.updater.checkForUpdateInformation()
    }

    func presentAvailableUpdate() {
        controller?.checkForUpdates(nil)
    }

    func stop() {
        eventHandler = nil
        currentRelease = nil
        controller = nil
    }

    func updater(_ updater: SPUUpdater, didFindValidUpdate item: SUAppcastItem) {
        let release = release(from: item)
        currentRelease = release
        eventHandler?(.found(release))
    }

    func updaterDidNotFindUpdate(_ updater: SPUUpdater, error: any Error) {
        eventHandler?(.noUpdate)
    }

    func updater(
        _ updater: SPUUpdater,
        willDownloadUpdate item: SUAppcastItem,
        with request: NSMutableURLRequest
    ) {
        let release = release(from: item)
        currentRelease = release
        eventHandler?(.downloadStarted(release))
    }

    func updater(_ updater: SPUUpdater, didDownloadUpdate item: SUAppcastItem) {
        let release = release(from: item)
        currentRelease = release
        eventHandler?(.downloaded(release))
    }

    func updater(_ updater: SPUUpdater, failedToDownloadUpdate item: SUAppcastItem, error: any Error) {
        eventHandler?(.failed(Self.failure(for: error)))
    }

    func userDidCancelDownload(_ updater: SPUUpdater) {
        eventHandler?(.cancelled(currentRelease))
    }

    func updater(_ updater: SPUUpdater, willExtractUpdate item: SUAppcastItem) {
        let release = release(from: item)
        currentRelease = release
        eventHandler?(.extracting(release))
    }

    func updater(_ updater: SPUUpdater, didExtractUpdate item: SUAppcastItem) {
        let release = release(from: item)
        currentRelease = release
        eventHandler?(.installing(release))
    }

    func updater(_ updater: SPUUpdater, willInstallUpdate item: SUAppcastItem) {
        let release = release(from: item)
        currentRelease = release
        eventHandler?(.installing(release))
    }

    func updater(_ updater: SPUUpdater, didAbortWithError error: any Error) {
        let nsError = error as NSError
        if nsError.domain == SUSparkleErrorDomain,
           nsError.code == SparkleErrorCode.installationCancelled {
            eventHandler?(.cancelled(currentRelease))
        } else if nsError.domain == SUSparkleErrorDomain,
                  nsError.code == SparkleErrorCode.noUpdate {
            eventHandler?(.noUpdate)
        } else {
            eventHandler?(.failed(Self.failure(for: error)))
        }
    }

    private func release(from item: SUAppcastItem) -> SoftwareUpdateRelease {
        let plainSummary: String?
        if item.itemDescriptionFormat == "plain-text" {
            plainSummary = item.itemDescription
        } else {
            plainSummary = nil
        }

        return SoftwareUpdateRelease(
            version: item.displayVersionString,
            build: item.versionString,
            publishedAt: item.date,
            summary: plainSummary
        )
    }

    private static func failure(for error: any Error) -> SoftwareUpdateFailure {
        let nsError = error as NSError

        if nsError.domain == NSURLErrorDomain {
            switch URLError.Code(rawValue: nsError.code) {
            case .notConnectedToInternet, .networkConnectionLost, .cannotConnectToHost,
                 .cannotFindHost, .dnsLookupFailed:
                return .offline
            case .timedOut:
                return .timedOut
            default:
                break
            }
        }

        if nsError.domain == SUSparkleErrorDomain {
            switch nsError.code {
            case SparkleErrorCode.invalidFeedURL, SparkleErrorCode.insecureFeedURL,
                 SparkleErrorCode.appcastParse, SparkleErrorCode.appcast:
                return .invalidFeed
            case SparkleErrorCode.noPublicKey, SparkleErrorCode.insufficientSigning,
                 SparkleErrorCode.signature, SparkleErrorCode.validation,
                 SparkleErrorCode.notValidUpdate:
                return .invalidSignature
            case SparkleErrorCode.authenticationFailure, SparkleErrorCode.installationWriteDenied:
                return .permissionDenied
            case SparkleErrorCode.invalidUpdater, SparkleErrorCode.invalidBundleIdentifier,
                 SparkleErrorCode.invalidHostVersion, SparkleErrorCode.incorrectAPIUsage:
                return .configuration
            default:
                break
            }
        }

        if let underlying = nsError.userInfo[NSUnderlyingErrorKey] as? Error {
            return failure(for: underlying)
        }
        return .other
    }

    /// Sparkle's `SUError` is declared as an Objective-C `NS_ENUM(OSStatus, ...)`,
    /// but the binary framework does not expose those cases as Swift symbols.
    /// These values mirror Sparkle 2's stable public `SUErrors.h` contract.
    private enum SparkleErrorCode {
        static let noPublicKey = 1
        static let insufficientSigning = 2
        static let insecureFeedURL = 3
        static let invalidFeedURL = 4
        static let invalidUpdater = 5
        static let invalidBundleIdentifier = 6
        static let invalidHostVersion = 7
        static let appcastParse = 1000
        static let noUpdate = 1001
        static let appcast = 1002
        static let signature = 3001
        static let validation = 3002
        static let authenticationFailure = 4001
        static let installationCancelled = 4007
        static let notValidUpdate = 4009
        static let installationWriteDenied = 4012
        static let incorrectAPIUsage = 5000
    }
}
