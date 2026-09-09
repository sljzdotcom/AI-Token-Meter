import Foundation

public struct CLIInstallationScriptBuilder: Sendable {
    public init() {}

    public func build(provider: UsageProvider) throws -> String {
        let url: String
        let interpreter: String
        switch provider {
        case .claude: url = "https://claude.ai/install.sh"; interpreter = "/bin/bash"
        case .codex: url = "https://chatgpt.com/codex/install.sh"; interpreter = "/bin/sh"
        case .deepSeek, .gemini: throw CLIAuthenticationScriptError.unsupportedProvider
        }
        return """
        #!/bin/sh
        set -eu
        umask 077
        installer_dir=$(mktemp -d "${TMPDIR:-/tmp}/ai-meter-install.XXXXXXXX")
        trap 'rm -rf "$installer_dir"' EXIT
        trap 'exit 130' INT TERM
        printf '%s\\n' 'Downloading the official \(provider.displayName) installer…'
        curl --fail --show-error --silent --location --proto '=https' --tlsv1.2 --output "$installer_dir/install.sh" '\(url)'
        \(interpreter) "$installer_dir/install.sh"

        """
    }
}
