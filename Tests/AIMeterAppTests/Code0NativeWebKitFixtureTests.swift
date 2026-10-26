#if AI_METER_NATIVE_FIXTURES
import AppKit
import CoreFoundation
import Foundation
import Testing
@preconcurrency import WebKit

@Suite("Code0 actual WebKit origin probe", .serialized,
       .enabled(if: ProcessInfo.processInfo.environment["AI_METER_SCREEN_TESTS"] == "1"))
@MainActor
struct Code0NativeWebKitFixtureTests {
    @Test("Inline HTML records actual native source without website requests or permission requests")
    func inlineHTMLReportsOwnedNativeOriginAndReleasesHiddenObjects() async {
        _ = NSApplication.shared
        let evidence = Code0OriginProbeEvidence()
        let weakObjects = Code0OriginProbeWeakObjects()
        let owner = Code0OriginProbeOwner(evidence: evidence, weakObjects: weakObjects)
        defer { owner.tearDown() }

        owner.recordVisibility(before: true)
        owner.loadInlineHTML()
        let finished = await waitForProbe(seconds: 15) { evidence.received || evidence.failure != nil }
        if !finished { evidence.fail("timeout") }
        if !evidence.allowedInitialNavigation { evidence.fail("initial-carrier-unobserved") }
        owner.recordVisibility(before: false)
        owner.tearDown()
        _ = await waitForProbe(seconds: 5) { weakObjects.webView == nil && weakObjects.window == nil }
        evidence.viewReleased = weakObjects.webView == nil
        evidence.windowReleased = weakObjects.window == nil
        if !evidence.viewReleased || !evidence.windowReleased { evidence.fail("cleanup-failed") }
        evidence.emit()

        #expect(finished, "The bounded origin probe must produce an observation or refusal")
        #expect(evidence.received, "A real owned main-frame script message must arrive")
        #expect(evidence.allowedInitialNavigation, "Only the one preselected inline carrier may navigate")
        #expect(evidence.failure == nil, "Read CODE0_NATIVE_PROBE for the fixed failure classification")
        #expect(evidence.hiddenBefore && evidence.hiddenAfter)
        #expect(evidence.unfocusedBefore && evidence.unfocusedAfter)
        #expect(evidence.viewReleased, "WKWebView must become actually weak nil")
        #expect(evidence.windowReleased, "NSWindow must become actually weak nil")
        // officialSourceObserved is deliberately an observation, not an assertion:
        // an inline document is not a real official HTTPS/account acceptance test.
    }

    private func waitForProbe(seconds: Int, condition: @MainActor () -> Bool) async -> Bool {
        let deadline = ContinuousClock.now.advanced(by: .seconds(seconds))
        while !condition(), ContinuousClock.now < deadline {
            do { try await Task.sleep(for: .milliseconds(10)) }
            catch { return condition() }
        }
        return condition()
    }
}

@MainActor
private final class Code0OriginProbeWeakObjects {
    weak var webView: WKWebView?
    weak var window: NSWindow?
}

@MainActor
private final class Code0OriginProbeEvidence {
    var stage = "created"
    var failure: String?
    var initialNavigation = "unobserved"
    var allowedInitialNavigation = false
    var navigationFinished = false
    var received = false
    var owned = false
    var mainFrame = false
    var nativeURL = "unobserved"
    var nativeProtocol = "unobserved"
    var nativeHost = "unobserved"
    var nativePort = "unobserved"
    var officialSourceObserved = false
    var pageOrigin = "unobserved"
    var pageDocument = "unobserved"
    var api: [String: Bool] = [:]
    var hiddenBefore = false
    var hiddenAfter = false
    var unfocusedBefore = false
    var unfocusedAfter = false
    var viewReleased = false
    var windowReleased = false
    var unexpectedPolicyCallback = false

    func fail(_ fixedReason: String) { if failure == nil { failure = fixedReason } }

    static func classifyURL(_ url: URL?) -> String {
        guard let url else { return "missing" }
        if url.absoluteString == "https://console.code0.ai/" { return "official-root" }
        if url.absoluteString == "about:blank" { return "about-blank" }
        return "unexpected"
    }

    func emit() {
        let complete = received && allowedInitialNavigation && failure == nil && hiddenBefore && hiddenAfter &&
            unfocusedBefore && unfocusedAfter && viewReleased && windowReleased
        var fields: [String: Any] = [
            "schema": 1, "result": complete ? "completed-observation" : "failed-observation",
            "stage": stage, "failure": failure ?? "none", "initialNavigation": initialNavigation,
            "allowedInitialNavigation": allowedInitialNavigation, "navigationFinished": navigationFinished,
            "received": received, "owned": owned, "mainFrame": mainFrame,
            "nativeURL": nativeURL, "nativeProtocol": nativeProtocol, "nativeHost": nativeHost,
            "nativePort": nativePort, "officialSourceObserved": officialSourceObserved,
            "pageOrigin": pageOrigin, "pageDocument": pageDocument,
            "hiddenBefore": hiddenBefore, "hiddenAfter": hiddenAfter,
            "unfocusedBefore": unfocusedBefore, "unfocusedAfter": unfocusedAfter,
            "viewReleased": viewReleased, "windowReleased": windowReleased,
            "unexpectedPolicyCallback": unexpectedPolicyCallback,
        ]
        for key in ["secure", "geolocation", "notification", "mediaDevices", "userMedia", "displayMedia"] {
            fields[key] = api[key].map { $0 as Any } ?? NSNull()
        }
        guard let data = try? JSONSerialization.data(withJSONObject: fields, options: [.sortedKeys]),
              data.count <= 2048, let line = String(data: data, encoding: .utf8) else {
            Issue.record("The fixed probe evidence exceeded its bounded JSON schema")
            print("CODE0_NATIVE_PROBE {\"schema\":1,\"result\":\"evidence-encoding-failed\"}")
            return
        }
        print("CODE0_NATIVE_PROBE \(line)")
    }
}

@MainActor
private final class Code0OriginProbeMessageProxy: NSObject, WKScriptMessageHandler {
    weak var owner: Code0OriginProbeOwner?
    func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
        owner?.receive(message)
    }
}

@MainActor
private final class Code0OriginProbeOwner: NSObject, WKNavigationDelegate, WKUIDelegate, WKDownloadDelegate {
    private static let handlerName = "code0OriginProbe"
    private static let baseURL = URL(string: "https://console.code0.ai/")!
    private let evidence: Code0OriginProbeEvidence
    private var webView: WKWebView?
    private var window: NSWindow?
    private var proxy: Code0OriginProbeMessageProxy?
    private var active = true
    private var attemptedLoad = false
    private var responseAllowed = false
    private var navigationDecided = false

    init(evidence: Code0OriginProbeEvidence, weakObjects: Code0OriginProbeWeakObjects) {
        self.evidence = evidence
        super.init()
        let proxy = Code0OriginProbeMessageProxy()
        proxy.owner = self
        self.proxy = proxy
        let controller = WKUserContentController()
        controller.add(proxy, name: Self.handlerName)
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        configuration.userContentController = controller
        configuration.preferences.javaScriptCanOpenWindowsAutomatically = false
        configuration.preferences.isElementFullscreenEnabled = false
        configuration.mediaTypesRequiringUserActionForPlayback = .all
        configuration.allowsAirPlayForMediaPlayback = false
        let view = WKWebView(frame: NSRect(x: 0, y: 0, width: 320, height: 180), configuration: configuration)
        view.navigationDelegate = self
        view.uiDelegate = self
        let hiddenWindow = NSWindow(contentRect: view.frame, styleMask: .borderless, backing: .buffered, defer: false)
        hiddenWindow.isReleasedWhenClosed = false
        hiddenWindow.isRestorable = false
        hiddenWindow.isExcludedFromWindowsMenu = true
        hiddenWindow.contentView = view
        self.webView = view
        self.window = hiddenWindow
        weakObjects.webView = view
        weakObjects.window = hiddenWindow
    }

    func recordVisibility(before: Bool) {
        let hidden = window.map { !$0.isVisible } ?? false
        let unfocused = window.map { !$0.isKeyWindow && !$0.isMainWindow && NSApplication.shared.keyWindow !== $0 } ?? false
        if before { evidence.hiddenBefore = hidden; evidence.unfocusedBefore = unfocused }
        else { evidence.hiddenAfter = hidden; evidence.unfocusedAfter = unfocused }
        if !hidden || !unfocused { evidence.fail("visibility-failed") }
    }

    func loadInlineHTML() {
        guard active, !attemptedLoad else { evidence.fail("duplicate-load"); return }
        attemptedLoad = true
        evidence.stage = "loading-inline-html"
        webView?.loadHTMLString(Self.html, baseURL: Self.baseURL)
    }

    func receive(_ message: WKScriptMessage) {
        guard active else { return }
        evidence.stage = "script-message"
        evidence.owned = message.webView === webView
        evidence.mainFrame = message.frameInfo.isMainFrame
        evidence.nativeURL = Code0OriginProbeEvidence.classifyURL(message.frameInfo.request.url)
        let origin = message.frameInfo.securityOrigin
        evidence.nativeProtocol = origin.protocol == "https" ? "https" : (origin.protocol.isEmpty ? "empty" : "other")
        evidence.nativeHost = origin.host == "console.code0.ai" ? "official" : (origin.host.isEmpty ? "empty" : "other")
        evidence.nativePort = origin.port == 0 ? "zero" : (origin.port == 443 ? "https443" : "other")
        evidence.officialSourceObserved = evidence.owned && evidence.mainFrame && evidence.nativeURL == "official-root" &&
            origin.protocol == "https" && origin.host == "console.code0.ai" && (origin.port == 0 || origin.port == 443)
        guard !evidence.received, evidence.owned, evidence.mainFrame, message.name == Self.handlerName,
              let body = message.body as? String, let data = body.data(using: .utf8), data.count <= 2048,
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              Set(object.keys) == ["version", "origin", "documentURL", "secure", "geolocation", "notification", "mediaDevices", "userMedia", "displayMedia"],
              let version = object["version"] as? NSNumber, CFGetTypeID(version) != CFBooleanGetTypeID(), version.doubleValue == 1,
              let pageOrigin = object["origin"] as? String, let pageDocument = object["documentURL"] as? String else {
            evidence.fail("message-rejected")
            return
        }
        var api: [String: Bool] = [:]
        for key in ["secure", "geolocation", "notification", "mediaDevices", "userMedia", "displayMedia"] {
            guard let number = object[key] as? NSNumber, CFGetTypeID(number) == CFBooleanGetTypeID() else {
                evidence.fail("message-rejected")
                return
            }
            api[key] = number.boolValue
        }
        evidence.api = api
        evidence.pageOrigin = pageOrigin == "https://console.code0.ai" ? "official" : (pageOrigin == "null" ? "opaque" : "unexpected")
        evidence.pageDocument = Code0OriginProbeEvidence.classifyURL(URL(string: pageDocument))
        evidence.received = true
        evidence.stage = "owned-main-frame-observed"
    }

    func tearDown() {
        guard active else { return }
        active = false
        proxy?.owner = nil
        webView?.configuration.userContentController.removeScriptMessageHandler(forName: Self.handlerName)
        webView?.stopLoading()
        webView?.navigationDelegate = nil
        webView?.uiDelegate = nil
        webView?.removeFromSuperview()
        window?.contentView = nil
        window?.close()
        webView = nil
        window = nil
        proxy = nil
    }

    func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction,
                 decisionHandler: @escaping @MainActor (WKNavigationActionPolicy) -> Void) {
        let classification = Code0OriginProbeEvidence.classifyURL(navigationAction.request.url)
        if !navigationDecided { evidence.initialNavigation = classification }
        let allowed = active && attemptedLoad && !navigationDecided && webView === self.webView &&
            navigationAction.targetFrame?.isMainFrame == true && navigationAction.navigationType == .other &&
            classification == "official-root" && !navigationAction.shouldPerformDownload
        navigationDecided = true
        if allowed {
            evidence.allowedInitialNavigation = true
            evidence.stage = "initial-carrier-allowed"
        } else {
            evidence.stage = "navigation-rejected"
            evidence.fail("unexpected-navigation")
        }
        decisionHandler(allowed ? .allow : .cancel)
    }

    func webView(_ webView: WKWebView, decidePolicyFor navigationResponse: WKNavigationResponse,
                 decisionHandler: @escaping @MainActor (WKNavigationResponsePolicy) -> Void) {
        let allowed = active && evidence.allowedInitialNavigation && !responseAllowed &&
            webView === self.webView && navigationResponse.isForMainFrame && navigationResponse.canShowMIMEType &&
            Code0OriginProbeEvidence.classifyURL(navigationResponse.response.url) == "official-root" &&
            navigationResponse.response.mimeType == "text/html"
        responseAllowed = true
        if !allowed { evidence.stage = "navigation-rejected"; evidence.fail("unexpected-response") }
        decisionHandler(allowed ? .allow : .cancel)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        guard active, webView === self.webView else { return }
        evidence.navigationFinished = true
    }
    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: any Error) {
        guard active, webView === self.webView else { return }
        evidence.fail("navigation-failed")
    }
    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: any Error) {
        guard active, webView === self.webView else { return }
        evidence.fail("navigation-failed")
    }
    func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
        guard active, webView === self.webView else { return }
        evidence.fail("content-terminated")
    }

    private func refuseUnexpectedPolicy() { evidence.unexpectedPolicyCallback = true; evidence.fail("unexpected-policy-callback") }

    func webView(_ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration,
                 for navigationAction: WKNavigationAction, windowFeatures: WKWindowFeatures) -> WKWebView? {
        refuseUnexpectedPolicy()
        return nil
    }
    func webView(_ webView: WKWebView, runJavaScriptAlertPanelWithMessage message: String,
                 initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping @MainActor () -> Void) {
        refuseUnexpectedPolicy(); completionHandler()
    }
    func webView(_ webView: WKWebView, runJavaScriptConfirmPanelWithMessage message: String,
                 initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping @MainActor (Bool) -> Void) {
        refuseUnexpectedPolicy(); completionHandler(false)
    }
    func webView(_ webView: WKWebView, runJavaScriptTextInputPanelWithPrompt prompt: String, defaultText: String?,
                 initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping @MainActor (String?) -> Void) {
        refuseUnexpectedPolicy(); completionHandler(nil)
    }
    func webView(_ webView: WKWebView, runOpenPanelWith parameters: WKOpenPanelParameters,
                 initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping @MainActor ([URL]?) -> Void) {
        refuseUnexpectedPolicy(); completionHandler(nil)
    }
    func webView(_ webView: WKWebView, requestMediaCapturePermissionFor origin: WKSecurityOrigin,
                 initiatedByFrame frame: WKFrameInfo, type: WKMediaCaptureType,
                 decisionHandler: @escaping @MainActor (WKPermissionDecision) -> Void) {
        refuseUnexpectedPolicy(); decisionHandler(.deny)
    }
    func webView(_ webView: WKWebView, didReceive challenge: URLAuthenticationChallenge,
                 completionHandler: @escaping @MainActor (URLSession.AuthChallengeDisposition, URLCredential?) -> Void) {
        refuseUnexpectedPolicy()
        completionHandler(challenge.protectionSpace.authenticationMethod == NSURLAuthenticationMethodServerTrust
            ? .performDefaultHandling : .cancelAuthenticationChallenge, nil)
    }
    func webView(_ webView: WKWebView, authenticationChallenge challenge: URLAuthenticationChallenge,
                 shouldAllowDeprecatedTLS decisionHandler: @escaping @MainActor (Bool) -> Void) {
        refuseUnexpectedPolicy(); decisionHandler(false)
    }
    func webView(_ webView: WKWebView, navigationAction: WKNavigationAction, didBecome download: WKDownload) {
        refuseUnexpectedPolicy(); download.delegate = self; download.cancel(nil)
    }
    func webView(_ webView: WKWebView, navigationResponse: WKNavigationResponse, didBecome download: WKDownload) {
        refuseUnexpectedPolicy(); download.delegate = self; download.cancel(nil)
    }
    func download(_ download: WKDownload, decideDestinationUsing response: URLResponse,
                  suggestedFilename: String, completionHandler: @escaping @MainActor (URL?) -> Void) {
        refuseUnexpectedPolicy(); completionHandler(nil)
    }
    func download(_ download: WKDownload, willPerformHTTPRedirection response: HTTPURLResponse,
                  newRequest request: URLRequest, decisionHandler: @escaping @MainActor (WKDownload.RedirectPolicy) -> Void) {
        refuseUnexpectedPolicy(); decisionHandler(.cancel)
    }
    func download(_ download: WKDownload, didReceive challenge: URLAuthenticationChallenge,
                  completionHandler: @escaping @MainActor (URLSession.AuthChallengeDisposition, URLCredential?) -> Void) {
        refuseUnexpectedPolicy(); completionHandler(.cancelAuthenticationChallenge, nil)
    }

    private static let html = #"""
    <!doctype html><html><head>
    <meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; connect-src 'none'">
    <title>Owned inline Code0 origin probe</title></head><body><script>
    (() => {
      const devices = navigator.mediaDevices;
      window.webkit.messageHandlers.code0OriginProbe.postMessage(JSON.stringify({
        version: 1, origin: location.origin, documentURL: document.URL,
        secure: globalThis.isSecureContext === true,
        geolocation: typeof navigator.geolocation === 'object',
        notification: typeof globalThis.Notification === 'function',
        mediaDevices: typeof devices === 'object',
        userMedia: !!devices && typeof devices.getUserMedia === 'function',
        displayMedia: !!devices && typeof devices.getDisplayMedia === 'function'
      }));
    })();
    </script></body></html>
    """#
}
#endif
