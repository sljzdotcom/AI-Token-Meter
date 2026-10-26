import AIMeterCore
import AppKit
import Foundation
@preconcurrency import WebKit

/// Native-owned Code0 driver contract. This RED scaffold never constructs a
/// browser, navigates, clears a website profile, or claims release of an object.
@MainActor
final class Code0NativeWebDriver: NSObject, Code0WindowDriving {
    static let officialURL = URL(string: "https://console.code0.ai/")!
    static let websiteDataStoreIdentifier = UUID(uuidString: "3292F126-DB63-49B5-9315-42C06E6CA289")!
    private let profile: any Code0ProfileLeasing

    init(profile: any Code0ProfileLeasing) {
        self.profile = profile
        super.init()
    }

    func acquireProfileLease() throws { try profile.acquire() }
    func releaseProfileLease() { profile.release() }
    func createUnloaded(_ request: Code0WindowRequest, events: @escaping @MainActor (Code0WindowEvent) -> Void) throws {
        throw Code0SessionFailure.validationRequired
    }
    func navigate(_ instanceID: UUID) throws { throw Code0SessionFailure.validationRequired }
    func close(_ instanceID: UUID) {}
    func clearWebsiteData(completion: @escaping @MainActor (Result<Void, Code0SessionFailure>) -> Void) {
        completion(.failure(.cleanupFailed))
    }

    #if AI_METER_NATIVE_FIXTURES
    private var fixturePage: Code0NativeFixturePage?
    static func fixture(profile: any Code0ProfileLeasing, page: Code0NativeFixturePage) -> Code0NativeWebDriver {
        let driver = Code0NativeWebDriver(profile: profile)
        driver.fixturePage = page
        return driver
    }
    func fixtureObjects(instanceID: UUID) -> (window: NSWindow?, webView: WKWebView?) { (nil, nil) }
    var fixtureObservations: [Code0NativeFixtureObservation] { [] }
    #endif
}

#if AI_METER_NATIVE_FIXTURES
/// Closed test pages, not a production origin/HTML/credential configuration API.
enum Code0NativeFixturePage: String, Equatable, Sendable {
    case quota, alert, confirm, prompt, popup
    case filePanel = "file-panel"
    case media, geolocation, notification
    case hiddenDisplay = "hidden-display"
    case externalNavigation = "external-navigation"
    case download
    case httpAuthentication = "http-auth"
}

enum Code0NativePolicyCallback: String, Equatable, Sendable {
    case alert, confirm, prompt, newWindow, filePanel, mediaCapture
    case navigation, download, httpAuthentication, clientCertificate, serverTrust
}

enum Code0NativePageOutcome: String, Equatable, Sendable {
    case rejected, unsupported, resolved, timedOut, defaultResult
}

enum Code0NativeFixtureObservation: Sendable {
    case delegate(Code0NativePolicyCallback)
    case page(Code0NativePageOutcome)
}
#endif
