import AIMeterCore
import AppKit
import SwiftUI

enum FloatingStripBackgroundAsset {
    static let filename = "floating-strip-deep-sea"

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

    static func horizontalScale(for edge: FloatingStripEdge) -> CGFloat {
        edge == .left ? -1 : 1
    }
}

struct FloatingStripSurface: View {
    let edge: FloatingStripEdge
    private let backgroundImage: NSImage?

    init(
        edge: FloatingStripEdge,
        backgroundImage: NSImage? = FloatingStripBackgroundAsset.load()
    ) {
        self.edge = edge
        self.backgroundImage = backgroundImage
    }

    var body: some View {
        FloatingStripShape(edge: edge)
            .fill(AIMeterVisualTheme.floatingGlass)
            .overlay {
                if let backgroundImage {
                    Image(nsImage: backgroundImage)
                        .resizable()
                        .scaledToFill()
                        .scaleEffect(
                            x: FloatingStripBackgroundPresentation
                                .horizontalScale(for: edge),
                            y: 1
                        )
                        .overlay(
                            Color.black.opacity(
                                FloatingStripBackgroundPresentation.scrimOpacity
                            )
                        )
                        .clipShape(FloatingStripShape(edge: edge))
                        .accessibilityHidden(true)
                }
            }
    }
}
