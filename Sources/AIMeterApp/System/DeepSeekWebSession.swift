import AIMeterCore
import Foundation
import Observation
@preconcurrency import WebKit

@MainActor
@Observable
final class DeepSeekWebSession: NSObject, WKNavigationDelegate, WKScriptMessageHandler {
    enum SyncState: Equatable {
        case signedOut
        case loading
        case ready
        case stale(String)
    }

    private static let officialHost = "platform.deepseek.com"
    static let usageURL = URL(string: "https://platform.deepseek.com/usage")!

    private let historyStore: DeepSeekHistoryStore
    private let parser = DeepSeekWebsitePayloadParser()
    private let normalizer = DeepSeekHistoryNormalizer()
    private var usageAccumulator = DeepSeekUsageFacetAccumulator()
    private var staleTask: Task<Void, Never>?

    private(set) var state: SyncState
    private(set) var history: DeepSeekUsageHistory?
    @ObservationIgnored var onHistoryChange: ((DeepSeekUsageHistory) -> Void)?
    @ObservationIgnored let webView: WKWebView

    var shouldPauseAutoHide: Bool {
        switch state {
        case .signedOut: true
        case .loading: history == nil
        case .ready, .stale: false
        }
    }

    init(historyStore: DeepSeekHistoryStore) {
        self.historyStore = historyStore
        let cached = try? historyStore.load()
        history = cached
        state = cached == nil ? .signedOut : .stale("Showing cached usage")

        let controller = WKUserContentController()
        controller.addUserScript(WKUserScript(
            source: Self.responseBridgeScript,
            injectionTime: .atDocumentStart,
            forMainFrameOnly: true
        ))
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .default()
        configuration.userContentController = controller
        webView = WKWebView(frame: .zero, configuration: configuration)
        super.init()
        controller.add(self, name: "deepSeekUsage")
        webView.navigationDelegate = self
    }

    isolated deinit {
        staleTask?.cancel()
        webView.configuration.userContentController.removeScriptMessageHandler(forName: "deepSeekUsage")
        webView.navigationDelegate = nil
    }

    func syncIfNeeded(force: Bool = false) {
        if !force,
           let history,
           Date().timeIntervalSince(history.updatedAt) < 30 * 60 {
            state = .ready
            return
        }
        state = .loading
        usageAccumulator = DeepSeekUsageFacetAccumulator()
        webView.load(URLRequest(url: Self.usageURL))
        scheduleStaleFallback()
    }

    func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
        guard message.name == "deepSeekUsage",
              let frameHost = message.frameInfo.request.url?.host?.lowercased(),
              frameHost == Self.officialHost,
              let payload = message.body as? [String: Any],
              payload["origin"] as? String == "https://\(Self.officialHost)",
              let sourceURLText = payload["url"] as? String,
              let sourceURL = URL(string: sourceURLText),
              sourceURL.scheme == "https",
              sourceURL.host?.lowercased() == Self.officialHost,
              let body = payload["body"] as? String,
              let data = body.data(using: .utf8),
              data.count <= 2 * 1_024 * 1_024,
              let facet = DeepSeekUsageFacet(responseURL: sourceURL),
              let records = try? parser.parse(data),
              let completeRecords = usageAccumulator.replace(facet, rows: records) else {
            return
        }

        let normalized = normalizer.normalize(
            records: completeRecords,
            endingAt: Date(),
            updatedAt: Date(),
            statusMessage: "Synced from DeepSeek"
        )
        do {
            try historyStore.save(normalized)
            history = normalized
            state = .ready
            staleTask?.cancel()
            onHistoryChange?(normalized)
        } catch {
            history = normalized
            state = .stale("Usage is visible but could not be cached")
            onHistoryChange?(normalized)
        }
    }

    func webView(
        _ webView: WKWebView,
        decidePolicyFor navigationAction: WKNavigationAction,
        decisionHandler: @escaping @MainActor (WKNavigationActionPolicy) -> Void
    ) {
        guard let url = navigationAction.request.url else {
            decisionHandler(.cancel)
            return
        }
        if url.scheme == "about" || (url.scheme == "https" && url.host?.lowercased() == Self.officialHost) {
            decisionHandler(.allow)
        } else {
            decisionHandler(.cancel)
        }
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        guard let url = webView.url else { return }
        let path = url.path.lowercased()
        if path.contains("sign_in") || path.contains("login") {
            state = .signedOut
            staleTask?.cancel()
        } else if history == nil {
            state = .loading
            scheduleStaleFallback()
        }
    }

    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: any Swift.Error) {
        state = .stale(history == nil ? "DeepSeek usage page could not be loaded" : "Using cached usage")
    }

    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: any Swift.Error) {
        state = .stale(history == nil ? "DeepSeek usage page could not be loaded" : "Using cached usage")
    }

    private func scheduleStaleFallback() {
        staleTask?.cancel()
        staleTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(12))
            guard !Task.isCancelled, let self, self.state == .loading else { return }
            self.state = .stale(
                self.history == nil
                    ? "Sign in if prompted, or open DeepSeek usage in your browser"
                    : "Using cached usage; automatic sync needs attention"
            )
        }
    }

    private static let responseBridgeScript = #"""
    (() => {
      if (window.__aiMeterDeepSeekBridgeInstalled) return;
      window.__aiMeterDeepSeekBridgeInstalled = true;
      const allowedOrigin = 'https://platform.deepseek.com';
      const relevant = value => /usage|billing|dashboard|stat|cost|token/i.test(value || '');
      const send = (url, body) => {
        try {
          if (location.origin !== allowedOrigin || !relevant(url) || typeof body !== 'string' || body.length > 2000000) return;
          window.webkit.messageHandlers.deepSeekUsage.postMessage({origin: location.origin, url, body});
        } catch (_) {}
      };
      const nativeFetch = window.fetch;
      window.fetch = async (...args) => {
        const response = await nativeFetch(...args);
        try {
          const url = response.url || String(args[0] || '');
          const type = response.headers.get('content-type') || '';
          if (relevant(url) && type.includes('json')) response.clone().text().then(body => send(url, body));
        } catch (_) {}
        return response;
      };
      const nativeOpen = XMLHttpRequest.prototype.open;
      const nativeSend = XMLHttpRequest.prototype.send;
      XMLHttpRequest.prototype.open = function(method, url, ...rest) {
        this.__aiMeterURL = new URL(String(url), location.href).href;
        return nativeOpen.call(this, method, url, ...rest);
      };
      XMLHttpRequest.prototype.send = function(...args) {
        this.addEventListener('load', () => {
          try {
            const type = this.getResponseHeader('content-type') || '';
            if (relevant(this.__aiMeterURL) && type.includes('json') && typeof this.responseText === 'string') send(this.__aiMeterURL, this.responseText);
          } catch (_) {}
        }, {once: true});
        return nativeSend.apply(this, args);
      };
    })();
    """#
}
