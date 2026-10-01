import Foundation

public enum CLIAuthenticationScriptError: Error, Equatable {
    case unsupportedProvider
    case invalidCompletionToken
}

public struct CLIAuthenticationScriptBuilder: Sendable {
    public init() {}

    public func build(
        provider: UsageProvider,
        executableURL: URL,
        completionToken: String? = nil
    ) throws -> String {
        let command: String
        switch provider {
        case .claude:
            command = "exec \(shellQuote(executableURL.path)) auth login"
        case .codex:
            command = "exec \(shellQuote(executableURL.path)) login"
        case .gemini:
            guard let completionToken, UUID(uuidString: completionToken) != nil else {
                throw CLIAuthenticationScriptError.invalidCompletionToken
            }
            var components = URLComponents()
            components.scheme = "aitokenmeter"
            components.host = "antigravity-login-complete"
            components.queryItems = [URLQueryItem(name: "token", value: completionToken)]
            guard let callbackURL = components.url?.absoluteString else {
                throw CLIAuthenticationScriptError.invalidCompletionToken
            }
            command = """
            \(shellQuote(executableURL.path))
            status=$?
            /usr/bin/open -g \(shellQuote(callbackURL))
            exit "$status"
            """
        case .deepSeek:
            throw CLIAuthenticationScriptError.unsupportedProvider
        }

        return """
        #!/bin/zsh
        set -u
        export PATH=\(shellQuote(executableURL.deletingLastPathComponent().path)):"${PATH:-/usr/bin:/bin:/usr/sbin:/sbin}"
        \(command)

        """
    }

    private func shellQuote(_ value: String) -> String {
        "'\(value.replacingOccurrences(of: "'", with: "'\\''"))'"
    }
}
