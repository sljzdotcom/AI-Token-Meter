import AIMeterCore
import Foundation
import SwiftUI

enum GeminiInstallationGuide {
    static let url = URL(string: "https://antigravity.google/docs/cli/install/")!
    static let installCommand = "curl -fsSL https://antigravity.google/cli/install.sh | bash"

    static func shouldOfferInstallation(
        for state: ServiceAccountConnectionState,
        pauseReason: GeminiPauseReason?
    ) -> Bool {
        state == .notInstalled || pauseReason == .notInstalled
    }

    static func instructions(
        for state: ServiceAccountConnectionState,
        pauseReason: GeminiPauseReason? = nil,
        localizer: AppLocalizer = AppLocalizer(language: .english)
    ) -> [String] {
        if pauseReason == .notInstalled { return [installCommand] }
        switch state {
        case .connected, .lastKnown, .checking, .unavailable:
            return []
        case .signInRequired:
            return [localizer.text("Google authentication is required. Sign in through AI Token Meter.")]
        case .notInstalled:
            return [installCommand]
        }
    }
}

struct GeminiInstallationHelp: View {
    @Environment(\.locale) private var locale
    let state: ServiceAccountConnectionState
    var pauseReason: GeminiPauseReason? = nil

    var body: some View {
        let instructions = GeminiInstallationGuide.instructions(
            for: state,
            pauseReason: pauseReason,
            localizer: AppLocalizer(locale: locale)
        )
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
