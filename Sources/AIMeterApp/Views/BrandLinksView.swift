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
    @StateObject private var model: BrandLinksModel

    init(action: BrandLinkOpenAction = BrandLinkOpenAction()) {
        self._model = StateObject(wrappedValue: BrandLinksModel(action: action))
    }

    init(model: BrandLinksModel) {
        self._model = StateObject(wrappedValue: model)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            ViewThatFits(in: .horizontal) {
                HStack(spacing: 8) { linkButtons }
                    .fixedSize(horizontal: true, vertical: false)
                VStack(alignment: .leading, spacing: 6) { linkButtons }
            }
            if model.openingFailed {
                BrandLinkFailureLabel()
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
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
        label.lineBreakMode = .byWordWrapping
        label.maximumNumberOfLines = 0
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
        let icon: BrandIcon.Kind = link.label == "GitHub" ? .github : link.label.hasPrefix("Telegram") ? .telegram : .x
        button.image = BrandIcon.image(for: icon)
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

    func sizeThatFits(
        _ proposal: ProposedViewSize,
        nsView button: NSButton,
        context: Context
    ) -> CGSize? {
        button.fittingSize
    }

    final class Coordinator: NSObject {
        var activateHandler: () -> Void
        init(activate: @escaping () -> Void) { self.activateHandler = activate }
        @objc func activate() { activateHandler() }
    }
}

private enum BrandIcon {
    enum Kind { case x, github, telegram }

    static func image(for kind: Kind) -> NSImage {
        NSImage(size: NSSize(width: 15, height: 15), flipped: true) { _ in
            NSColor.labelColor.setFill()
            let path = switch kind {
            case .x: xPath
            case .github: githubPath
            case .telegram: telegramPath
            }
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

    private static var telegramPath: NSBezierPath {
        let path = NSBezierPath()
        path.move(to: NSPoint(x: 1, y: 7.2))
        path.line(to: NSPoint(x: 14, y: 1.5))
        path.line(to: NSPoint(x: 11.8, y: 13.7))
        path.line(to: NSPoint(x: 7.8, y: 10.8))
        path.line(to: NSPoint(x: 5.8, y: 12.7))
        path.line(to: NSPoint(x: 6.1, y: 9.5))
        path.line(to: NSPoint(x: 11.8, y: 4.3))
        path.line(to: NSPoint(x: 4.7, y: 8.8))
        path.close()
        return path
    }
}
