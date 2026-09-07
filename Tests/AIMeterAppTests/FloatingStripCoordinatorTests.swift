import Testing
import AIMeterCore
@testable import AIMeterApp

@Suite("Display window ownership")
@MainActor
struct FloatingStripCoordinatorTests {
    @Test func topologyReusesWindowsAndClosesRemovedResources() {
        let windows = FloatingStripWindowRegistry<TestDisplayWindow>()
        windows.reconcile(targets: ["a", "b"]) { _ in TestDisplayWindow() }
        let a = windows.windows["a"]!
        let b = windows.windows["b"]!
        windows.reconcile(targets: ["b", "c", "c"]) { _ in TestDisplayWindow() }
        #expect(a.closed)
        #expect(windows.windows["b"] === b)
        #expect(windows.windows.count == 2)
        windows.reconcile(targets: []) { _ in TestDisplayWindow() }
        #expect(b.closed)
        #expect(windows.windows.isEmpty)
    }

    @Test func newDetailReleasesOldHostBeforePresenting() {
        let windows = FloatingStripWindowRegistry<TestDisplayWindow>()
        windows.reconcile(targets: ["a", "b"]) { _ in TestDisplayWindow() }
        let a = windows.windows["a"]!
        let b = windows.windows["b"]!
        windows.present(.deepSeek, on: "a")
        #expect(a.provider == .deepSeek)
        b.beforePresent = { #expect(a.provider == nil) }
        windows.present(.deepSeek, on: "b")
        #expect(a.provider == nil)
        #expect(b.provider == .deepSeek)
        windows.reconcile(targets: ["a"]) { _ in TestDisplayWindow() }
        #expect(b.provider == nil)
        #expect(b.closed)
        windows.present(.codex, on: "missing")
        #expect(a.provider == nil)
    }
}

// AppKit window allocation is the external boundary; test the registry's actual
// ordering and resource lifecycle contract without creating visible OS windows.
@MainActor private final class TestDisplayWindow: FloatingStripWindow {
    var provider: UsageProvider?
    var closed = false
    var beforePresent: (() -> Void)?
    func dismissDetail() { provider = nil }
    func showDetail(for provider: UsageProvider) { beforePresent?(); self.provider = provider }
    func close() { dismissDetail(); closed = true }
}
