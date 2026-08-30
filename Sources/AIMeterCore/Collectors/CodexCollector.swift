import Foundation

public struct CodexCollector: UsageCollector {
    public let provider = UsageProvider.codex

    private let locator: any ExecutableLocating
    private let appServerClient: CodexAppServerClient
    private let localActivityReader: CodexLocalActivityReader

    public init(
        locator: any ExecutableLocating = ExecutableLocator()
    ) {
        self.locator = locator
        self.appServerClient = CodexAppServerClient()
        self.localActivityReader = CodexLocalActivityReader()
    }

    public func collect() async throws -> UsageSnapshot {
        guard let executableURL = locator.locate(named: "codex") else {
            throw UsageCollectionError.notInstalled
        }

        async let official = appServerClient.readRateLimits(executableURL: executableURL)
        async let localActivity = optionalLocalActivity()
        return try await official.withCodexLocalActivity(localActivity)
    }

    private func optionalLocalActivity() async -> CodexLocalActivitySummary? {
        try? await localActivityReader.read()
    }
}
