import Foundation

public struct SnapshotCache: Sendable {
    public let fileURL: URL

    public init(
        directoryURL: URL,
        fileName: String = "usage-snapshots.json"
    ) {
        self.fileURL = directoryURL.appendingPathComponent(fileName, isDirectory: false)
    }

    public func save(_ snapshots: [UsageSnapshot]) throws {
        let envelope = CacheEnvelope(version: 1, snapshots: snapshots)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        let data = try encoder.encode(envelope)

        try FileManager.default.createDirectory(
            at: fileURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try data.write(to: fileURL, options: [.atomic])
    }

    public func load() throws -> [UsageSnapshot] {
        guard FileManager.default.fileExists(atPath: fileURL.path) else { return [] }
        let data = try Data(contentsOf: fileURL)
        guard let envelope = try? JSONDecoder().decode(CacheEnvelope.self, from: data),
              envelope.version == 1 else {
            return []
        }
        return envelope.snapshots
    }
}

private struct CacheEnvelope: Codable {
    let version: Int
    let snapshots: [UsageSnapshot]
}
