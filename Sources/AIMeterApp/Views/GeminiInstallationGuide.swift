import AIMeterCore
import Foundation
import SwiftUI

enum GeminiInstallationGuide {
    static let url = URL(string: "https://antigravity.google/docs/cli/install/")!
    static let installCommand = "curl -fsSL https://antigravity.google/cli/install.sh | bash"

    static func instructions(for state: ServiceAccountConnectionState, localizer: AppLocalizer = AppLocalizer(language: .english)) -> [String] {
        let signIn = localizer.text("Run agy and complete Google sign-in.")
        let finish = localizer.text("Return to AI Token Meter and choose Retry.")
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
    @Environment(\.locale) private var locale
    let state: ServiceAccountConnectionState

    var body: some View {
        let instructions = GeminiInstallationGuide.instructions(for: state, localizer: AppLocalizer(locale: locale))
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
