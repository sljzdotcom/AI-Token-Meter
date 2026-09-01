import Foundation

protocol ClaudeLocalActivityReading: Sendable {
    func read(now: Date, dayCount: Int) async throws -> ClaudeLocalActivitySummary
}

struct ClaudeLocalActivityReader: ClaudeLocalActivityReading, Sendable {
    enum Error: Swift.Error, Equatable {
        case directoryUnavailable
    }

    static let maximumLineBytes = 2 * 1_024 * 1_024
    static let maximumFileBytes = 256 * 1_024 * 1_024

    private let projectsDirectoryURL: URL
    private let calendar: Calendar

    init(
        projectsDirectoryURL: URL = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".claude/projects", isDirectory: true),
        calendar: Calendar = .current
    ) {
        self.projectsDirectoryURL = projectsDirectoryURL
        self.calendar = calendar
    }

    func read(now: Date = Date(), dayCount: Int = 30) async throws -> ClaudeLocalActivitySummary {
        let directoryURL = projectsDirectoryURL
        let calendar = calendar
        return try await Task.detached(priority: .utility) {
            try Self.readSynchronously(
                directoryURL: directoryURL,
                calendar: calendar,
                now: now,
                dayCount: dayCount
            )
        }.value
    }

    private static func readSynchronously(
        directoryURL: URL,
        calendar: Calendar,
        now: Date,
        dayCount: Int
    ) throws -> ClaudeLocalActivitySummary {
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: directoryURL.path, isDirectory: &isDirectory),
              isDirectory.boolValue,
              FileManager.default.isReadableFile(atPath: directoryURL.path) else {
            throw Error.directoryUnavailable
        }

        let boundedDayCount = max(dayCount, 1)
        let today = calendar.startOfDay(for: now)
        let windowStart = calendar.date(
            byAdding: .day,
            value: -(boundedDayCount - 1),
            to: today
        ) ?? today
        let windowEnd = calendar.date(byAdding: .day, value: 1, to: today) ?? now
        let fileURLs = jsonlFiles(in: directoryURL)
        var buckets: [Date: TokenAccumulator] = [:]
        var sessions = Set<String>()
        var models: [String: Int64] = [:]
        let timestampParser = TimestampParser()

        for fileURL in fileURLs {
            let isSubagent = fileURL.pathComponents.contains("subagents")
            for entry in entries(in: fileURL) {
                guard let timestamp = timestampParser.date(from: entry.timestamp),
                      timestamp >= windowStart,
                      timestamp < windowEnd,
                      let usage = entry.message?.usage,
                      let components = usage.nonnegativeComponents else {
                    continue
                }
                let day = calendar.startOfDay(for: timestamp)
                var bucket = buckets[day] ?? TokenAccumulator()
                bucket.input = bucket.input.addingClamped(components.input)
                bucket.output = bucket.output.addingClamped(components.output)
                bucket.cache = bucket.cache.addingClamped(components.cache)
                buckets[day] = bucket

                if !isSubagent, let sessionID = entry.sessionID, !sessionID.isEmpty {
                    sessions.insert(sessionID)
                }
                if let model = entry.message?.model?.trimmingCharacters(in: .whitespacesAndNewlines),
                   !model.isEmpty {
                    models[model] = (models[model] ?? 0).addingClamped(components.total)
                }
            }
        }

        let days = (0..<boundedDayCount).compactMap { offset -> ClaudeDailyActivity? in
            guard let day = calendar.date(byAdding: .day, value: offset, to: windowStart) else {
                return nil
            }
            let bucket = buckets[day] ?? TokenAccumulator()
            return ClaudeDailyActivity(
                date: day,
                inputTokens: bucket.input,
                outputTokens: bucket.output,
                cacheTokens: bucket.cache
            )
        }
        let modelRows = models
            .map { ClaudeModelActivity(modelID: $0.key, tokenCount: $0.value) }
            .sorted {
                if $0.tokenCount == $1.tokenCount { return $0.modelID < $1.modelID }
                return $0.tokenCount > $1.tokenCount
            }

        return ClaudeLocalActivitySummary(
            days: days,
            sessionCount: sessions.count,
            activeDayCount: days.count(where: { $0.totalTokens > 0 }),
            models: modelRows,
            updatedAt: now,
            dayCount: boundedDayCount
        )
    }

    private static func jsonlFiles(in directoryURL: URL) -> [URL] {
        let keys: [URLResourceKey] = [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey]
        guard let enumerator = FileManager.default.enumerator(
            at: directoryURL,
            includingPropertiesForKeys: keys,
            options: [.skipsHiddenFiles]
        ) else {
            return []
        }

        var files: [URL] = []
        for case let url as URL in enumerator {
            guard let values = try? url.resourceValues(forKeys: Set(keys)) else { continue }
            if values.isSymbolicLink == true {
                enumerator.skipDescendants()
                continue
            }
            guard values.isRegularFile == true,
                  url.pathExtension.lowercased() == "jsonl",
                  (values.fileSize ?? 0) <= maximumFileBytes else {
                continue
            }
            files.append(url)
        }
        return files.sorted { $0.path < $1.path }
    }

    private static func entries(in fileURL: URL) -> [ClaudeLogEntry] {
        guard let data = try? Data(contentsOf: fileURL, options: .mappedIfSafe),
              data.count <= maximumFileBytes else {
            return []
        }
        let decoder = JSONDecoder()
        return data.split(separator: 0x0A, omittingEmptySubsequences: true).compactMap { line in
            guard line.count <= maximumLineBytes else { return nil }
            return try? decoder.decode(ClaudeLogEntry.self, from: Data(line))
        }
    }
}

private struct ClaudeLogEntry: Decodable {
    let timestamp: String
    let sessionID: String?
    let message: Message?

    enum CodingKeys: String, CodingKey {
        case timestamp
        case sessionID = "sessionId"
        case legacySessionID = "session_id"
        case message
    }

    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        timestamp = try values.decode(String.self, forKey: .timestamp)
        sessionID = try values.decodeIfPresent(String.self, forKey: .sessionID)
            ?? values.decodeIfPresent(String.self, forKey: .legacySessionID)
        message = try values.decodeIfPresent(Message.self, forKey: .message)
    }

    struct Message: Decodable {
        let model: String?
        let usage: Usage?
    }

    struct Usage: Decodable {
        let inputTokens: Int64?
        let outputTokens: Int64?
        let cacheCreationInputTokens: Int64?
        let cacheReadInputTokens: Int64?

        enum CodingKeys: String, CodingKey {
            case inputTokens = "input_tokens"
            case outputTokens = "output_tokens"
            case cacheCreationInputTokens = "cache_creation_input_tokens"
            case cacheReadInputTokens = "cache_read_input_tokens"
        }

        var nonnegativeComponents: TokenComponents? {
            let values = [
                inputTokens ?? 0,
                outputTokens ?? 0,
                cacheCreationInputTokens ?? 0,
                cacheReadInputTokens ?? 0,
            ]
            guard values.allSatisfy({ $0 >= 0 }) else { return nil }
            return TokenComponents(
                input: values[0],
                output: values[1],
                cache: values[2].addingClamped(values[3])
            )
        }
    }
}

private struct TokenAccumulator {
    var input: Int64 = 0
    var output: Int64 = 0
    var cache: Int64 = 0
}

private struct TokenComponents {
    let input: Int64
    let output: Int64
    let cache: Int64

    var total: Int64 {
        input.addingClamped(output).addingClamped(cache)
    }
}

private struct TimestampParser {
    private let fractional: ISO8601DateFormatter
    private let standard: ISO8601DateFormatter

    init() {
        fractional = ISO8601DateFormatter()
        fractional.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        standard = ISO8601DateFormatter()
        standard.formatOptions = [.withInternetDateTime]
    }

    func date(from value: String) -> Date? {
        if let date = fractional.date(from: value) {
            return date
        }
        return standard.date(from: value)
    }
}

private extension Int64 {
    func addingClamped(_ value: Int64) -> Int64 {
        let (sum, overflow) = addingReportingOverflow(value)
        return overflow ? .max : Swift.max(sum, 0)
    }
}
