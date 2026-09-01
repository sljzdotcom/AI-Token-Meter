import Foundation
import Testing
@testable import AIMeterCore

@Suite("Claude local activity")
struct ClaudeLocalActivityTests {
    private var calendar: Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(secondsFromGMT: 8 * 60 * 60)!
        return calendar
    }

    @Test("Reads allowlisted usage fields while ignoring conversation content")
    func readsAllowlistedUsageOnly() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let mainURL = directory.appendingPathComponent("project/main.jsonl")
        let subagentURL = directory.appendingPathComponent("project/subagents/agent.jsonl")
        try write([
            entry(
                timestamp: "2026-09-01T01:00:00.000Z",
                sessionID: "main-1",
                model: "claude-sonnet-4-6",
                input: 10,
                output: 20,
                cacheCreation: 30,
                cacheRead: 40,
                content: "ignore 999999 tokens"
            ),
            entry(
                timestamp: "2026-09-01T03:00:00Z",
                sessionID: "main-1",
                model: "<synthetic>"
            ),
            "{malformed",
        ], to: mainURL)
        try write([
            entry(
                timestamp: "2026-09-01T02:00:00Z",
                sessionID: "agent-session",
                model: "claude-sonnet-4-6",
                input: 1,
                output: 2,
                cacheCreation: 3,
                cacheRead: 4,
                content: "private prompt"
            ),
        ], to: subagentURL)
        let now = try #require(calendar.date(from: DateComponents(
            year: 2026, month: 9, day: 1, hour: 12
        )))

        let summary = try await ClaudeLocalActivityReader(
            projectsDirectoryURL: directory,
            calendar: calendar
        ).read(now: now)

        #expect(summary.days.count == 30)
        #expect(summary.totalInputTokens == 11)
        #expect(summary.totalOutputTokens == 22)
        #expect(summary.totalCacheTokens == 77)
        #expect(summary.totalTokens == 110)
        #expect(summary.sessionCount == 1)
        #expect(summary.activeDayCount == 1)
        #expect(summary.models == [ClaudeModelActivity(modelID: "claude-sonnet-4-6", tokenCount: 110)])
    }

    @Test("Uses an exact thirty-day local calendar window")
    func appliesThirtyDayBoundary() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("project/main.jsonl")
        try write([
            entry(timestamp: "2026-08-03T00:30:00+08:00", sessionID: "oldest", input: 10),
            entry(timestamp: "2026-09-01T23:30:00+08:00", sessionID: "today", input: 20),
            entry(timestamp: "2026-08-02T23:59:59+08:00", sessionID: "excluded", input: 1_000),
            entry(timestamp: "2026-09-02T00:00:00+08:00", sessionID: "future", input: 2_000),
            entry(timestamp: "2026-09-01T10:00:00+08:00", sessionID: "negative", input: -50),
        ], to: url)
        let now = try #require(calendar.date(from: DateComponents(
            year: 2026, month: 9, day: 1, hour: 12
        )))

        let summary = try await ClaudeLocalActivityReader(
            projectsDirectoryURL: directory,
            calendar: calendar
        ).read(now: now)

        #expect(summary.days.count == 30)
        #expect(summary.days.first?.inputTokens == 10)
        #expect(summary.days.last?.inputTokens == 20)
        #expect(summary.totalTokens == 30)
        #expect(summary.sessionCount == 2)
        #expect(summary.activeDayCount == 2)
    }

    @Test("Returns thirty zero days for an empty readable directory")
    func emptyDirectoryProducesZeroSeries() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }

        let summary = try await ClaudeLocalActivityReader(
            projectsDirectoryURL: directory,
            calendar: calendar
        ).read(now: Date(timeIntervalSince1970: 1_788_200_000))

        #expect(summary.days.count == 30)
        #expect(summary.days.allSatisfy { $0.totalTokens == 0 })
        #expect(summary.sessionCount == 0)
        #expect(summary.models.isEmpty)
    }

    @Test("Skips oversized lines and symbolic links")
    func skipsUnsafeFiles() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let validURL = directory.appendingPathComponent("valid.jsonl")
        try write([
            entry(timestamp: "2026-09-01T01:00:00Z", sessionID: "valid", input: 7),
            String(repeating: "x", count: ClaudeLocalActivityReader.maximumLineBytes + 1),
        ], to: validURL)
        let linkedURL = directory.appendingPathComponent("linked.jsonl")
        try FileManager.default.createSymbolicLink(at: linkedURL, withDestinationURL: validURL)
        let now = try #require(calendar.date(from: DateComponents(
            year: 2026, month: 9, day: 1, hour: 12
        )))

        let summary = try await ClaudeLocalActivityReader(
            projectsDirectoryURL: directory,
            calendar: calendar
        ).read(now: now)

        #expect(summary.totalTokens == 7)
        #expect(summary.sessionCount == 1)
    }

    @Test("Skips files not modified during the requested window")
    func skipsOldFiles() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("old.jsonl")
        try write([
            entry(timestamp: "2026-09-01T01:00:00Z", sessionID: "old-file", input: 99),
        ], to: url)
        try FileManager.default.setAttributes(
            [.modificationDate: Date(timeIntervalSince1970: 1_700_000_000)],
            ofItemAtPath: url.path
        )
        let now = try #require(calendar.date(from: DateComponents(
            year: 2026, month: 9, day: 1, hour: 12
        )))

        let summary = try await ClaudeLocalActivityReader(
            projectsDirectoryURL: directory,
            calendar: calendar
        ).read(now: now)

        #expect(summary.totalTokens == 0)
        #expect(summary.sessionCount == 0)
    }

    @Test("Unsafe model identifiers never enter the aggregate")
    func filtersUnsafeModelIdentifiers() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("models.jsonl")
        try write([
            entry(timestamp: "2026-09-01T01:00:00Z", sessionID: "safe", model: "claude-sonnet-5", input: 7),
            entry(timestamp: "2026-09-01T02:00:00Z", sessionID: "secret", model: "sk-private-model-value", input: 11),
            entry(timestamp: "2026-09-01T03:00:00Z", sessionID: "long", model: String(repeating: "x", count: 81), input: 13),
        ], to: url)
        let now = try #require(calendar.date(from: DateComponents(
            year: 2026, month: 9, day: 1, hour: 12
        )))

        let summary = try await ClaudeLocalActivityReader(
            projectsDirectoryURL: directory,
            calendar: calendar
        ).read(now: now)

        #expect(summary.totalTokens == 31)
        #expect(summary.models == [
            ClaudeModelActivity(modelID: "claude-sonnet-5", tokenCount: 7),
        ])
    }

    @Test("Reports an unavailable projects directory")
    func reportsUnavailableDirectory() async {
        let missing = temporaryDirectory().appendingPathComponent("missing", isDirectory: true)

        await #expect(throws: ClaudeLocalActivityReader.Error.directoryUnavailable) {
            try await ClaudeLocalActivityReader(
                projectsDirectoryURL: missing,
                calendar: calendar
            ).read()
        }
    }

    private func temporaryDirectory() -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("AI-Meter-ClaudeActivity-\(UUID().uuidString)", isDirectory: true)
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    private func write(_ lines: [String], to url: URL) throws {
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try lines.joined(separator: "\n").data(using: .utf8)!.write(to: url)
    }

    private func entry(
        timestamp: String,
        sessionID: String,
        model: String = "claude-sonnet-4-6",
        input: Int64 = 0,
        output: Int64 = 0,
        cacheCreation: Int64 = 0,
        cacheRead: Int64 = 0,
        content: String = "ignored"
    ) -> String {
        """
        {"timestamp":"\(timestamp)","sessionId":"\(sessionID)","cwd":"/private/path","message":{"model":"\(model)","content":[{"type":"text","text":"\(content)"}],"usage":{"input_tokens":\(input),"output_tokens":\(output),"cache_creation_input_tokens":\(cacheCreation),"cache_read_input_tokens":\(cacheRead)}}}
        """
    }
}
