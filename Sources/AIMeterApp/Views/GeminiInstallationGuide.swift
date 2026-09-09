import AIMeterCore
import Foundation
import SwiftUI

enum GeminiInstallationGuide {
    static let url = URL(string: "https://geminicli.com/docs/get-started/installation/")!
    static let installCommand = "npm install -g @google/gemini-cli@0.58.0"

    static func instructions(for state: ServiceAccountConnectionState) -> [String] {
        let signIn = "Run gemini and choose Sign in with Google."
        let finish = "Return to AI Token Meter and choose Retry."
        switch state {
        case .connected:
            return []
        case .signInRequired:
            return [signIn, finish]
        case .notInstalled, .checking, .unavailable:
            return ["Requires Node.js 20 or later.", installCommand, signIn, finish]
        }
    }
}

struct GeminiInstallationHelp: View {
    let state: ServiceAccountConnectionState

    var body: some View {
        let instructions = GeminiInstallationGuide.instructions(for: state)
        if !instructions.isEmpty {
            VStack(alignment: .leading, spacing: 5) {
                ForEach(Array(instructions.enumerated()), id: \.offset) { _, instruction in
                    if instruction == GeminiInstallationGuide.installCommand {
                        Text(instruction)
                            .monospaced()
                            .textSelection(.enabled)
                    } else {
                        Text(instruction)
                    }
                }
            }
            .aiMeterFont(.caption)
            .foregroundStyle(.secondary)
        }
    }
}
