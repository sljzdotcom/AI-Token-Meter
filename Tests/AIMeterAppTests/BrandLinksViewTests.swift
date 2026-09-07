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
    func renderedButtonsOpenTheirTargets() throws {
        var opened: URL?
        let model = BrandLinksModel(action: BrandLinkOpenAction { url in
            opened = url
            return true
        })
        let host = NSHostingView(rootView: BrandLinksView(model: model))
        let window = hostInWindow(host, height: 80)
        #expect(window.contentView === host)

        let buttons = viewDescendants(of: host).compactMap { $0 as? NSButton }
        #expect(buttons.map(\.title) == ["@MillerPanYue", "GitHub"])
        #expect(buttons.allSatisfy { $0.image?.size == NSSize(width: 15, height: 15) })

        try #require(buttons.first).performClick(nil)

        #expect(opened?.absoluteString == "https://twitter.com/MillerPanYue")
    }

    @MainActor
    @Test("A rejected click renders recoverable failure feedback")
    func rejectedClickRendersFeedback() throws {
        let model = BrandLinksModel(action: BrandLinkOpenAction { _ in false })
        let host = NSHostingView(rootView: BrandLinksView(model: model))
        let window = hostInWindow(host, height: 100)
        #expect(window.contentView === host)
        let github = try #require(viewDescendants(of: host).compactMap { $0 as? NSButton }.first {
            $0.title == "GitHub"
        })

        github.performClick(nil)
        host.layoutSubtreeIfNeeded()

        #expect(model.openingFailed)
        #expect(viewDescendants(of: host).contains {
            ($0 as? NSTextField)?.stringValue == "The author link could not be opened."
        })
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
