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
