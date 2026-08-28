import Foundation

public struct CodexCollector: UsageCollector {
    public let provider = UsageProvider.codex

    private let locator: any ExecutableLocating
    private let appServerClient: CodexAppServerClient

    public init(
        locator: any ExecutableLocating = ExecutableLocator()
    ) {
        self.locator = locator
        self.appServerClient = CodexAppServerClient()
    }

    public func collect() async throws -> UsageSnapshot {
        guard let executableURL = locator.locate(named: "codex") else {
            throw UsageCollectionError.notInstalled
        }

        return try await appServerClient.readRateLimits(executableURL: executableURL)
    }
}
