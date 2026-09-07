import AIMeterCore
import AppKit
import SwiftUI

struct BrandLinkOpenAction {
    private let openURL: (URL) -> Bool

    init(openURL: @escaping (URL) -> Bool = { NSWorkspace.shared.open($0) }) {
        self.openURL = openURL
    }

    func open(_ link: AppBrand.Link) -> Bool {
        openURL(link.url)
    }
}

@MainActor
final class BrandLinksModel: ObservableObject {
    @Published private(set) var openingFailed = false
    let action: BrandLinkOpenAction

    init(action: BrandLinkOpenAction) {
        self.action = action
    }

    func activate(_ link: AppBrand.Link) {
        openingFailed = !action.open(link)
    }
}

struct BrandLinksView: View {
    let action: BrandLinkOpenAction
    @ObservedObject private var model: BrandLinksModel

    init(action: BrandLinkOpenAction = BrandLinkOpenAction()) {
        self.action = action
        self.model = BrandLinksModel(action: action)
    }

    init(model: BrandLinksModel) {
        self.action = model.action
        self.model = model
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            ViewThatFits(in: .horizontal) {
                HStack(spacing: 8) { linkButtons }
                VStack(alignment: .leading, spacing: 6) { linkButtons }
            }
            if model.openingFailed {
                BrandLinkFailureLabel()
                    .fixedSize()
            }
        }
    }

    @ViewBuilder
    private var linkButtons: some View {
        ForEach(AppBrand.authorLinks, id: \.url) { link in
            BrandLinkButton(link: link) { model.activate(link) }
                .fixedSize()
        }
    }
}

private struct BrandLinkFailureLabel: NSViewRepresentable {
    func makeNSView(context: Context) -> NSTextField {
        let label = NSTextField(labelWithString: "The author link could not be opened.")
        label.font = .preferredFont(forTextStyle: .caption1)
        label.textColor = .secondaryLabelColor
        label.setAccessibilityLabel("The author link could not be opened.")
        return label
    }

    func updateNSView(_ label: NSTextField, context: Context) {}
}

private struct BrandLinkButton: NSViewRepresentable {
    let link: AppBrand.Link
    let activate: () -> Void

    func makeCoordinator() -> Coordinator { Coordinator(activate: activate) }

    func makeNSView(context: Context) -> NSButton {
        let button = NSButton(title: link.label, target: context.coordinator, action: #selector(Coordinator.activate))
        button.bezelStyle = .inline
        button.isBordered = false
        button.image = BrandIcon.image(for: link.label == "GitHub" ? .github : .x)
        button.imagePosition = .imageLeading
        button.imageHugsTitle = true
        button.toolTip = "Opens in your default browser"
        button.setAccessibilityLabel(link.label)
        button.setAccessibilityHelp("Opens in your default browser")
        return button
    }

    func updateNSView(_ button: NSButton, context: Context) {
        context.coordinator.activateHandler = activate
    }

    final class Coordinator: NSObject {
        var activateHandler: () -> Void
        init(activate: @escaping () -> Void) { self.activateHandler = activate }
        @objc func activate() { activateHandler() }
    }
}

private enum BrandIcon {
    enum Kind { case x, github }

    static func image(for kind: Kind) -> NSImage {
        NSImage(size: NSSize(width: 15, height: 15), flipped: true) { _ in
            NSColor.labelColor.setFill()
            let path = kind == .x ? xPath : githubPath
            path.fill()
            return true
        }
    }

    private static var xPath: NSBezierPath {
        let path = NSBezierPath()
        path.move(to: NSPoint(x: 1, y: 1)); path.line(to: NSPoint(x: 5, y: 1))
        path.line(to: NSPoint(x: 14, y: 14)); path.line(to: NSPoint(x: 10, y: 14)); path.close()
        path.move(to: NSPoint(x: 10.5, y: 1)); path.line(to: NSPoint(x: 14, y: 1))
        path.line(to: NSPoint(x: 4.5, y: 14)); path.line(to: NSPoint(x: 1, y: 14)); path.close()
        return path
    }

    private static var githubPath: NSBezierPath {
        let path = NSBezierPath(ovalIn: NSRect(x: 1.5, y: 2.5, width: 12, height: 11))
        path.move(to: NSPoint(x: 2.5, y: 6)); path.line(to: NSPoint(x: 3, y: 1))
        path.line(to: NSPoint(x: 6, y: 3)); path.close()
        path.move(to: NSPoint(x: 9, y: 3)); path.line(to: NSPoint(x: 12, y: 1))
        path.line(to: NSPoint(x: 12.5, y: 6)); path.close()
        path.appendRect(NSRect(x: 5.5, y: 11, width: 4, height: 4))
        return path
    }
}
