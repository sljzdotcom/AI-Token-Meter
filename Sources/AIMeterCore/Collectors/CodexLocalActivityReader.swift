import Foundation

struct CodexLocalThreadActivity: Equatable, Sendable {
    let tokensUsed: Int64
    let createdAt: Date
    let updatedAt: Date
}

struct CodexLocalActivityRowParser: Sendable {
    func parse(_ text: String) -> [CodexLocalThreadActivity] {
        text.split(whereSeparator: \Character.isNewline).compactMap { line in
            let fields = line.split(separator: "\t", omittingEmptySubsequences: false)
            guard fields.count == 3,
                  let tokens = Int64(fields[0]),
                  let created = TimeInterval(fields[1]),
                  let updated = TimeInterval(fields[2]),
                  tokens >= 0,
                  created.isFinite,
                  updated.isFinite else {
                return nil
            }
            return CodexLocalThreadActivity(
                tokensUsed: tokens,
                createdAt: Date(timeIntervalSince1970: created),
                updatedAt: Date(timeIntervalSince1970: updated)
            )
        }
    }
}

struct CodexLocalActivitySummarizer: Sendable {
    let calendar: Calendar

    init(calendar: Calendar = .current) {
        self.calendar = calendar
    }

    func summarize(
        _ rows: [CodexLocalThreadActivity],
        now: Date = Date(),
        dayCount: Int = 30
    ) -> CodexLocalActivitySummary {
        let boundedDayCount = max(dayCount, 1)
        let today = calendar.startOfDay(for: now)
        let windowStart = calendar.date(
            byAdding: .day,
            value: -(boundedDayCount - 1),
            to: today
        ) ?? today
        let windowEnd = calendar.date(byAdding: .day, value: 1, to: today) ?? now
        let recent = rows.filter { $0.updatedAt >= windowStart && $0.updatedAt < windowEnd }
        let tokenCount = recent.reduce(Int64.zero) { partial, row in
            let (sum, overflow) = partial.addingReportingOverflow(row.tokensUsed)
            return overflow ? Int64.max : sum
        }
        let longest = recent.reduce(TimeInterval.zero) { longest, row in
            max(longest, max(row.updatedAt.timeIntervalSince(row.createdAt), 0))
        }

        var activeDays = Set<Date>()
        for row in recent {
            for value in [row.createdAt, row.updatedAt] where value >= windowStart && value < windowEnd {
                activeDays.insert(calendar.startOfDay(for: value))
            }
        }
        let yesterday = calendar.date(byAdding: .day, value: -1, to: today) ?? today
        let streakEnd: Date? = if activeDays.contains(today) {
            today
        } else if activeDays.contains(yesterday) {
            yesterday
        } else {
            nil
        }
        var streak = 0
        if var day = streakEnd {
            while activeDays.contains(day) {
                streak += 1
                guard let previous = calendar.date(byAdding: .day, value: -1, to: day) else { break }
                day = previous
            }
        }

        return CodexLocalActivitySummary(
            tokenCount: tokenCount,
            currentStreakDays: streak,
            longestSessionDuration: longest,
            dayCount: boundedDayCount
        )
    }
}

struct CodexLocalActivityReader: Sendable {
    enum Error: Swift.Error {
        case databaseUnavailable
        case queryFailed
    }

    private let databaseURL: URL
    private let sqliteURL: URL
    private let calendar: Calendar

    init(
        databaseURL: URL = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".codex/state_5.sqlite"),
        sqliteURL: URL = URL(fileURLWithPath: "/usr/bin/sqlite3"),
        calendar: Calendar = .current
    ) {
        self.databaseURL = databaseURL
        self.sqliteURL = sqliteURL
        self.calendar = calendar
    }

    func read(now: Date = Date(), dayCount: Int = 30) async throws -> CodexLocalActivitySummary {
        guard FileManager.default.isReadableFile(atPath: databaseURL.path),
              FileManager.default.isExecutableFile(atPath: sqliteURL.path) else {
            throw Error.databaseUnavailable
        }
        let today = calendar.startOfDay(for: now)
        let windowStart = calendar.date(
            byAdding: .day,
            value: -(max(dayCount, 1) - 1),
            to: today
        ) ?? today
        let output = try await query(since: Int64(windowStart.timeIntervalSince1970))
        let rows = CodexLocalActivityRowParser().parse(output)
        return CodexLocalActivitySummarizer(calendar: calendar).summarize(
            rows,
            now: now,
            dayCount: dayCount
        )
    }

    private func query(since timestamp: Int64) async throws -> String {
        try await Task.detached(priority: .utility) {
            let process = Process()
            let output = Pipe()
            process.executableURL = sqliteURL
            process.arguments = [
                "-readonly",
                "-separator", "\t",
                databaseURL.path,
                "PRAGMA query_only=ON; PRAGMA busy_timeout=1000; " +
                    "SELECT tokens_used, created_at, updated_at FROM threads " +
                    "WHERE updated_at >= \(timestamp) ORDER BY updated_at;",
            ]
            process.standardOutput = output
            process.standardError = FileHandle.nullDevice
            try process.run()
            let data = output.fileHandleForReading.readDataToEndOfFile()
            process.waitUntilExit()
            guard process.terminationStatus == 0 else { throw Error.queryFailed }
            guard data.count <= 1_048_576,
                  let text = String(data: data, encoding: .utf8) else {
                throw Error.queryFailed
            }
            return text
        }.value
    }
}
