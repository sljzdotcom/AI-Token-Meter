import AIMeterCore
import AppKit
import SwiftUI

struct ProviderLogo: View {
    let provider: UsageProvider
    var size: CGFloat = 28

    var body: some View {
        Group {
            if let image = Self.image(for: provider) {
                Image(nsImage: image)
                    .resizable()
                    .renderingMode(.template)
                    .scaledToFit()
            } else {
                Image(systemName: provider.symbolName)
                    .resizable()
                    .scaledToFit()
            }
        }
        .frame(width: size, height: size)
        .scaleEffect(ProviderLogoStyle.opticalScale(for: provider))
        .foregroundStyle(.white)
        .accessibilityHidden(true)
    }

    private static func image(for provider: UsageProvider) -> NSImage? {
        guard let url = AppResourceLocator.url(
            forResource: provider.logoResourceName,
            withExtension: provider.logoResourceExtension,
            subdirectory: "Logos"
        ) else {
            return nil
        }
        return NSImage(contentsOf: url)
    }
}

private extension UsageProvider {
    var logoResourceName: String {
        switch self {
        case .claude: "claude"
        case .codex: "codex"
        case .deepSeek: "deepseek"
        }
    }

    var logoResourceExtension: String {
        self == .claude ? "png" : "svg"
    }
}
