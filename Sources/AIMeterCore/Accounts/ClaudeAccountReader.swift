import Foundation

public struct ClaudeAccountStatusParser: Sendable {
    public init() {}

    public func parse(
        _ output: String,
        checkedAt: Date = Date()
    ) throws -> ServiceAccountStatus {
        let sanitized = ANSITextSanitizer.sanitize(output)
        guard let start = sanitized.firstIndex(of: "{"),
              let end = sanitized.lastIndex(of: "}"),
              start <= end,
              let data = String(sanitized[start...end]).data(using: .utf8) else {
            throw ClaudeAccountStatusParsingError.invalidJSON
        }

        let response = try JSONDecoder().decode(Response.self, from: data)
        guard response.loggedIn else {
            return ServiceAccountStatus(
                provider: .claude,
                connectionState: .signInRequired,
                checkedAt: checkedAt
            )
        }

        let method = displayAuthenticationMethod(response.authMethod)
        let subscription = displaySubscription(response.subscriptionType)
        let email = response.email?.trimmingCharacters(in: .whitespacesAndNewlines)
        let accountLabel = email?.isEmpty == false
            ? email
            : method.map { "\($0) account" }
        let accountDetail: String?
        if email?.isEmpty == false {
            accountDetail = [method, subscription].compactMap { $0 }.joined(separator: " · ").nilIfEmpty
        } else {
            accountDetail = subscription
        }

        return ServiceAccountStatus(
            provider: .claude,
            connectionState: .connected,
            accountLabel: accountLabel ?? "Connected account",
            accountDetail: accountDetail,
            checkedAt: checkedAt
        )
    }

    private func displayAuthenticationMethod(_ value: String?) -> String? {
        guard let value = value?.trimmingCharacters(in: .whitespacesAndNewlines), !value.isEmpty else {
            return nil
        }
        switch value.lowercased() {
        case "oauth": return "OAuth"
        case "claude.ai", "claudeai": return UsageProvider.claude.displayName
        case "api_key", "apikey", "api-key": return "API Key"
        case "none": return nil
        default: return value
        }
    }

    private func displaySubscription(_ value: String?) -> String? {
        guard let value = value?.trimmingCharacters(in: .whitespacesAndNewlines), !value.isEmpty else {
            return nil
        }
        return value.prefix(1).uppercased() + value.dropFirst()
    }

    private struct Response: Decodable {
        let loggedIn: Bool
        let email: String?
        let authMethod: String?
        let subscriptionType: String?
    }
}

public enum ClaudeAccountStatusParsingError: Error {
    case invalidJSON
}

public struct ClaudeAccountReader: ServiceAccountReading {
    public let provider = UsageProvider.claude

    private let runner: any CommandRunning
    private let locator: any ExecutableLocating
    private let parser: ClaudeAccountStatusParser

    public init(
        runner: any CommandRunning = PTYCommandRunner(),
        locator: any ExecutableLocating = ExecutableLocator(),
        parser: ClaudeAccountStatusParser = ClaudeAccountStatusParser()
    ) {
        self.runner = runner
        self.locator = locator
        self.parser = parser
    }

    public func read() async -> ServiceAccountStatus {
        guard let executableURL = locator.locate(named: "claude") else {
            return ServiceAccountStatus(provider: provider, connectionState: .notInstalled)
        }

        do {
            let result = try await runner.run(CommandRequest(
                executableURL: executableURL,
                arguments: ["auth", "status", "--json"],
                inputLines: [],
                timeout: 5
            ))
            guard result.exitCode == 0 else {
                return ServiceAccountStatus(provider: provider, connectionState: .unavailable)
            }
            return try parser.parse(result.output)
        } catch {
            return ServiceAccountStatus(provider: provider, connectionState: .unavailable)
        }
    }
}

private extension String {
    var nilIfEmpty: String? { isEmpty ? nil : self }
}
