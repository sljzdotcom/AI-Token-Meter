import AIMeterCore
import Foundation
import SwiftUI

enum GeminiInstallationGuide {
    static let url = URL(string: "https://antigravity.google/docs/cli/install/")!
    static let installCommand = "curl -fsSL https://antigravity.google/cli/install.sh | bash"

    static func instructions(for state: ServiceAccountConnectionState) -> [String] {
        let signIn = "Run agy and complete Google sign-in."
        let finish = "Return to AI Token Meter and choose Retry."
        switch state {
        case .connected:
            return []
        case .signInRequired:
            return [signIn, finish]
        case .notInstalled, .checking, .unavailable:
            return [installCommand, signIn, finish]
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
