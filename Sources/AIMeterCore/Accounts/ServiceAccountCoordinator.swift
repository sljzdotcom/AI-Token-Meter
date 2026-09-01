import Foundation

public struct ServiceAccountCoordinator: Sendable {
    private let claudeReader: any ServiceAccountReading
    private let codexReader: any ServiceAccountReading
    private let deepSeekReader: any ServiceAccountReading

    public init(
        claudeReader: any ServiceAccountReading = ClaudeAccountReader(),
        codexReader: any ServiceAccountReading = CodexAccountReader(),
        deepSeekReader: any ServiceAccountReading = DeepSeekCredentialManager()
    ) {
        self.claudeReader = claudeReader
        self.codexReader = codexReader
        self.deepSeekReader = deepSeekReader
    }

    public func read(_ provider: UsageProvider) async -> ServiceAccountStatus {
        switch provider {
        case .claude: await claudeReader.read()
        case .codex: await codexReader.read()
        case .deepSeek: await deepSeekReader.read()
        }
    }

    public func readAll() async -> [ServiceAccountStatus] {
        async let claude = claudeReader.read()
        async let codex = codexReader.read()
        async let deepSeek = deepSeekReader.read()
        return await [claude, codex, deepSeek]
    }
}
