import Foundation

public struct CodexAccountReader: ServiceAccountReading {
    public let provider = UsageProvider.codex

    private let locator: any ExecutableLocating
    private let client: CodexAppServerClient

    public init(locator: any ExecutableLocating = ExecutableLocator()) {
        self.locator = locator
        self.client = CodexAppServerClient()
    }

    init(locator: any ExecutableLocating, client: CodexAppServerClient) {
        self.locator = locator
        self.client = client
    }

    public func read() async -> ServiceAccountStatus {
        let executableURL: URL
        switch locator.discover(named: "codex") {
        case .found(let url): executableURL = url
        case .missing: return ServiceAccountStatus(provider: provider, connectionState: .notInstalled)
        case .unavailable: return ServiceAccountStatus(provider: provider, connectionState: .unavailable)
        }

        do {
            let result = try await client.readAccount(executableURL: executableURL)
            return status(from: result)
        } catch {
            return ServiceAccountStatus(provider: provider, connectionState: .unavailable)
        }
    }

    private func status(from result: CodexAccountResult) -> ServiceAccountStatus {
        guard let account = result.account else {
            return ServiceAccountStatus(provider: provider, connectionState: .signInRequired)
        }

        switch account {
        case .apiKey:
            return ServiceAccountStatus(
                provider: provider,
                connectionState: .connected,
                accountLabel: "API Key account"
            )
        case .chatGPT(let email, let planType):
            let trimmedEmail = email?.trimmingCharacters(in: .whitespacesAndNewlines)
            let plan = planType.map(displayPlan)
            return ServiceAccountStatus(
                provider: provider,
                connectionState: .connected,
                accountLabel: trimmedEmail?.isEmpty == false ? trimmedEmail : "ChatGPT account",
                accountDetail: ["ChatGPT", plan].compactMap { $0 }.joined(separator: " · ")
            )
        }
    }

    private func displayPlan(_ value: String) -> String {
        let words = value
            .replacingOccurrences(of: "_", with: " ")
            .replacingOccurrences(of: "-", with: " ")
            .split(separator: " ")
            .map { $0.prefix(1).uppercased() + $0.dropFirst() }
        return words.joined(separator: " ")
    }
}
