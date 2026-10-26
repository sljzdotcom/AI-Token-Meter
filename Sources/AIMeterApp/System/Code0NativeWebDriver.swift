import AIMeterCore
import AppKit
import Foundation
@preconcurrency import WebKit
#if AI_METER_NATIVE_FIXTURES
@preconcurrency import Network
#endif

/// One lease, one owned native instance. Construction is inert; the first load
/// is possible only after all delegates and the document-start transport exist.
@MainActor
final class Code0NativeWebDriver: NSObject, Code0WindowDriving, WKNavigationDelegate, WKUIDelegate, WKDownloadDelegate, NSWindowDelegate {
    static let officialURL = URL(string: "https://console.code0.ai/")!
    static let websiteDataStoreIdentifier = UUID(uuidString: "3292F126-DB63-49B5-9315-42C06E6CA289")!
    private static let handler = "code0Capture"
    private let profile: any Code0ProfileLeasing
    private let canonicalSourceLoader: @MainActor () throws -> String
    private var owned: Owned?
    private var store: WKWebsiteDataStore?
    private var clearing = false
    private var releaseTask: Task<Void, Never>?
    private var authorityTask: Task<Void, Never>?
    #if AI_METER_NATIVE_FIXTURES
    private var fixturePage: Code0NativeFixturePage?
    private(set) var fixtureObservations: [Code0NativeFixtureObservation] = []
    private var fixtureServer: Code0AuthenticationFixtureServer?
    #endif

    private final class Owned {
        let request: Code0WindowRequest
        let events: @MainActor (Code0WindowEvent) -> Void
        var window: NSWindow?
        var view: WKWebView?
        weak var weakWindow: NSWindow?
        weak var weakView: WKWebView?
        var proxy: Code0NativeMessageProxy?
        var active = true
        var navigated = false
        var acceptedMainDocument = false
        var failed = false
        init(request: Code0WindowRequest, events: @escaping @MainActor (Code0WindowEvent) -> Void) {
            self.request = request; self.events = events
        }
    }

    init(profile: any Code0ProfileLeasing) {
        self.profile = profile
        self.canonicalSourceLoader = Self.readCanonicalSource
        super.init()
    }
    /// Resource-failure injection before any WebKit or window object exists.
    init(profile: any Code0ProfileLeasing, canonicalSourceLoader: @escaping @MainActor () throws -> String) {
        self.profile = profile
        self.canonicalSourceLoader = canonicalSourceLoader
        super.init()
    }
    private static func readCanonicalSource() throws -> String {
        guard let resource = AppResourceLocator.url(forResource: "code0-bridge", withExtension: "js", subdirectory: "Code0"),
              let canonical = try? String(contentsOf: resource, encoding: .utf8) else { throw Code0SessionFailure.validationRequired }
        return canonical
    }
    func acquireProfileLease() throws {
        guard !clearing else { throw Code0SessionFailure.cleanupFailed }
        do { try profile.acquire() }
        catch Code0ProfileLeaseError.busy { throw Code0SessionFailure.profileBusy }
        catch { throw Code0SessionFailure.storageFailed }
    }
    func releaseProfileLease() {
        // An uncertain close must keep both the local factory and OS lease busy.
        guard owned == nil, !clearing else { return }
        profile.release()
    }
    func createUnloaded(_ request: Code0WindowRequest, events: @escaping @MainActor (Code0WindowEvent) -> Void) throws {
        guard owned == nil, !clearing else { throw Code0SessionFailure.profileBusy }
        let instance = Owned(request: request, events: events)
        owned = instance
        do {
            guard profile.isHeld else { throw Code0SessionFailure.storageFailed }
            let canonical = try canonicalSourceLoader()
            let controller = WKUserContentController()
            let proxy = Code0NativeMessageProxy(owner: self, instanceID: request.instanceID)
            controller.add(proxy, name: Self.handler)
            #if AI_METER_NATIVE_FIXTURES
            if fixturePage != nil {
                controller.add(proxy, name: "code0FixtureResult")
                if fixturePage == .quota {
                    controller.addUserScript(WKUserScript(source: Self.syntheticFetch, injectionTime: .atDocumentStart, forMainFrameOnly: true))
                }
            }
            #endif
            if request.mode == .sync {
                let config: [String: Any] = ["version": 1, "nonce": request.nonce.uuidString, "document": request.documentID.uuidString]
                let data = try JSONSerialization.data(withJSONObject: config, options: [.sortedKeys])
                guard let json = String(data: data, encoding: .utf8) else { throw Code0SessionFailure.validationRequired }
                let script = "(() => {const c=\(json);c.send=m=>window.webkit.messageHandlers.\(Self.handler).postMessage(m);(\(canonical))(c);})()"
                controller.addUserScript(WKUserScript(source: script, injectionTime: .atDocumentStart, forMainFrameOnly: true))
            }
            let configuration = WKWebViewConfiguration()
            configuration.websiteDataStore = dataStore()
            configuration.userContentController = controller
            configuration.preferences.javaScriptCanOpenWindowsAutomatically = false
            #if AI_METER_NATIVE_FIXTURES
            // This controlled page reaches the actual new-window delegate; it still
            // cannot create a window because the native return value is always nil.
            if fixturePage == .popup { configuration.preferences.javaScriptCanOpenWindowsAutomatically = true }
            #endif
            let view = WKWebView(frame: NSRect(x: 0, y: 0, width: 640, height: 720), configuration: configuration)
            view.navigationDelegate = self
            view.uiDelegate = self
            let window = NSWindow(contentRect: view.frame, styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
            window.isReleasedWhenClosed = false
            window.title = "Code0"
            window.delegate = self
            window.contentView = view
            instance.window = window; instance.view = view
            instance.weakWindow = window; instance.weakView = view; instance.proxy = proxy
            let requiredSelectors = [
                "webView:runJavaScriptAlertPanelWithMessage:initiatedByFrame:completionHandler:",
                "webView:runJavaScriptConfirmPanelWithMessage:initiatedByFrame:completionHandler:",
                "webView:runJavaScriptTextInputPanelWithPrompt:defaultText:initiatedByFrame:completionHandler:",
                "webView:runOpenPanelWithParameters:initiatedByFrame:completionHandler:",
                "webView:requestMediaCapturePermissionForOrigin:initiatedByFrame:type:decisionHandler:",
                "webView:didReceiveAuthenticationChallenge:completionHandler:",
                "webView:authenticationChallenge:shouldAllowDeprecatedTLS:",
            ]
            guard requiredSelectors.allSatisfy({ responds(to: NSSelectorFromString($0)) }) else {
                close(request.instanceID)
                throw Code0SessionFailure.validationRequired
            }
            // Reentrant callbacks may navigate or close immediately. No state is
            // changed after this acknowledgement without rechecking the instance.
            guard profile.isHeld else { throw Code0SessionFailure.storageFailed }
            monitorAuthority(instance)
            instance.events(.policiesInstalled)
        } catch {
            close(request.instanceID)
            throw error
        }
    }
    private func monitorAuthority(_ instance: Owned) {
        authorityTask?.cancel()
        authorityTask = Task { @MainActor [weak self, weak instance] in
            while !Task.isCancelled {
                do { try await Task.sleep(for: .milliseconds(100)) } catch { return }
                guard let self, let instance, self.owned === instance, instance.active else { return }
                if !self.profile.isHeld {
                    self.fail(instance, .storageFailed)
                    self.close(instance.request.instanceID)
                    return
                }
            }
        }
    }
    func navigate(_ instanceID: UUID) throws {
        guard let instance = owned, instance.active, instance.request.instanceID == instanceID,
              !instance.navigated, let view = instance.view else { throw Code0SessionFailure.navigationFailed }
        guard profile.isHeld else {
            fail(instance, .storageFailed); close(instanceID); throw Code0SessionFailure.storageFailed
        }
        instance.navigated = true
        if instance.request.mode == .login { instance.window?.makeKeyAndOrderFront(nil) }
        #if AI_METER_NATIVE_FIXTURES
        if let page = fixturePage {
            if page == .httpAuthentication {
                let server = Code0AuthenticationFixtureServer()
                fixtureServer = server
                try server.start { [weak self, weak instance] url in
                    guard let self, let instance, self.owned === instance, instance.active else { return }
                    if let url { instance.view?.load(URLRequest(url: url)) }
                    else { self.fail(instance, .navigationFailed) }
                }
            } else { view.loadHTMLString(Self.fixtureHTML(page), baseURL: Self.officialURL) }
            return
        }
        #endif
        view.load(URLRequest(url: Self.officialURL))
    }
    func close(_ instanceID: UUID) {
        guard let instance = owned, instance.request.instanceID == instanceID, instance.active else { return }
        instance.active = false
        authorityTask?.cancel(); authorityTask = nil
        instance.proxy?.owner = nil
        instance.view?.configuration.userContentController.removeScriptMessageHandler(forName: Self.handler)
        #if AI_METER_NATIVE_FIXTURES
        instance.view?.configuration.userContentController.removeScriptMessageHandler(forName: "code0FixtureResult")
        fixtureServer?.stop(); fixtureServer = nil
        #endif
        instance.view?.configuration.userContentController.removeAllUserScripts()
        instance.view?.stopLoading()
        instance.view?.navigationDelegate = nil
        instance.view?.uiDelegate = nil
        instance.view?.removeFromSuperview()
        instance.window?.delegate = nil
        instance.window?.contentView = nil
        instance.window?.close()
        instance.view = nil; instance.window = nil; instance.proxy = nil
        releaseTask?.cancel()
        releaseTask = Task { @MainActor [weak self, weak instance] in
            // Keep checking after the session's own 10s deadline: a late actual
            // release can finish cleanup, but never retroactively meet its deadline.
            while !Task.isCancelled {
                guard let self, let instance, self.owned === instance else { return }
                if instance.weakWindow == nil, instance.weakView == nil {
                    self.owned = nil
                    self.releaseTask = nil
                    instance.events(.released)
                    return
                }
                do { try await Task.sleep(for: .milliseconds(10)) } catch { return }
            }
        }
    }
    func clearWebsiteData(completion: @escaping @MainActor (Result<Void, Code0SessionFailure>) -> Void) {
        guard profile.isHeld, owned == nil, !clearing else { completion(.failure(.cleanupFailed)); return }
        clearing = true
        #if AI_METER_NATIVE_FIXTURES
        if fixturePage != nil {
            dataStore().removeData(ofTypes: WKWebsiteDataStore.allWebsiteDataTypes(), modifiedSince: .distantPast) { [weak self] in
                Task { @MainActor in
                    guard let self else { return }
                    self.store = nil; self.clearing = false
                    completion(self.profile.isHeld ? .success(()) : .failure(.cleanupFailed))
                }
            }
            return
        }
        #endif
        store = nil
        WKWebsiteDataStore.remove(forIdentifier: Self.websiteDataStoreIdentifier) { [weak self] error in
            Task { @MainActor in
                guard let self else { return }
                self.clearing = false
                completion(error == nil && self.profile.isHeld ? .success(()) : .failure(.cleanupFailed))
            }
        }
    }
    private func dataStore() -> WKWebsiteDataStore {
        if let store { return store }
        let result: WKWebsiteDataStore
        #if AI_METER_NATIVE_FIXTURES
        if fixturePage != nil { result = .nonPersistent() }
        else { result = WKWebsiteDataStore(forIdentifier: Self.websiteDataStoreIdentifier) }
        #else
        result = WKWebsiteDataStore(forIdentifier: Self.websiteDataStoreIdentifier)
        #endif
        store = result
        return result
    }
    private func current(_ view: WKWebView) -> Owned? {
        guard let instance = owned, instance.active, !instance.failed, instance.view === view else { return nil }
        guard profile.isHeld else {
            fail(instance, .storageFailed); close(instance.request.instanceID); return nil
        }
        return instance
    }
    private func fail(_ instance: Owned, _ reason: Code0SessionFailure) {
        guard owned === instance, instance.active, !instance.failed else { return }
        instance.failed = true
        instance.events(.failed(reason))
    }
    private static func isOfficial(_ url: URL?) -> Bool {
        guard let url, let parts = URLComponents(url: url, resolvingAgainstBaseURL: false) else { return false }
        return parts.scheme == "https" && parts.host?.lowercased() == "console.code0.ai" &&
            parts.user == nil && parts.password == nil && (parts.port == nil || parts.port == 443)
    }
    fileprivate func receive(_ message: WKScriptMessage, instanceID: UUID) {
        guard let instance = owned, instance.active, instance.navigated, instance.request.instanceID == instanceID,
              message.webView === instance.view, message.frameInfo.isMainFrame,
              Self.isOfficial(message.frameInfo.request.url),
              message.frameInfo.securityOrigin.protocol == "https",
              message.frameInfo.securityOrigin.host.lowercased() == "console.code0.ai",
              [0, 443].contains(message.frameInfo.securityOrigin.port) else { return }
        guard profile.isHeld else { fail(instance, .storageFailed); close(instance.request.instanceID); return }
        #if AI_METER_NATIVE_FIXTURES
        if message.name == "code0FixtureResult" {
            if let raw = message.body as? String, raw.utf8.count <= 64, fixtureObservations.count < 64 {
                if let result = Code0NativePageOutcome(rawValue: raw) { fixtureObservations.append(.page(result)) }
                else if let stage = Code0NativePageStage(rawValue: raw) { fixtureObservations.append(.stage(stage)) }
            }
            return
        }
        #endif
        // Fixture-only result observation can follow a deny callback, while
        // production quota authority is revoked immediately on failure.
        guard !instance.failed, message.name == Self.handler, instance.request.mode == .sync,
              let payload = message.body as? [String: Any], let data = encodeBridgePayload(payload),
              let source = message.frameInfo.request.url?.absoluteString else { fail(instance, .invalidResponse); return }
        instance.events(.message(payload: data, sourceURL: source, isMainFrame: true))
    }
    /// The same native envelope path, with an observable serialization boundary.
    /// This checkpoint preserves the current validation behavior for actual RED.
    func encodeBridgePayload(_ payload: [String: Any],
        serialize: ([String: Any]) throws -> Data = { try JSONSerialization.data(withJSONObject: $0) }) -> Data? {
        let allowed: Set<String> = ["version", "nonce", "document", "sequence", "kind", "request", "revision", "quota", "usedQuota", "divisor", "reason"]
        guard payload.count <= 12,
              payload.keys.allSatisfy({ $0.utf8.count <= 16 && allowed.contains($0) }),
              payload.values.allSatisfy({ value in
            if let text = value as? String { return text.utf8.count <= 128 }
            return value is NSNumber
        }), JSONSerialization.isValidJSONObject(payload),
              let data = try? serialize(payload), data.count <= 2048 else { return nil }
        return data
    }
    func webView(_ webView: WKWebView, decidePolicyFor action: WKNavigationAction,
                 decisionHandler: @escaping @MainActor (WKNavigationActionPolicy) -> Void) {
        guard let instance = current(webView) else { decisionHandler(.cancel); return }
        guard instance.navigated else { decisionHandler(.cancel); fail(instance, .policyRejected); return }
        #if AI_METER_NATIVE_FIXTURES
        if fixturePage == .popup, action.targetFrame == nil { decisionHandler(.allow); return }
        if fixturePage == .download, action.shouldPerformDownload {
            decisionHandler(.download); return
        }
        let isAuthenticationFixture = fixturePage == .httpAuthentication && fixtureServer?.url != nil && fixtureServer?.url == action.request.url
        #else
        let isAuthenticationFixture = false
        #endif
        let allowed = action.targetFrame?.isMainFrame == true && !action.shouldPerformDownload &&
            (Self.isOfficial(action.request.url) || isAuthenticationFixture) &&
            (instance.request.mode == .login || !instance.acceptedMainDocument)
        if allowed { instance.acceptedMainDocument = true; decisionHandler(.allow) }
        else {
            observeNavigation()
            decisionHandler(.cancel)
            fail(instance, .policyRejected)
        }
    }
    func webView(_ webView: WKWebView, decidePolicyFor response: WKNavigationResponse,
                 decisionHandler: @escaping @MainActor (WKNavigationResponsePolicy) -> Void) {
        guard let instance = current(webView) else { decisionHandler(.cancel); return }
        let allowed = response.isForMainFrame && response.canShowMIMEType && Self.isOfficial(response.response.url)
        decisionHandler(allowed ? .allow : .cancel)
        if !allowed { observeNavigation(); fail(instance, .policyRejected) }
    }
    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: any Error) {
        if let instance = current(webView), (error as NSError).code != NSURLErrorCancelled { fail(instance, .navigationFailed) }
    }
    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: any Error) {
        if let instance = current(webView), (error as NSError).code != NSURLErrorCancelled { fail(instance, .navigationFailed) }
    }
    func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
        if let instance = current(webView) { fail(instance, .navigationFailed) }
    }
    func windowShouldClose(_ sender: NSWindow) -> Bool {
        if let instance = owned, instance.active, instance.window === sender { fail(instance, .navigationFailed) }
        return false
    }
    func webView(_ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration,
                 for navigationAction: WKNavigationAction, windowFeatures: WKWindowFeatures) -> WKWebView? {
        if let instance = current(webView) { observeNewWindow(); fail(instance, .policyRejected) }
        return nil
    }
    // Use WebKit's public async requirement to avoid the SDK's ambiguous closure
    // isolation import. The ObjC optional selector is checked by real fixtures.
    func webView(_ webView: WKWebView, runJavaScriptAlertPanelWithMessage message: String,
                 initiatedByFrame frame: WKFrameInfo) async {
        if let instance = current(webView) { observeAlert(); fail(instance, .policyRejected) }
    }
    func webView(_ webView: WKWebView, runJavaScriptConfirmPanelWithMessage message: String,
                 initiatedByFrame frame: WKFrameInfo) async -> Bool {
        if let instance = current(webView) { observeConfirm(); fail(instance, .policyRejected) }
        return false
    }
    func webView(_ webView: WKWebView, runJavaScriptTextInputPanelWithPrompt prompt: String, defaultText: String?,
                 initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping @MainActor (String?) -> Void) {
        if let instance = current(webView) { observePrompt(); fail(instance, .policyRejected) }
        completionHandler(nil)
    }
    func webView(_ webView: WKWebView, runOpenPanelWith parameters: WKOpenPanelParameters,
                 initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping @MainActor ([URL]?) -> Void) {
        if let instance = current(webView) { observeFile(); fail(instance, .policyRejected) }
        completionHandler(nil)
    }
    func webView(_ webView: WKWebView, requestMediaCapturePermissionFor origin: WKSecurityOrigin,
                 initiatedByFrame frame: WKFrameInfo, type: WKMediaCaptureType,
                 decisionHandler: @escaping @MainActor (WKPermissionDecision) -> Void) {
        if let instance = current(webView) { observeMedia(); fail(instance, .policyRejected) }
        decisionHandler(.deny)
    }
    func webView(_ webView: WKWebView, didReceive challenge: URLAuthenticationChallenge,
                 completionHandler: @escaping @MainActor (URLSession.AuthChallengeDisposition, URLCredential?) -> Void) {
        let serverTrust = challenge.protectionSpace.authenticationMethod == NSURLAuthenticationMethodServerTrust
        if let instance = current(webView) {
            observeAuthentication(challenge.protectionSpace.authenticationMethod)
            if !serverTrust { fail(instance, .policyRejected) }
        }
        completionHandler(serverTrust && current(webView) != nil ? .performDefaultHandling : .cancelAuthenticationChallenge, nil)
    }
    func webView(_ webView: WKWebView, shouldAllowDeprecatedTLSFor challenge: URLAuthenticationChallenge) async -> Bool { false }
    func webView(_ webView: WKWebView, navigationAction: WKNavigationAction, didBecome download: WKDownload) {
        refuseDownload(webView, download: download)
    }
    func webView(_ webView: WKWebView, navigationResponse: WKNavigationResponse, didBecome download: WKDownload) {
        refuseDownload(webView, download: download)
    }
    private func refuseDownload(_ view: WKWebView, download: WKDownload) {
        download.delegate = self
        if let instance = current(view) { observeDownload(); fail(instance, .policyRejected) }
        download.cancel(nil)
    }
    func download(_ download: WKDownload, decideDestinationUsing response: URLResponse,
                  suggestedFilename: String, completionHandler: @escaping @MainActor (URL?) -> Void) { completionHandler(nil) }
    func download(_ download: WKDownload, willPerformHTTPRedirection response: HTTPURLResponse,
                  newRequest request: URLRequest, decisionHandler: @escaping @MainActor (WKDownload.RedirectPolicy) -> Void) { decisionHandler(.cancel) }
    func download(_ download: WKDownload, didReceive challenge: URLAuthenticationChallenge,
                  completionHandler: @escaping @MainActor (URLSession.AuthChallengeDisposition, URLCredential?) -> Void) {
        completionHandler(.cancelAuthenticationChallenge, nil)
    }

    // These no-op instrumentation methods compile away from ordinary builds.
    private func observeNavigation() { observe("navigation") }
    private func observeNewWindow() { observe("newWindow") }
    private func observeAlert() { observe("alert") }
    private func observeConfirm() { observe("confirm") }
    private func observePrompt() { observe("prompt") }
    private func observeFile() { observe("filePanel") }
    private func observeMedia() { observe("mediaCapture") }
    private func observeDownload() { observe("download") }
    private func observeAuthentication(_ method: String) {
        if method == NSURLAuthenticationMethodServerTrust { observe("serverTrust") }
        else if method == NSURLAuthenticationMethodClientCertificate { observe("clientCertificate") }
        else { observe("httpAuthentication") }
    }
    private func observe(_ name: String) {
        #if AI_METER_NATIVE_FIXTURES
        guard fixturePage != nil, fixtureObservations.count < 64 else { return }
        let kinds: [String: Code0NativePolicyCallback] = ["navigation": .navigation, "newWindow": .newWindow,
            "alert": .alert, "confirm": .confirm, "prompt": .prompt, "filePanel": .filePanel, "mediaCapture": .mediaCapture,
            "download": .download, "httpAuthentication": .httpAuthentication, "clientCertificate": .clientCertificate, "serverTrust": .serverTrust]
        if let kind = kinds[name] { fixtureObservations.append(.delegate(kind)) }
        #endif
    }
    #if AI_METER_NATIVE_FIXTURES
    static func fixture(profile: any Code0ProfileLeasing, page: Code0NativeFixturePage) -> Code0NativeWebDriver {
        let driver = Code0NativeWebDriver(profile: profile); driver.fixturePage = page; return driver
    }
    func fixtureObjects(instanceID: UUID) -> (window: NSWindow?, webView: WKWebView?) {
        guard let owned, owned.request.instanceID == instanceID else { return (nil, nil) }
        return (owned.view == nil ? owned.weakWindow : owned.window, owned.view ?? owned.weakView)
    }
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        guard current(webView) != nil, fixturePage == .filePanel else { return }
        // evaluateJavaScript on this one controlled file page supplies WebKit's
        // own script-triggered chooser path. Display-media never uses this path.
        webView.evaluateJavaScript("document.getElementById('file').click()", completionHandler: nil)
    }
    private static let syntheticFetch = #"""
    (() => {
      function response(url, text) {
        const result = new Response(text, {status:200,headers:{'content-type':'application/json'}});
        Object.defineProperty(result,'url',{value:url});
        Object.defineProperty(result,'clone',{value:()=>response(url,text)});
        return result;
      }
      window.fetch = input => {
        const url = new URL(String(input),location.href).href;
        if(url === 'https://console.code0.ai/api/status') return Promise.resolve(response(url,'{"success":true,"data":{"quota_per_unit":500000}}'));
        if(url === 'https://console.code0.ai/api/user/self') return Promise.resolve(response(url,'{"success":true,"data":{"quota":1250000,"used_quota":750000}}'));
        return Promise.reject(new Error('unsupported-fixture'));
      };
    })();
    """#
    private static func fixtureHTML(_ page: Code0NativeFixturePage) -> String {
        let script: String
        switch page {
        case .quota: script = "fetch('/api/status');fetch('/api/user/self');"
        case .alert: script = "alert('fixture');"
        case .confirm: script = "confirm('fixture');"
        case .prompt: script = "prompt('fixture');"
        case .popup: script = "window.open('about:blank');"
        case .filePanel: script = ""
        case .externalNavigation: script = "location.href='https://invalid.example/';"
        case .download: script = "const a=document.createElement('a');a.href=URL.createObjectURL(new Blob(['fixture'],{type:'application/octet-stream'}));a.download='fixture.txt';a.click();"
        case .media: script = "if(!navigator.mediaDevices?.getUserMedia) done('unsupported');else { stage('request-started');setTimeout(()=>stage('page-timer-fired'),1000);mediaResult(navigator.mediaDevices.getUserMedia({video:true,audio:true})); }"
        case .geolocation: script = "if(!navigator.geolocation)done('unsupported');else { stage('request-started');setTimeout(()=>stage('page-timer-fired'),1000);navigator.geolocation.getCurrentPosition(()=>done('resolved'),e=>done(e.code===1?'permissionDenied':(e.code===2?'positionUnavailable':(e.code===3?'positionTimedOut':'otherError'))),{timeout:2000}); }"
        case .notification: script = "if(typeof Notification==='undefined')done('unsupported');else Notification.requestPermission().then(v=>done(v==='granted'?'resolved':'defaultResult'),()=>done('rejected'));"
        case .hiddenDisplay: script = "if(!navigator.mediaDevices?.getDisplayMedia)done('unsupported');else result(navigator.mediaDevices.getDisplayMedia({video:true}));"
        case .httpAuthentication: script = ""
        }
        return """
        <!doctype html><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none';script-src 'unsafe-inline';connect-src 'none';media-src 'none'">
        <input type="file" id="file"><script>
        function done(value){window.webkit.messageHandlers.code0FixtureResult.postMessage(value);}
        function stage(value){window.webkit.messageHandlers.code0FixtureResult.postMessage('stage-'+value);}
        function result(promise){promise.then(stream=>{stream?.getTracks?.().forEach(t=>t.stop());done('resolved');},()=>done('rejected'));}
        function mediaResult(promise){promise.then(stream=>{done('resolved');stream?.getTracks?.().forEach(t=>t.stop());},e=>{
          done(e.name==='NotAllowedError'?'notAllowed':(e.name==='SecurityError'?'securityError':(e.name==='NotFoundError'?'notFound':(e.name==='NotReadableError'?'notReadable':(e.name==='AbortError'?'abortError':'otherError')))));
        });}
        function visibility(){stage(document.visibilityState==='hidden'?'visibility-hidden':(document.visibilityState==='visible'?'visibility-visible':'visibility-other'));}
        document.addEventListener('visibilitychange',visibility);
        window.addEventListener('error',()=>stage('script-error'));
        window.addEventListener('unhandledrejection',()=>stage('promise-error'));
        stage('script-started');
        visibility();
        stage(document.hasFocus()?'focus-present':'focus-absent');
        \(script)
        </script>
        """
    }
    #endif
}

@MainActor
private final class Code0NativeMessageProxy: NSObject, WKScriptMessageHandler {
    weak var owner: Code0NativeWebDriver?
    let instanceID: UUID
    init(owner: Code0NativeWebDriver, instanceID: UUID) { self.owner = owner; self.instanceID = instanceID }
    func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
        owner?.receive(message, instanceID: instanceID)
    }
}

#if AI_METER_NATIVE_FIXTURES
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
    case permissionDenied, positionUnavailable, positionTimedOut
    case notAllowed, securityError, notFound, notReadable, abortError, otherError
}
enum Code0NativePageStage: String, Sendable {
    case scriptStarted = "stage-script-started"
    case requestStarted = "stage-request-started"
    case pageTimerFired = "stage-page-timer-fired"
    case scriptError = "stage-script-error"
    case promiseError = "stage-promise-error"
    case visibilityHidden = "stage-visibility-hidden"
    case visibilityVisible = "stage-visibility-visible"
    case visibilityOther = "stage-visibility-other"
    case focusPresent = "stage-focus-present"
    case focusAbsent = "stage-focus-absent"
}
enum Code0NativeFixtureObservation: Sendable {
    case delegate(Code0NativePolicyCallback)
    case page(Code0NativePageOutcome)
    case stage(Code0NativePageStage)
}

/// Only fixture-compiled code can bind a loopback listener. The one native-owned
/// URL serves a fixed 401; no account data, TLS exception or production URL flag.
@MainActor
final class Code0AuthenticationFixtureServer {
    private var listener: NWListener?
    private var ready: (@MainActor (URL?) -> Void)?
    private(set) var url: URL?
    private var connections: [UUID: Client] = [:]
    var acceptedConnectionCount: Int { connections.count }
    private final class Client {
        let connection: NWConnection
        var line = Data()
        var timeout: Task<Void, Never>?
        init(_ connection: NWConnection) { self.connection = connection }
    }
    func start(ready: @escaping @MainActor (URL?) -> Void) throws {
        guard listener == nil else { throw Code0SessionFailure.navigationFailed }
        let parameters = NWParameters.tcp
        parameters.requiredLocalEndpoint = .hostPort(host: "127.0.0.1", port: .any)
        let listener = try NWListener(using: parameters)
        self.listener = listener; self.ready = ready
        let path = "/fixture-\(UUID().uuidString)"
        listener.stateUpdateHandler = { [weak self, weak listener] state in
            Task { @MainActor in
                guard let self, self.listener === listener else { return }
                switch state {
                case .ready:
                    guard let port = listener?.port else { return }
                    let url = URL(string: "http://127.0.0.1:\(port.rawValue)\(path)")!
                    self.url = url; let callback = self.ready; self.ready = nil; callback?(url)
                case .failed:
                    let callback = self.ready; self.ready = nil; callback?(nil)
                default: break
                }
            }
        }
        listener.newConnectionHandler = { [weak self, weak listener] connection in
            Task { @MainActor in
                guard let self, self.listener === listener, listener != nil else { connection.cancel(); return }
                self.accept(connection, path: path)
            }
        }
        listener.start(queue: DispatchQueue(label: "Code0.synthetic-auth-listener"))
    }
    private func accept(_ connection: NWConnection, path: String) {
        let id = UUID(), client = Client(connection)
        connections[id] = client
        client.timeout = Task { @MainActor [weak self] in
            do { try await Task.sleep(for: .seconds(2)) } catch { return }
            self?.finish(id)
        }
        connection.start(queue: DispatchQueue(label: "Code0.synthetic-auth-connection"))
        receive(id, path: path)
    }
    private func receive(_ id: UUID, path: String) {
        guard let client = connections[id] else { return }
        client.connection.receive(minimumIncompleteLength: 1, maximumLength: 256) { [weak self] bytes, _, complete, error in
            Task { @MainActor in
                guard let self, let client = self.connections[id] else { return }
                if let bytes {
                    // Inspect only the bounded first request line; headers and
                    // credentials are neither parsed, retained nor logged.
                    for byte in bytes {
                        guard client.line.count < 256 else { self.respond(id, authorized: false); return }
                        client.line.append(byte)
                        if byte == 10 {
                            let exact = client.line == Data("GET \(path) HTTP/1.1\r\n".utf8)
                            self.respond(id, authorized: exact)
                            return
                        }
                    }
                }
                if complete || error != nil { self.finish(id) }
                else { self.receive(id, path: path) }
            }
        }
    }
    private func respond(_ id: UUID, authorized: Bool) {
        guard let client = connections[id] else { return }
        let first = authorized ? "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"Code0Synthetic\"\r\n" : "HTTP/1.1 404 Not Found\r\n"
        let data = Data((first + "Content-Length: 0\r\nConnection: close\r\n\r\n").utf8)
        client.connection.send(content: data, completion: .contentProcessed { [weak self] _ in
            Task { @MainActor in self?.finish(id) }
        })
    }
    private func finish(_ id: UUID) {
        guard let client = connections.removeValue(forKey: id) else { return }
        client.timeout?.cancel(); client.connection.cancel()
    }
    func stop() {
        ready = nil; listener?.stateUpdateHandler = nil; listener?.newConnectionHandler = nil
        listener?.cancel(); listener = nil
        for id in Array(connections.keys) { finish(id) }
    }
}
#endif
