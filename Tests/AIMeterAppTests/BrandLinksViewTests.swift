import AIMeterCore
import AppKit
import Foundation
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("About brand links")
struct BrandLinksViewTests {
    @MainActor
    @Test("The rendered author buttons preserve labels and dispatch their fixed targets")
    func renderedButtonsOpenTheirTargets() async throws {
        var opened: URL?
        let model = BrandLinksModel(action: BrandLinkOpenAction { url in
            opened = url
            return true
        })
        let host = NSHostingView(rootView: BrandLinksView(model: model))
        let window = hostInWindow(host, height: 80)
        #expect(window.contentView === host)

        let buttons = try #require(await renderedViews(in: host) {
            let buttons = $0.compactMap { $0 as? NSButton }
            return buttons.count == 2 ? buttons : nil
        })
        #expect(buttons.map(\.title) == ["@MillerPanYue", "GitHub"])
        #expect(buttons.allSatisfy { $0.image?.size == NSSize(width: 15, height: 15) })

        try #require(buttons.first).performClick(nil)

        #expect(opened?.absoluteString == "https://twitter.com/MillerPanYue")
    }

    @MainActor
    @Test("A rejected click renders recoverable failure feedback")
    func rejectedClickRendersFeedback() async throws {
        var opened: URL?
        let model = BrandLinksModel(action: BrandLinkOpenAction {
            opened = $0
            return false
        })
        let host = NSHostingView(rootView: BrandLinksView(model: model))
        let window = hostInWindow(host, height: 100)
        #expect(window.contentView === host)
        let github = try #require(await renderedViews(in: host) { views in
            views.compactMap { $0 as? NSButton }.first { $0.title == "GitHub" }
        })

        github.performClick(nil)

        #expect(opened?.absoluteString == "https://github.com/sljzdotcom/AI-Token-Meter")
        #expect(model.openingFailed)
        let feedback = await renderedViews(in: host) { views in
            views.compactMap { $0 as? NSTextField }.first {
                $0.stringValue == "The author link could not be opened."
            }
        }
        #expect(feedback != nil)
    }
}

@MainActor
private func viewDescendants(of view: NSView) -> [NSView] {
    view.subviews.flatMap { child in
        [child] + viewDescendants(of: child)
    }
}

@MainActor
private func hostInWindow<Content: View>(_ host: NSHostingView<Content>, height: CGFloat) -> NSWindow {
    let frame = NSRect(x: 0, y: 0, width: 320, height: height)
    let window = NSWindow(contentRect: frame, styleMask: .borderless, backing: .buffered, defer: false)
    host.frame = frame
    window.contentView = host
    window.layoutIfNeeded()
    host.layoutSubtreeIfNeeded()
    return window
}

@MainActor
private func renderedViews<Result>(
    in host: NSView,
    timeout: Duration = .seconds(2),
    select: ([NSView]) -> Result?
) async -> Result? {
    let clock = ContinuousClock()
    let deadline = clock.now.advanced(by: timeout)
    while clock.now < deadline {
        host.window?.layoutIfNeeded()
        host.layoutSubtreeIfNeeded()
        if let result = select(viewDescendants(of: host)) {
            return result
        }
        try? await Task.sleep(for: .milliseconds(10))
    }
    host.window?.layoutIfNeeded()
    host.layoutSubtreeIfNeeded()
    return select(viewDescendants(of: host))
}
