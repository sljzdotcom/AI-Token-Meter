import Foundation

public struct DeepSeekHistoryStore: Sendable {
    public let fileURL: URL

    public init(
        directoryURL: URL,
        fileName: String = "deepseek-usage-history.json"
    ) {
        self.fileURL = directoryURL.appendingPathComponent(fileName, isDirectory: false)
    }

    public func save(_ history: DeepSeekUsageHistory) throws {
        let sanitized = DeepSeekUsageHistory(
            days: history.days,
            updatedAt: history.updatedAt,
            statusMessage: history.statusMessage.map(SensitiveTextRedactor.redact)
        )
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        let data = try encoder.encode(sanitized)
        try FileManager.default.createDirectory(
            at: fileURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try data.write(to: fileURL, options: [.atomic])
    }

    public func load() throws -> DeepSeekUsageHistory? {
        guard FileManager.default.fileExists(atPath: fileURL.path) else { return nil }
        return try JSONDecoder().decode(
            DeepSeekUsageHistory.self,
            from: Data(contentsOf: fileURL)
        )
    }
}
