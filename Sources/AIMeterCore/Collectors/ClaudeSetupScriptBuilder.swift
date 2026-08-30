import Foundation

public struct ClaudeSetupScriptBuilder: Sendable {
    public init() {}

    public func build(
        workspaceURL: URL,
        executableURL: URL
    ) -> String {
        """
        #!/bin/zsh
        set -eu
        cd -- \(shellQuote(workspaceURL.path))
        exec \(shellQuote(executableURL.path)) --ax-screen-reader --safe-mode

        """
    }

    private func shellQuote(_ value: String) -> String {
        "'\(value.replacingOccurrences(of: "'", with: "'\\''"))'"
    }
}
