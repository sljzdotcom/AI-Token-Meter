import Foundation

public struct WidgetSnapshotStore: Sendable {
    public let fileURL: URL

    public init(
        directoryURL: URL,
        fileName: String = AITokenMeterWidgetContract.snapshotFileName
    ) {
        fileURL = directoryURL.appendingPathComponent(fileName, isDirectory: false)
    }

    public init?(
        appGroupIdentifier: String,
        fileManager: FileManager = .default,
        fileName: String = AITokenMeterWidgetContract.snapshotFileName
    ) {
        guard let directoryURL = fileManager.containerURL(
            forSecurityApplicationGroupIdentifier: appGroupIdentifier
        ) else {
            return nil
        }
        self.init(directoryURL: directoryURL, fileName: fileName)
    }

    public func save(_ envelope: WidgetSnapshotEnvelope) throws {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        let data = try encoder.encode(envelope)
        try FileManager.default.createDirectory(
            at: fileURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try data.write(to: fileURL, options: .atomic)
    }

    public func load() throws -> WidgetSnapshotEnvelope? {
        guard FileManager.default.fileExists(atPath: fileURL.path) else { return nil }
        let data = try Data(contentsOf: fileURL)
        guard let envelope = try? JSONDecoder().decode(WidgetSnapshotEnvelope.self, from: data),
              envelope.version == WidgetSnapshotEnvelope.currentVersion else {
            return nil
        }
        return envelope
    }
}
