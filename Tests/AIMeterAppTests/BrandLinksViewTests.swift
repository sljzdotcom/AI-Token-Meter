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
            return buttons.count == 3 ? buttons : nil
        })
        #expect(buttons.map(\.title) == ["@MillerPanYue", "GitHub", "Telegram @sljzdotcom"])
        #expect(buttons.allSatisfy { $0.image?.size == NSSize(width: 15, height: 15) })
        #expect(buttons.map { $0.accessibilityLabel() } == ["@MillerPanYue", "GitHub", "Telegram @sljzdotcom"])
        #expect(buttons.allSatisfy { $0.accessibilityHelp() == "Opens in your default browser" })

        try #require(buttons.last).performClick(nil)

        #expect(opened?.absoluteString == "https://t.me/sljzdotcom")
    }

    @MainActor
    @Test("A narrow host renders all three buttons without clipping")
    func narrowHostKeepsAllButtonsVisible() async throws {
        let model = BrandLinksModel(action: BrandLinkOpenAction { _ in true })
        let host = NSHostingView(
            rootView: BrandLinksView(model: model)
                .frame(width: 180, alignment: .leading)
        )
        let window = hostInWindow(host, width: 180, height: 100)
        #expect(window.contentView === host)

        let buttons = try #require(await renderedViews(in: host) { views in
            let buttons = views.compactMap { $0 as? NSButton }
            return buttons.count == 3 ? buttons : nil
        })
        let frames = buttons.map { $0.convert($0.bounds, to: host) }
        #expect(frames.allSatisfy { host.bounds.contains($0) })
        #expect(Set(frames.map { Int($0.minY.rounded()) }).count == 3)
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
private func hostInWindow<Content: View>(_ host: NSHostingView<Content>, width: CGFloat = 320, height: CGFloat) -> NSWindow {
    let frame = NSRect(x: 0, y: 0, width: width, height: height)
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
