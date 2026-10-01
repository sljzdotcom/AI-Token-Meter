import Foundation

public enum GeminiDiagnosticStage: String, CaseIterable, Codable, Sendable {
    case discovery
    case environment
    case version
    case usage
    case model
    case catalog
}

public enum GeminiDiagnosticCategory: String, CaseIterable, Codable, Sendable {
    case succeeded
    case authenticationRequired
    case timedOut
    case rateLimited
    case outputTruncated
    case invalidResponse
    case unrecognizedOutput
    case transportFailure
    case cancelled
    case unsupportedVersion
    case notInstalled
    case environmentRejected
    case unknown
}

public enum GeminiPauseReason: String, CaseIterable, Codable, Sendable {
    case authenticationRequired
    case timeout
    case networkOrProcessFailure
    case cancelled
    case rateLimited
    case outputTruncated
    case invalidResponse
    case unrecognizedOutput
    case unsupportedVersion
    case notInstalled
    case environmentRejected
    case unknown

    public var statusMessage: String {
        switch self {
        case .authenticationRequired: "Antigravity refresh paused after an authentication error"
        case .timeout: "Antigravity refresh paused after a timeout"
        case .networkOrProcessFailure: "Antigravity refresh paused after a service or process error"
        case .cancelled: "Antigravity refresh paused after an interrupted check"
        case .rateLimited: "Antigravity refresh paused after rate limiting"
        case .outputTruncated: "Antigravity refresh paused because CLI output exceeded the safe limit"
        case .invalidResponse: "Antigravity refresh paused after an invalid response"
        case .unrecognizedOutput: "Antigravity refresh paused because CLI output was not recognized"
        case .unsupportedVersion: "Antigravity refresh paused because the CLI version is unsupported"
        case .notInstalled: "Antigravity refresh paused because the CLI is unavailable"
        case .environmentRejected: "Antigravity refresh paused because the CLI environment is unsupported"
        case .unknown: "Antigravity refresh paused; the previous failure reason is unknown"
        }
    }

    public var diagnosticCategory: GeminiDiagnosticCategory {
        switch self {
        case .authenticationRequired: .authenticationRequired
        case .timeout: .timedOut
        case .networkOrProcessFailure: .transportFailure
        case .cancelled: .cancelled
        case .rateLimited: .rateLimited
        case .outputTruncated: .outputTruncated
        case .invalidResponse: .invalidResponse
        case .unrecognizedOutput: .unrecognizedOutput
        case .unsupportedVersion: .unsupportedVersion
        case .notInstalled: .notInstalled
        case .environmentRejected: .environmentRejected
        case .unknown: .unknown
        }
    }
}

/// Contains only fixed, non-sensitive diagnostic fields. Never add CLI output,
/// command arguments, paths, environment values, account identity, or tokens.
public struct GeminiDiagnosticRecord: Codable, Equatable, Sendable {
    public let recordedAt: Date
    public let stage: GeminiDiagnosticStage
    public let category: GeminiDiagnosticCategory
    public let durationMilliseconds: Int
    public let outputTruncated: Bool

    public init(
        recordedAt: Date,
        stage: GeminiDiagnosticStage,
        category: GeminiDiagnosticCategory,
        durationMilliseconds: Int,
        outputTruncated: Bool
    ) {
        self.recordedAt = recordedAt
        self.stage = stage
        self.category = category
        self.durationMilliseconds = min(max(durationMilliseconds, 0), 3_600_000)
        self.outputTruncated = outputTruncated
    }
}

/// Stores a small local ring buffer of allowlisted Antigravity diagnostic events.
public actor GeminiDiagnosticStore {
    public static let maximumRecordCount = 20
    public static let maximumFileBytes = 16 * 1_024

    private let fileURL: URL?
    private var storedRecords: [GeminiDiagnosticRecord]

    public init(fileURL: URL?) {
        self.fileURL = fileURL
        if let fileURL,
           let data = try? Data(contentsOf: fileURL),
           data.count <= Self.maximumFileBytes,
           let records = try? JSONDecoder().decode([GeminiDiagnosticRecord].self, from: data) {
            storedRecords = Array(records.suffix(Self.maximumRecordCount))
        } else {
            storedRecords = []
        }
    }

    public func record(_ record: GeminiDiagnosticRecord) {
        storedRecords.append(record)
        if storedRecords.count > Self.maximumRecordCount {
            storedRecords.removeFirst(storedRecords.count - Self.maximumRecordCount)
        }
        persistWithinLimit()
    }

    public func records() -> [GeminiDiagnosticRecord] { storedRecords }

    public func summary(pauseReason: GeminiPauseReason?, lastQuotaAt: Date?) -> String {
        var lines = ["Antigravity local diagnostic summary"]
        lines.append("refreshPaused=\(pauseReason == nil ? "false" : "true") reason=\(pauseReason?.rawValue ?? "none")")
        if let lastQuotaAt { lines.append("lastQuotaAt=\(Self.timestamp(lastQuotaAt))") }
        if storedRecords.isEmpty {
            lines.append("stage=unknown result=unknown (no phase details were recorded)")
        } else {
            lines.append(contentsOf: storedRecords.map { record in
                "time=\(Self.timestamp(record.recordedAt)) stage=\(record.stage.rawValue) result=\(record.category.rawValue) elapsed_ms=\(record.durationMilliseconds) output_truncated=\(record.outputTruncated)"
            })
        }
        return lines.joined(separator: "\n")
    }

    private func persistWithinLimit() {
        guard let fileURL else { return }
        while let data = try? JSONEncoder().encode(storedRecords), data.count > Self.maximumFileBytes,
              !storedRecords.isEmpty {
            storedRecords.removeFirst()
        }
        guard let data = try? JSONEncoder().encode(storedRecords), data.count <= Self.maximumFileBytes else { return }
        do {
            try FileManager.default.createDirectory(
                at: fileURL.deletingLastPathComponent(),
                withIntermediateDirectories: true,
                attributes: [.posixPermissions: 0o700]
            )
            try data.write(to: fileURL, options: .atomic)
            try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: fileURL.path)
        } catch {
            // Diagnostics must never affect quota collection or pause protection.
        }
    }

    private static func timestamp(_ date: Date) -> String {
        ISO8601DateFormatter().string(from: date)
    }
}
