import AIMeterCore
import AppKit
import SwiftUI

enum FloatingStripBackgroundAsset {
    static let filename = "floating-strip-deep-sea"
    static let defaultImage = load()

    static func resourceURL(in bundle: Bundle = .module) -> URL? {
        bundle.url(
            forResource: filename,
            withExtension: "png",
            subdirectory: "Backgrounds"
        )
    }

    static func load(in bundle: Bundle = .module) -> NSImage? {
        resourceURL(in: bundle).flatMap(NSImage.init(contentsOf:))
    }
}

enum FloatingStripBackgroundPresentation {
    static let scrimOpacity = 0.38
    static let contentScale: CGFloat = 1.22

    static func scale(for edge: FloatingStripEdge) -> CGSize {
        CGSize(
            width: edge == .left ? -contentScale : contentScale,
            height: contentScale
        )
    }
}

struct FloatingStripSurface: View {
    let edge: FloatingStripEdge
    private let backgroundImage: NSImage?

    init(
        edge: FloatingStripEdge,
        backgroundImage: NSImage? = FloatingStripBackgroundAsset.defaultImage
    ) {
        self.edge = edge
        self.backgroundImage = backgroundImage
    }

    var body: some View {
        ZStack {
            FloatingStripShape(edge: edge)
                .fill(AIMeterVisualTheme.floatingGlass)

            if let backgroundImage {
                let scale = FloatingStripBackgroundPresentation.scale(for: edge)
                Image(nsImage: backgroundImage)
                    .resizable()
                    .scaledToFill()
                    .scaleEffect(x: scale.width, y: scale.height, anchor: .center)
                    .overlay {
                        Color.black.opacity(FloatingStripBackgroundPresentation.scrimOpacity)
                    }
                    .accessibilityHidden(true)
            }
        }
        .clipShape(FloatingStripShape(edge: edge))
    }
}
