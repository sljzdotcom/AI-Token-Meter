import AIMeterCore
import AppKit
import SwiftUI

struct WidgetProviderLogo: View {
    let provider: WidgetProvider

    var body: some View {
        Group {
            if let image = WidgetResource.logo(for: provider) {
                Image(nsImage: image)
                    .resizable()
                    .scaledToFit()
            } else {
                Image(systemName: provider.fallbackSymbol)
                    .resizable()
                    .scaledToFit()
            }
        }
        .foregroundStyle(.white)
        .accessibilityHidden(true)
    }
}

enum WidgetResource {
    static func logo(for provider: WidgetProvider) -> NSImage? {
        switch provider {
        case .claude:
            image(name: "claude", extension: "png", subdirectory: "Logos")
        case .codex:
            image(name: "codex", extension: "svg", subdirectory: "Logos")
        case .deepSeek:
            image(name: "deepseek", extension: "svg", subdirectory: "Logos")
        }
    }

    static func image(name: String, extension fileExtension: String, subdirectory: String) -> NSImage? {
        let url = Bundle.main.url(
            forResource: name,
            withExtension: fileExtension,
            subdirectory: subdirectory
        ) ?? Bundle.module.url(
            forResource: name,
            withExtension: fileExtension,
            subdirectory: subdirectory
        )
        guard let url, let image = NSImage(contentsOf: url) else {
            return nil
        }
        image.isTemplate = true
        return image
    }
}

extension WidgetProvider {
    var fallbackSymbol: String {
        switch self {
        case .claude: "sparkles"
        case .codex: "terminal"
        case .deepSeek: "wave.3.right"
        }
    }

    var accentColors: [Color] {
        switch self {
        case .claude:
            [Color(red: 0.91, green: 0.73, blue: 0.43), Color(red: 0.85, green: 0.47, blue: 0.34)]
        case .codex:
            [Color(red: 1.0, green: 0.44, blue: 0.68), Color(red: 0.66, green: 0.43, blue: 1.0)]
        case .deepSeek:
            [Color(red: 0.33, green: 0.93, blue: 0.78), Color(red: 0.47, green: 0.41, blue: 1.0)]
        }
    }
}

extension WidgetSnapshotSemantic {
    func accentColors(for provider: WidgetProvider) -> [Color] {
        // DeepSeek's ring represents depletion of its configured RMB baseline,
        // so keep its established palette while the status icon carries state.
        if provider == .deepSeek {
            return provider.accentColors
        }
        switch self {
        case .normal:
            return provider.accentColors
        case .warning:
            return [Color(red: 1.0, green: 0.78, blue: 0.18)]
        case .critical:
            return [Color(red: 1.0, green: 0.28, blue: 0.26)]
        case .stale:
            return [Color(red: 0.98, green: 0.55, blue: 0.22)]
        case .unavailable:
            return [Color.white.opacity(0.34)]
        }
    }
}
