import Foundation

public enum CLIAuthenticationScriptError: Error, Equatable {
    case unsupportedProvider
}

public struct CLIAuthenticationScriptBuilder: Sendable {
    public init() {}

    public func build(
        provider: UsageProvider,
        executableURL: URL
    ) throws -> String {
        let arguments: String
        switch provider {
        case .claude:
            arguments = "auth login"
        case .codex:
            arguments = "login"
        case .deepSeek:
            throw CLIAuthenticationScriptError.unsupportedProvider
        }

        return """
        #!/bin/zsh
        set -eu
        export PATH=\(shellQuote(executableURL.deletingLastPathComponent().path)):"${PATH:-/usr/bin:/bin:/usr/sbin:/sbin}"
        exec \(shellQuote(executableURL.path)) \(arguments)

        """
    }

    private func shellQuote(_ value: String) -> String {
        "'\(value.replacingOccurrences(of: "'", with: "'\\''"))'"
    }
}
