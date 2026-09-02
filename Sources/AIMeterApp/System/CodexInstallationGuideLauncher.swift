import AppKit
import Foundation

@MainActor
final class CodexInstallationGuideLauncher {
    static let guideURL = URL(string: "https://help.openai.com/en/articles/11096431")!

    private let openURL: (URL) -> Bool

    init(openURL: @escaping (URL) -> Bool = { NSWorkspace.shared.open($0) }) {
        self.openURL = openURL
    }

    func open() -> Bool {
        openURL(Self.guideURL)
    }
}
