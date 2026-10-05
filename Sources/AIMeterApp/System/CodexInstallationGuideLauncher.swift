import AppKit
import Foundation

@MainActor
final class CodexInstallationGuideLauncher {
    static let guideURL = URL(string: "https://help.openai.com/en/articles/11096431")!

    private let openURL: (URL) -> Bool
    private let systemActionPolicy: SystemActionPolicy
    private let usesSystemOpener: Bool

    init(openURL: ((URL) -> Bool)? = nil, systemActionPolicy: SystemActionPolicy = .current) {
        self.systemActionPolicy = systemActionPolicy
        self.usesSystemOpener = openURL == nil
        self.openURL = openURL ?? { NSWorkspace.shared.open($0) }
    }

    func open() -> Bool {
        guard !usesSystemOpener else {
            return systemActionPolicy.open(Self.guideURL, using: openURL)
        }
        return openURL(Self.guideURL)
    }
}
