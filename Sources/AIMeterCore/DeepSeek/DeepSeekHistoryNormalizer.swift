import Foundation

public struct DeepSeekHistoryNormalizer: Sendable {
    private let calendar: Calendar

    public init(calendar: Calendar = .current) {
        self.calendar = calendar
    }

    public func normalize(
        records: [DeepSeekDailyUsage],
        endingAt: Date = Date(),
        updatedAt: Date = Date(),
        statusMessage: String? = nil
    ) -> DeepSeekUsageHistory {
        let end = calendar.startOfDay(for: endingAt)
        let start = calendar.date(byAdding: .day, value: -29, to: end) ?? end
        var totalsByDay: [Date: DayTotals] = [:]

        for record in records {
            let day = calendar.startOfDay(for: record.date)
            guard day >= start, day <= end else { continue }
            let prior = totalsByDay[day] ?? DayTotals()
            totalsByDay[day] = DayTotals(
                costCNY: prior.costCNY + max(record.costCNY, 0),
                requestCount: prior.requestCount + max(record.requestCount, 0),
                tokenCount: prior.tokenCount + max(record.tokenCount, 0)
            )
        }

        let days = (0..<30).compactMap { offset -> DeepSeekDailyUsage? in
            guard let day = calendar.date(byAdding: .day, value: offset, to: start) else {
                return nil
            }
            let totals = totalsByDay[day] ?? DayTotals()
            return DeepSeekDailyUsage(
                date: day,
                costCNY: totals.costCNY,
                requestCount: totals.requestCount,
                tokenCount: totals.tokenCount
            )
        }

        return DeepSeekUsageHistory(
            days: days,
            updatedAt: updatedAt,
            statusMessage: statusMessage.map(SensitiveTextRedactor.redact)
        )
    }
}

private struct DayTotals {
    var costCNY: Double = 0
    var requestCount: Int = 0
    var tokenCount: Int = 0
}
