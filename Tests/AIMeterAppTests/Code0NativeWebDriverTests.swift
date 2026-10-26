#if AI_METER_NATIVE_FIXTURES
import AIMeterCore
import AppKit
import Foundation
import Testing
@preconcurrency import WebKit
@preconcurrency import Network
@testable import AIMeterApp

@Suite("Code0 actual owned native driver", .serialized,
       .enabled(if: ProcessInfo.processInfo.environment["AI_METER_SCREEN_TESTS"] == "1"))
@MainActor
struct Code0NativeDriverFixtureTests {
    @Test func coldDriverDoesNotCreateBrowserOrAcquireLease() async throws {
        let fixture = try NativeDriverFixture(page: .quota)
        #expect(fixture.profile.acquisitions == 0)
        fixture.inspect { window, view in #expect(window == nil); #expect(view == nil) }
        await fixture.finish(caseID: "cold-driver", outcome: "cold-no-browser")
    }

    @Test func unloadedOwnedWindowInstallsPoliciesBeforeNavigation() async throws {
        await run(try NativeDriverFixture(page: .quota), caseID: "unloaded-policies") { fixture in
            #expect(fixture.record.messages == 0)
            fixture.inspect { window, view in
                #expect(window?.isVisible == false)
                #expect(window?.isKeyWindow == false)
                #expect(view?.url == nil || view?.url?.absoluteString == "about:blank")
                #expect(view?.configuration.websiteDataStore.isPersistent == false)
                let selectors = [
                    "webView:runJavaScriptAlertPanelWithMessage:initiatedByFrame:completionHandler:",
                    "webView:runJavaScriptConfirmPanelWithMessage:initiatedByFrame:completionHandler:",
                    "webView:runJavaScriptTextInputPanelWithPrompt:defaultText:initiatedByFrame:completionHandler:",
                    "webView:runOpenPanelWithParameters:initiatedByFrame:completionHandler:",
                    "webView:requestMediaCapturePermissionForOrigin:initiatedByFrame:type:decisionHandler:",
                ]
                for selector in selectors { #expect(view?.uiDelegate?.responds(to: NSSelectorFromString(selector)) == true) }
                #expect(view?.navigationDelegate?.responds(to: NSSelectorFromString("webView:authenticationChallenge:shouldAllowDeprecatedTLS:")) == true)
            }
        }
    }

    @Test func canonicalBridgeUsesRealOwnedSourceAndDedicatedStore() async throws {
        await run(try NativeDriverFixture(page: .quota), caseID: "capture") { fixture in
            fixture.navigate()
            let captured = await nativeWait(seconds: 4) { fixture.record.capture != nil }
            #expect(captured, "A valid synthetic page must reach the real native validator")
            #expect(fixture.record.captureCount == 1)
            #expect(fixture.record.onlyOfficialMainFrame)
            if let capture = fixture.record.capture {
                do {
                    try fixture.store.save(capture.makeSnapshot(fetchedAt: Date(timeIntervalSince1970: 1_793_016_000)))
                    let stored = try fixture.store.load()
                    #expect(stored?.primaryMetric?.current == 2.5)
                    #expect(stored?.secondaryMetric?.current == 1.5)
                } catch { Issue.record("The held-lease dedicated store must commit the real native capture") }
            }
        }
    }

    @Test(arguments: ["window", "view"])
    func retainedNativeObjectPreventsReleaseAndNextCreation(retained: String) async throws {
        await run(try NativeDriverFixture(page: .quota), caseID: "retain-\(retained)") { fixture in
            var retainedWindow = retained == "window" ? fixture.driver.fixtureObjects(instanceID: fixture.request.instanceID).window : nil
            var retainedView = retained == "view" ? fixture.driver.fixtureObjects(instanceID: fixture.request.instanceID).webView : nil
            fixture.driver.close(fixture.request.instanceID)
            _ = await nativeWait(seconds: 1) { false }
            #expect(!fixture.record.released.contains(fixture.request.instanceID))
            #expect(retainedWindow != nil || retainedView != nil)
            let next = fixture.nextRequest()
            var secondCreated = false
            do { try fixture.driver.createUnloaded(next) { _ in }; secondCreated = true } catch {}
            #expect(!secondCreated, "A retained owned object must block the next instance")
            fixture.driver.releaseProfileLease()
            #expect(fixture.profile.isHeld, "Uncertain owned-object release must retain the profile lease")
            withExtendedLifetime((retainedWindow, retainedView)) {}
            retainedWindow = nil
            retainedView = nil
            let released = await nativeWait(seconds: 4) { fixture.record.released.contains(fixture.request.instanceID) }
            #expect(released)
            #expect(fixture.weakObjects.window == nil)
            #expect(fixture.weakObjects.view == nil)
        }
    }

    @Test(arguments: [Code0NativeFixturePage.alert, .confirm, .prompt])
    func actualDialogCallbacksAreInstalledAndCancel(page: Code0NativeFixturePage) async throws {
        await run(try NativeDriverFixture(page: page), caseID: page.rawValue) { fixture in
            fixture.navigate()
            let callback: Code0NativePolicyCallback = page == .alert ? .alert : (page == .confirm ? .confirm : .prompt)
            let observed = await nativeWait(seconds: 4) { fixture.hasDelegate(callback) }
            #expect(observed, "A selector or hand-called delegate is not an actual WebKit dialog event")
            #expect(fixture.record.failure == .policyRejected)
            fixture.assertHidden()
        }
    }

    @Test(arguments: [Code0NativeFixturePage.popup, .filePanel, .externalNavigation, .download])
    func actualWindowFileNavigationAndDownloadRequestsAreRefused(page: Code0NativeFixturePage) async throws {
        await run(try NativeDriverFixture(page: page), caseID: page.rawValue) { fixture in
            fixture.navigate()
            let callback: Code0NativePolicyCallback
            switch page {
            case .popup: callback = .newWindow
            case .filePanel: callback = .filePanel
            case .download: callback = .download
            default: callback = .navigation
            }
            let observed = await nativeWait(seconds: 4) { fixture.hasDelegate(callback) }
            #expect(observed, "This native branch remains UNOBSERVED unless WebKit actually calls it")
            #expect(fixture.record.failure == .policyRejected)
            fixture.assertHidden()
        }
    }

    @Test(arguments: [Code0NativeFixturePage.media, .geolocation, .notification, .hiddenDisplay])
    func permissionResultDistinguishesNativeDelegateFromEarlyRefusal(page: Code0NativeFixturePage) async throws {
        await run(try NativeDriverFixture(page: page), caseID: page.rawValue) { fixture in
            fixture.navigate()
            let terminal = await nativeWait(seconds: 4) { fixture.hasDelegate(.mediaCapture) || fixture.pageOutcome != nil }
            #expect(terminal, "A missing result is UNOBSERVED, never evidence of denial")
            #expect(fixture.pageOutcome != .resolved)
            #expect(fixture.pageOutcome != .timedOut)
            if fixture.hasDelegate(.mediaCapture) { #expect(fixture.record.failure == .policyRejected) }
            fixture.assertHidden()
            // No camera/mic callback is claimed for an earlier security/gesture
            // refusal, or for macOS's geolocation/notification default-deny path.
        }
    }

    @Test func actualControlledHTTPAuthenticationChallengeIsCancelled() async throws {
        await run(try NativeDriverFixture(page: .httpAuthentication), caseID: "http-auth") { fixture in
            fixture.navigate()
            let observed = await nativeWait(seconds: 4) { fixture.hasDelegate(.httpAuthentication) }
            #expect(observed, "Only an actual controlled HTTP 401 WebKit challenge covers this branch")
            #expect(fixture.record.failure == .policyRejected)
            fixture.assertHidden()
            // Client-certificate and TLS handshake coverage is separate and
            // remains UNOBSERVED; this test never grants a trust exception.
        }
    }

    @Test func controlledHTTPListenerChallengesOnlyItsExactGETPath() async throws {
        let server = Code0AuthenticationFixtureServer()
        defer { server.stop() }
        var ready = false
        var endpoint: URL?
        try server.start { url in endpoint = url; ready = true }
        let becameReady = await nativeWait(seconds: 2) { ready }
        try #require(becameReady, "The controlled listener must become ready within its bounded fixture lifetime")
        let url = try #require(endpoint)
        let exact = await NativeHTTPRouteExchange().exchange(url: url, requestLine: "GET \(url.path) HTTP/1.1")
        #expect(exact == .status(401), "The one native-owned GET path must exercise the real challenge server")
        for requestLine in ["GET /unrelated HTTP/1.1", "POST \(url.path) HTTP/1.1", "not-http"] {
            let status = await NativeHTTPRouteExchange().exchange(url: url, requestLine: requestLine)
            #expect(status == .closed || status == .status(404), "Unrelated paths/methods/non-HTTP input must never receive an authentication challenge")
        }
        // No CODE0_NATIVE_DRIVER marker: the original17-case evidence set stays
        // exact. This additional regression observes the same server's route.
    }

    private func run(_ fixture: NativeDriverFixture, caseID: String,
                     body: @MainActor (NativeDriverFixture) async -> Void) async {
        let created = fixture.createUnloaded()
        let ready = created ? await nativeWait(seconds: 4) { fixture.record.policiesInstalled } : false
        if created && ready {
            fixture.sampleWeakObjects()
            #expect(fixture.weakObjects.window != nil && fixture.weakObjects.view != nil)
            await body(fixture)
        }
        await fixture.finish(caseID: caseID, outcome: created && ready ? "driver-observation" : "denied-no-owned-view")
        #expect(created, "The deny-only RED driver cannot create the required real owned WebKit instance")
        #expect(ready, "Native policy installation acknowledgement must precede first navigation")
    }
}

@MainActor
private func nativeWait(seconds: Int, condition: @MainActor () -> Bool) async -> Bool {
    let deadline = ContinuousClock.now.advanced(by: .seconds(seconds))
    while !condition(), ContinuousClock.now < deadline {
        do { try await Task.sleep(for: .milliseconds(10)) } catch { return condition() }
    }
    return condition()
}

/// A bounded raw loopback client reads only the fixed HTTP status line. It
/// sends no header, credential, cookie or body and never uses a website URL.
@MainActor
private final class NativeHTTPRouteExchange {
    enum Outcome: Equatable, Sendable { case status(Int), closed, timedOut }
    private var connection: NWConnection?
    private var continuation: CheckedContinuation<Outcome, Never>?
    private var timeout: Task<Void, Never>?
    private var bytes = Data()

    func exchange(url: URL, requestLine: String) async -> Outcome {
        defer { withExtendedLifetime(self) {} }
        guard url.host == "127.0.0.1", let value = url.port, value > 0, value <= 65535,
              let port = NWEndpoint.Port(rawValue: UInt16(value)) else { return .closed }
        return await withCheckedContinuation { continuation in
            self.continuation = continuation
            let connection = NWConnection(host: "127.0.0.1", port: port, using: .tcp)
            self.connection = connection
            timeout = Task { @MainActor [weak self] in
                do { try await Task.sleep(for: .seconds(2)) } catch { return }
                self?.finish(.timedOut)
            }
            connection.stateUpdateHandler = { [weak self] state in
                Task { @MainActor in
                    guard let self, self.connection === connection else { return }
                    switch state {
                    case .ready:
                        connection.send(content: Data((requestLine + "\r\n\r\n").utf8), completion: .contentProcessed { _ in })
                        self.receive(connection)
                    case .failed, .cancelled: self.finish(.closed)
                    default: break
                    }
                }
            }
            connection.start(queue: DispatchQueue(label: "Code0.synthetic-route-client"))
        }
    }
    private func receive(_ connection: NWConnection) {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 256) { [weak self] data, _, complete, error in
            Task { @MainActor in
                guard let self, self.connection === connection else { return }
                if let data { self.bytes.append(data.prefix(max(0, 256 - self.bytes.count))) }
                if let end = self.bytes.range(of: Data([13, 10])) {
                    let line = String(decoding: self.bytes[..<end.lowerBound], as: UTF8.self)
                    let fields = line.split(separator: " ")
                    if fields.count >= 2, fields[0] == "HTTP/1.1", let status = Int(fields[1]) { self.finish(.status(status)) }
                    else { self.finish(.closed) }
                } else if complete || error != nil || self.bytes.count >= 256 { self.finish(.closed) }
                else { self.receive(connection) }
            }
        }
    }
    private func finish(_ result: Outcome) {
        guard let continuation else { return }
        self.continuation = nil
        timeout?.cancel(); timeout = nil
        connection?.stateUpdateHandler = nil
        connection?.cancel(); connection = nil
        bytes = Data()
        continuation.resume(returning: result)
    }
}

@MainActor
private final class NativeDriverProfile: Code0ProfileLeasing {
    let location: Code0StorageLocation
    var isHeld = false
    var acquisitions = 0
    init(location: Code0StorageLocation) { self.location = location }
    func acquire() throws { acquisitions += 1; isHeld = true }
    func release() { isHeld = false }
}

@MainActor
private final class NativeDriverWeakObjects {
    weak var window: NSWindow?
    weak var view: WKWebView?
}

@MainActor
private final class NativeDriverRecord {
    var policiesInstalled = false
    var capture: Code0WebsiteCapture?
    var captureCount = 0
    var messages = 0
    var onlyOfficialMainFrame = true
    var failure: Code0SessionFailure?
    var released: Set<UUID> = []
    var validator: Code0BridgeValidator
    init(request: Code0WindowRequest) {
        validator = Code0BridgeValidator(connectionEpoch: request.connectionEpoch,
            attemptID: request.instanceID, nonce: request.nonce, documentID: request.documentID)
    }
    func receive(_ event: Code0WindowEvent, request: Code0WindowRequest) {
        switch event {
        case .policiesInstalled: policiesInstalled = true
        case .released: released.insert(request.instanceID)
        case .failed(let reason): failure = reason
        case .message(let payload, let sourceURL, let isMainFrame):
            messages += 1
            onlyOfficialMainFrame = onlyOfficialMainFrame && isMainFrame && sourceURL == "https://console.code0.ai/"
            let event = validator.receive(payload, context: Code0BridgeContext(connectionEpoch: request.connectionEpoch,
                attemptID: request.instanceID, sourceURL: sourceURL, isMainFrame: isMainFrame))
            if case .capture(let capture) = event { self.capture = capture; captureCount += 1 }
        }
    }
}

@MainActor
private final class NativeDriverFixture {
    let root: URL
    let request: Code0WindowRequest
    let profile: NativeDriverProfile
    let driver: Code0NativeWebDriver
    let store: Code0SnapshotStore
    let record: NativeDriverRecord
    let weakObjects = NativeDriverWeakObjects()
    private var requests: [Code0WindowRequest]
    private var hiddenBefore = false
    private var unfocusedBefore = false

    init(page: Code0NativeFixturePage) throws {
        _ = NSApplication.shared
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("Code0Native-\(UUID())")
        let location = try Code0StorageLocation(applicationSupportRoot: root)
        try FileManager.default.createDirectory(at: location.directoryURL, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        let profile = NativeDriverProfile(location: location)
        let request = Code0WindowRequest(instanceID: UUID(), connectionEpoch: UUID(), nonce: UUID(), documentID: UUID(), mode: .sync)
        self.root = root; self.request = request; self.requests = [request]; self.profile = profile
        self.store = Code0SnapshotStore(profile: profile)
        self.driver = Code0NativeWebDriver.fixture(profile: profile, page: page)
        self.record = NativeDriverRecord(request: request)
    }

    func createUnloaded() -> Bool {
        do {
            try driver.acquireProfileLease()
            let record = self.record, request = self.request
            try driver.createUnloaded(request) { event in record.receive(event, request: request) }
            return true
        } catch { return false }
    }
    func navigate() {
        do { try driver.navigate(request.instanceID) }
        catch { Issue.record("An installed fixture must accept only its predefined local document") }
    }
    func inspect(_ body: (NSWindow?, WKWebView?) -> Void) {
        let objects = driver.fixtureObjects(instanceID: request.instanceID)
        body(objects.window, objects.webView)
    }
    func sampleWeakObjects() {
        inspect { window, view in
            weakObjects.window = window; weakObjects.view = view
            hiddenBefore = window.map { !$0.isVisible } ?? false
            unfocusedBefore = window.map { !$0.isKeyWindow && !$0.isMainWindow } ?? false
        }
    }
    func nextRequest() -> Code0WindowRequest {
        let next = Code0WindowRequest(instanceID: UUID(), connectionEpoch: request.connectionEpoch,
            nonce: UUID(), documentID: UUID(), mode: .sync)
        requests.append(next)
        return next
    }
    var pageOutcome: Code0NativePageOutcome? {
        driver.fixtureObservations.compactMap { if case .page(let result) = $0 { return result }; return nil }.last
    }
    func hasDelegate(_ kind: Code0NativePolicyCallback) -> Bool {
        driver.fixtureObservations.contains { if case .delegate(let value) = $0 { return value == kind }; return false }
    }
    func assertHidden() {
        inspect { window, _ in #expect(window?.isVisible == false); #expect(window?.isKeyWindow == false) }
    }
    func finish(caseID: String, outcome: String) async {
        var hiddenAfter = true, unfocusedAfter = true
        inspect { window, _ in
            hiddenAfter = window.map { !$0.isVisible } ?? true
            unfocusedAfter = window.map { !$0.isKeyWindow && !$0.isMainWindow } ?? true
        }
        let cleanupObjects = requests.map { request in
            let weak = NativeDriverWeakObjects()
            let objects = driver.fixtureObjects(instanceID: request.instanceID)
            weak.window = objects.window; weak.view = objects.webView
            return weak
        }
        let hadOwnedObjects = cleanupObjects.contains { $0.window != nil || $0.view != nil }
        for request in requests { driver.close(request.instanceID) }
        let released = await nativeWait(seconds: 11) {
            cleanupObjects.allSatisfy { $0.window == nil && $0.view == nil } &&
                weakObjects.window == nil && weakObjects.view == nil &&
                (!hadOwnedObjects || record.released.contains(request.instanceID))
        }
        #expect(released, "Actual owned objects and the release acknowledgement are required during cleanup")
        driver.releaseProfileLease()
        let delegates = driver.fixtureObservations.compactMap { observation -> String? in
            if case .delegate(let value) = observation { return value.rawValue }; return nil
        }
        let fields: [String: Any] = [
            "schema": 1, "case": caseID, "outcome": outcome,
            "delegates": Array(Set(delegates)).sorted(), "page": pageOutcome?.rawValue ?? "unobserved",
            "stages": Array(Set(driver.fixtureObservations.compactMap { observation -> String? in
                if case .stage(let value) = observation { return value.rawValue }; return nil
            })).sorted(),
            "policiesInstalled": record.policiesInstalled, "messages": min(record.messages, 16),
            "captureCount": min(record.captureCount, 2),
            "officialMainFrame": record.messages > 0 ? record.onlyOfficialMainFrame as Any : NSNull(),
            "coverage": !delegates.isEmpty ? "native-delegate-observed" : (pageOutcome == nil ? "unobserved" : "page-only-result"),
            "hiddenBefore": hiddenBefore, "unfocusedBefore": unfocusedBefore,
            "hiddenAfter": hiddenAfter, "unfocusedAfter": unfocusedAfter,
            "releasedAck": record.released.contains(request.instanceID),
            "windowReleased": weakObjects.window == nil, "viewReleased": weakObjects.view == nil,
            "unobservedHandshake": ["client-certificate", "tls-server-trust"],
        ]
        if let data = try? JSONSerialization.data(withJSONObject: fields, options: [.sortedKeys]),
           data.count <= 2048, let text = String(data: data, encoding: .utf8) {
            print("CODE0_NATIVE_DRIVER \(text)")
        } else { Issue.record("Driver evidence must remain finite and contain no raw website data") }
        try? FileManager.default.removeItem(at: root)
    }
}
#endif
