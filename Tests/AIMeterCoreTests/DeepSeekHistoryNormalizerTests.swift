import Foundation
import Testing
@testable import AIMeterCore

@Suite("DeepSeek history normalizer")
struct DeepSeekHistoryNormalizerTests {
    @Test("Normalizes exactly thirty local days and fills missing dates")
    func normalizesThirtyDays() throws {
        let calendar = Self.utcCalendar
        let normalizer = DeepSeekHistoryNormalizer(calendar: calendar)
        let end = try Self.date("2026-08-30")

        let history = normalizer.normalize(
            records: [
                DeepSeekDailyUsage(date: try Self.date("2026-08-29"), costCNY: 2.5, requestCount: 10, tokenCount: 1_000),
                DeepSeekDailyUsage(date: try Self.date("2026-08-29"), costCNY: 1.5, requestCount: 5, tokenCount: 500),
                DeepSeekDailyUsage(date: try Self.date("2026-07-01"), costCNY: 99, requestCount: 99, tokenCount: 99),
            ],
            endingAt: end,
            updatedAt: end
        )

        #expect(history.days.count == 30)
        #expect(history.days.last?.date == end)
        #expect(history.days.last?.costCNY == 0)
        #expect(history.days.dropLast().last?.costCNY == 4)
        #expect(history.totalRequests == 15)
        #expect(history.totalTokens == 1_500)
    }

    @Test("Negative website values are rejected from totals")
    func clampsNegativeValues() throws {
        let end = try Self.date("2026-08-30")
        let history = DeepSeekHistoryNormalizer(calendar: Self.utcCalendar).normalize(
            records: [
                DeepSeekDailyUsage(date: end, costCNY: -1, requestCount: -2, tokenCount: -3),
            ],
            endingAt: end,
            updatedAt: end
        )

        #expect(history.totalCostCNY == 0)
        #expect(history.totalRequests == 0)
        #expect(history.totalTokens == 0)
    }

    private static var utcCalendar: Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(secondsFromGMT: 0)!
        return calendar
    }

    private static func date(_ value: String) throws -> Date {
        let formatter = DateFormatter()
        formatter.calendar = utcCalendar
        formatter.timeZone = utcCalendar.timeZone
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd"
        return try #require(formatter.date(from: value))
    }
}
