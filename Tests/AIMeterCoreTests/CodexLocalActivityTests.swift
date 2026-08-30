import Foundation
import Testing
@testable import AIMeterCore

@Suite("Codex local activity")
struct CodexLocalActivityTests {
    private var calendar: Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(secondsFromGMT: 8 * 60 * 60)!
        return calendar
    }

    @Test("Summarizes a rolling local window without reading conversation text")
    func summarizesRecentRows() throws {
        let now = try #require(calendar.date(from: DateComponents(
            year: 2026, month: 8, day: 31, hour: 10
        )))
        let day = { (offset: Int, hour: Int) throws -> Date in
            try #require(self.calendar.date(
                byAdding: DateComponents(day: offset, hour: hour),
                to: self.calendar.startOfDay(for: now)
            ))
        }
        let rows = [
            CodexLocalThreadActivity(tokensUsed: 1_000, createdAt: try day(-3, 8), updatedAt: try day(-2, 9)),
            CodexLocalThreadActivity(tokensUsed: 2_000, createdAt: try day(-1, 8), updatedAt: try day(-1, 10)),
            CodexLocalThreadActivity(tokensUsed: 500, createdAt: try day(0, 8), updatedAt: try day(0, 9)),
            CodexLocalThreadActivity(tokensUsed: 99_000, createdAt: try day(-40, 8), updatedAt: try day(-35, 9)),
        ]

        let summary = CodexLocalActivitySummarizer(calendar: calendar).summarize(
            rows,
            now: now,
            dayCount: 30
        )

        #expect(summary.tokenCount == 3_500)
        #expect(summary.currentStreakDays == 4)
        #expect(summary.longestSessionDuration == 90_000)
        #expect(summary.dayCount == 30)
    }

    @Test("A streak may end yesterday but not before yesterday")
    func appliesStreakGraceDay() throws {
        let now = try #require(calendar.date(from: DateComponents(
            year: 2026, month: 8, day: 31, hour: 10
        )))
        let start = calendar.startOfDay(for: now)
        let yesterday = try #require(calendar.date(byAdding: .day, value: -1, to: start))
        let twoDaysAgo = try #require(calendar.date(byAdding: .day, value: -2, to: start))
        let fourDaysAgo = try #require(calendar.date(byAdding: .day, value: -4, to: start))

        let activeYesterday = CodexLocalActivitySummarizer(calendar: calendar).summarize([
            CodexLocalThreadActivity(tokensUsed: 10, createdAt: twoDaysAgo, updatedAt: yesterday),
        ], now: now)
        let stale = CodexLocalActivitySummarizer(calendar: calendar).summarize([
            CodexLocalThreadActivity(tokensUsed: 10, createdAt: fourDaysAgo, updatedAt: twoDaysAgo),
        ], now: now)

        #expect(activeYesterday.currentStreakDays == 2)
        #expect(stale.currentStreakDays == 0)
    }

    @Test("Parses only bounded numeric SQLite output")
    func parsesSQLiteRows() throws {
        let text = "120\t1788100000\t1788100300\ninvalid\trow\n50\t1788100400\t1788100100\n"

        let rows = CodexLocalActivityRowParser().parse(text)

        #expect(rows.count == 2)
        #expect(rows[0].tokensUsed == 120)
        #expect(rows[0].updatedAt.timeIntervalSince(rows[0].createdAt) == 300)
        #expect(rows[1].tokensUsed == 50)
    }
}
