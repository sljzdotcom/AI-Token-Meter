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

struct BrandLinksView: View {
    let action: BrandLinkOpenAction
    @State private var openingFailed = false

    init(action: BrandLinkOpenAction = BrandLinkOpenAction()) {
        self.action = action
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            ViewThatFits(in: .horizontal) {
                HStack(spacing: 8) { linkButtons }
                VStack(alignment: .leading, spacing: 6) { linkButtons }
            }
            if openingFailed {
                Text("The author link could not be opened.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .accessibilityLabel("The author link could not be opened.")
            }
        }
    }

    @ViewBuilder
    private var linkButtons: some View {
        ForEach(AppBrand.authorLinks, id: \.url) { link in
            Button {
                openingFailed = !action.open(link)
            } label: {
                HStack(spacing: 5) {
                    Text(link.label == "GitHub" ? "GH" : "𝕏")
                        .font(.caption2.monospaced().bold())
                        .accessibilityHidden(true)
                    Text(link.label)
                }
            }
            .buttonStyle(.link)
            .accessibilityHint("Opens in your default browser")
        }
    }
}
