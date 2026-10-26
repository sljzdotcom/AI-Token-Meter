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

    @Test(arguments: [Code0NativeFixturePage.notification, .hiddenDisplay])
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

    @Test(arguments: [Code0NativeFixturePage.media, .geolocation])
    func hiddenPermissionWaitIsCancelledByRealSessionDeadline(page: Code0NativeFixturePage) async throws {
        let fixture = try NativeDriverFixture(page: page)
        let clock = NativePermissionClock()
        defer { fixture.session?.stop() }
        var started = false
        do { try fixture.prepareCachedSession(clock: clock); started = fixture.session?.syncNow() == .started }
        catch { Issue.record("A valid synthetic cached session must start an explicit fresh hidden attempt") }
        let ready = started ? await nativeWait(seconds: 4) { fixture.record.policiesInstalled } : false
        if ready {
            fixture.sampleWeakObjects()
            let old = fixture.session?.snapshot
            #expect(old?.primaryMetric?.current == 2)
            #expect(old?.fetchedAt == clock.wallNow)
            #expect(old?.collectionStatus == .cached)
            let requested = await nativeWait(seconds: 4) {
                fixture.sampleCaptureState()
                return fixture.hasStage(.requestStarted) && fixture.hasStage(.visibilityHidden) && fixture.hasStage(.pageTimerFired)
            }
            #expect(requested, "The real API must be called in an actually hidden, running document")
            #expect(fixture.pageOutcome == nil)
            #expect(!fixture.hasDelegate(.mediaCapture), "Pending before permission must not be reported as native denial")
            fixture.permissionEvidence = requested && fixture.pageOutcome == nil && !fixture.hasDelegate(.mediaCapture)
                ? "pending-before-permission" : "unobserved"
            fixture.assertHidden()
            #expect(fixture.captureInactive == true)
            // This is the injected monotonic 45s session deadline, not a claim
            // that the native fixture waited 45 wall-clock seconds.
            clock.advance(45)
            fixture.session?.processScheduledWork()
            let released = await nativeWait(seconds: 4) { fixture.record.released.contains(fixture.request.instanceID) }
            #expect(released)
            #expect(fixture.session?.failure == .syncTimedOut)
            #expect(fixture.session?.state == .paused)
            #expect(fixture.session?.snapshot == old)
            #expect(fixture.publications == 0)
            #expect(fixture.session?.isAutomaticallySyncing == false)
            #expect(fixture.record.captureCount == 0)
            #expect(fixture.pageOutcome != .resolved)
            fixture.sessionTimeout = fixture.session?.failure == .syncTimedOut
            fixture.sessionTermination = released && fixture.session?.failure == .syncTimedOut && fixture.session?.state == .paused
                ? "cancelled-by-session" : "unobserved"
            fixture.deadlineEvidence = "injected-monotonic-45s"
        }
        await fixture.finish(caseID: page.rawValue, outcome: started && ready ? "driver-observation" : "denied-no-owned-view")
        #expect(started && ready)
    }

    @Test(arguments: [Code0NativeFixturePage.media, .geolocation])
    func visibleExplicitLoginObservesActualPermissionDenial(page: Code0NativeFixturePage) async throws {
        let caseID = page == .media ? "visible-media-denial" : "visible-geolocation-denial"
        await run(try NativeDriverFixture(page: page, mode: .login), caseID: caseID) { fixture in
            fixture.navigate()
            fixture.sampleWeakObjects()
            let terminal = await nativeWait(seconds: 4) {
                fixture.sampleCaptureState()
                fixture.sampleVisibility()
                return fixture.visibleObserved && (fixture.hasDelegate(.mediaCapture) || fixture.pageOutcome != nil)
            }
            fixture.sampleCaptureState(); fixture.sampleVisibility()
            #expect(fixture.visibleObserved, "A login label alone does not prove a visible native document")
            #expect(terminal)
            #expect(fixture.captureInactive == true)
            #expect(fixture.pageOutcome != .resolved)
            #expect(fixture.pageOutcome != .timedOut)
            if page == .geolocation {
                #expect(fixture.pageOutcome == .permissionDenied, "Only actual Geolocation error.code1 proves permission denial")
                fixture.denialEvidence = fixture.pageOutcome == .permissionDenied ? "page-permission-denied" : "unobserved"
            } else if fixture.hasDelegate(.mediaCapture) {
                #expect(fixture.record.failure == .policyRejected)
                fixture.denialEvidence = "native-delegate-deny"
            } else {
                let denied = fixture.pageOutcome == .notAllowed || fixture.pageOutcome == .securityError
                #expect(denied, "No-device/other error or pending is page-only UNOBSERVED, not permission denial")
                fixture.denialEvidence = denied ? "page-permission-denied" : "unobserved"
            }
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
        let previousClientsClosed = await nativeWait(seconds: 1) { server.acceptedConnectionCount == 0 }
        #expect(previousClientsClosed)
        let idle = NativeHTTPRouteExchange()
        let idleTask = Task { @MainActor in await idle.exchange(url: url, requestLine: nil) }
        let accepted = await nativeWait(seconds: 1) { server.acceptedConnectionCount == 1 }
        #expect(accepted)
        server.stop()
        let idleResult = await idleTask.value
        #expect(idleResult == .closed, "stop must cancel an accepted client that sent no request line")
        // No owned-window marker: this regression observes the same server's
        // route and lifetime separately from the19 required driver records.
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

    func exchange(url: URL, requestLine: String?) async -> Outcome {
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
                        if let requestLine {
                            connection.send(content: Data((requestLine + "\r\n\r\n").utf8), completion: .contentProcessed { _ in })
                        }
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
private final class NativePermissionClock: Code0SessionClock {
    var now = ContinuousClock.now
    var wallNow = Date(timeIntervalSince1970: 1_793_016_000)
    func advance(_ seconds: Int) {
        now = now.advanced(by: .seconds(seconds)); wallNow = wallNow.addingTimeInterval(Double(seconds))
    }
}

@MainActor
private final class NativeSessionDriverObserver: Code0WindowDriving {
    let driver: Code0NativeWebDriver
    weak var fixture: NativeDriverFixture?
    init(driver: Code0NativeWebDriver, fixture: NativeDriverFixture) { self.driver = driver; self.fixture = fixture }
    func acquireProfileLease() throws { try driver.acquireProfileLease() }
    func releaseProfileLease() { driver.releaseProfileLease() }
    func createUnloaded(_ request: Code0WindowRequest, events: @escaping @MainActor (Code0WindowEvent) -> Void) throws {
        fixture?.adopt(request)
        try driver.createUnloaded(request) { [weak fixture = self.fixture] event in
            fixture?.record.receive(event, request: request)
            events(event)
        }
    }
    func navigate(_ instanceID: UUID) throws { try driver.navigate(instanceID) }
    func close(_ instanceID: UUID) { driver.close(instanceID) }
    func clearWebsiteData(completion: @escaping @MainActor (Result<Void, Code0SessionFailure>) -> Void) {
        driver.clearWebsiteData(completion: completion)
    }
}

@MainActor
private final class NativeDriverFixture {
    let root: URL
    private(set) var request: Code0WindowRequest
    let profile: NativeDriverProfile
    let driver: Code0NativeWebDriver
    let store: Code0SnapshotStore
    private(set) var record: NativeDriverRecord
    private(set) var session: Code0WebSession?
    private(set) var publications = 0
    private(set) var captureInactive: Bool?
    private(set) var visibleObserved = false
    var permissionEvidence = "unobserved"
    var sessionTermination = "unobserved"
    var sessionTimeout = false
    var deadlineEvidence = "unobserved"
    var denialEvidence = "unobserved"
    let weakObjects = NativeDriverWeakObjects()
    private var requests: [Code0WindowRequest]
    private var hiddenBefore = false
    private var unfocusedBefore = false

    init(page: Code0NativeFixturePage, mode: Code0WindowMode = .sync) throws {
        _ = NSApplication.shared
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("Code0Native-\(UUID())")
        let location = try Code0StorageLocation(applicationSupportRoot: root)
        try FileManager.default.createDirectory(at: location.directoryURL, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        let profile = NativeDriverProfile(location: location)
        let request = Code0WindowRequest(instanceID: UUID(), connectionEpoch: UUID(), nonce: UUID(), documentID: UUID(), mode: mode)
        self.root = root; self.request = request; self.requests = [request]; self.profile = profile
        self.store = Code0SnapshotStore(profile: profile)
        self.driver = Code0NativeWebDriver.fixture(profile: profile, page: page)
        self.record = NativeDriverRecord(request: request)
    }

    func prepareCachedSession(clock: NativePermissionClock) throws {
        try profile.acquire()
        let capture = try Code0WebsiteCapture.decode(filteredPayload: Data("{\"quota\":1000000,\"usedQuota\":500000,\"divisor\":500000}".utf8))
        try store.save(capture.makeSnapshot(fetchedAt: clock.wallNow))
        profile.release()
        let observer = NativeSessionDriverObserver(driver: driver, fixture: self)
        let session = Code0WebSession(clock: clock, driver: observer, store: store)
        session.onSnapshotChange = { [weak self] _ in self?.publications += 1 }
        self.session = session
    }
    func adopt(_ request: Code0WindowRequest) {
        self.request = request; requests = [request]; record = NativeDriverRecord(request: request)
    }
    func hasStage(_ stage: Code0NativePageStage) -> Bool {
        driver.fixtureObservations.contains { if case .stage(let value) = $0 { return value == stage }; return false }
    }
    func sampleCaptureState() {
        inspect { _, view in
            guard let view else { return }
            let inactive = view.cameraCaptureState == .none && view.microphoneCaptureState == .none
            captureInactive = (captureInactive ?? true) && inactive
        }
    }
    func sampleVisibility() {
        inspect { window, _ in
            if window?.isVisible == true, hasStage(.visibilityVisible) { visibleObserved = true }
        }
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
        // Preserve the original end-of-work hidden/unfocused oracle for every
        // sync case. Only the two explicit-login cases may be visible here.
        if request.mode == .sync {
            inspect { window, _ in
                hiddenAfter = window.map { !$0.isVisible } ?? true
                unfocusedAfter = window.map { !$0.isKeyWindow && !$0.isMainWindow } ?? true
            }
        }
        let cleanupObjects = requests.map { request in
            let weak = NativeDriverWeakObjects()
            let objects = driver.fixtureObjects(instanceID: request.instanceID)
            weak.window = objects.window; weak.view = objects.webView
            return weak
        }
        let hadOwnedObjects = cleanupObjects.contains { $0.window != nil || $0.view != nil }
        for request in requests { driver.close(request.instanceID) }
        for objects in cleanupObjects {
            hiddenAfter = hiddenAfter && (objects.window.map { !$0.isVisible } ?? true)
            unfocusedAfter = unfocusedAfter && (objects.window.map { !$0.isKeyWindow && !$0.isMainWindow } ?? true)
        }
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
            "displayMode": request.mode == .login ? "explicit-login" : "hidden-sync",
            "visibleObserved": visibleObserved, "denialEvidence": denialEvidence,
            "permissionEvidence": permissionEvidence, "sessionTermination": sessionTermination,
            "sessionTimeout": sessionTimeout, "sessionPublications": min(publications, 2),
            "deadline": deadlineEvidence, "requestStarted": hasStage(.requestStarted),
            "captureInactive": captureInactive.map { $0 as Any } ?? NSNull(),
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
