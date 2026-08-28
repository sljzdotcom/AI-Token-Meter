import Foundation

public struct ClaudeCollector: UsageCollector {
    public let provider = UsageProvider.claude

    private let runner: any CommandRunning
    private let locator: any ExecutableLocating
    private let parser: ClaudeUsageParser

    public init(
        runner: any CommandRunning = PTYCommandRunner(),
        locator: any ExecutableLocating = ExecutableLocator(),
        parser: ClaudeUsageParser = ClaudeUsageParser()
    ) {
        self.runner = runner
        self.locator = locator
        self.parser = parser
    }

    public func collect() async throws -> UsageSnapshot {
        guard let executableURL = locator.locate(named: "claude") else {
            throw UsageCollectionError.notInstalled
        }

        try await verifyAuthentication(executableURL: executableURL)

        let result = try await runner.run(CommandRequest(
            executableURL: executableURL,
            inputLines: ["/usage", "/exit"],
            timeout: 10
        ))
        try detectAuthenticationFailure(in: result.output)

        do {
            return try parser.parse(result.output)
        } catch UsageCollectionError.unrecognizedOutput where result.exitCode != 0 {
            throw UsageCollectionError.transportFailure
        }
    }

    private func verifyAuthentication(executableURL: URL) async throws {
        let result = try await runner.run(CommandRequest(
            executableURL: executableURL,
            arguments: ["auth", "status"],
            inputLines: [],
            timeout: 5
        ))
        let sanitized = ANSITextSanitizer.sanitize(result.output)
        if let start = sanitized.firstIndex(of: "{"),
           let end = sanitized.lastIndex(of: "}"),
           let data = String(sanitized[start...end]).data(using: .utf8),
           let authStatus = try? JSONDecoder().decode(ClaudeAuthStatus.self, from: data) {
            if !authStatus.loggedIn {
                throw UsageCollectionError.authenticationRequired
            }
            return
        }

        if result.exitCode != 0 {
            throw UsageCollectionError.transportFailure
        }
    }

    private func detectAuthenticationFailure(in output: String) throws {
        let normalized = ANSITextSanitizer.sanitize(output).lowercased()
        if normalized.contains("not logged in")
            || normalized.contains("please log in")
            || normalized.contains("login required")
            || normalized.contains("需要登录") {
            throw UsageCollectionError.authenticationRequired
        }
    }
}

private struct ClaudeAuthStatus: Decodable {
    let loggedIn: Bool
}
