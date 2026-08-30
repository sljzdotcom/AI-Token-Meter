import Foundation

public struct DeepSeekWebsitePayloadParser: Sendable {
    public enum Error: Swift.Error, Equatable {
        case payloadTooLarge
        case invalidJSON
        case unrecognizedPayload
    }

    private let maxPayloadBytes: Int

    public init(maxPayloadBytes: Int = 2 * 1_024 * 1_024) {
        self.maxPayloadBytes = maxPayloadBytes
    }

    public func parse(_ data: Data) throws -> [DeepSeekDailyUsage] {
        guard data.count <= maxPayloadBytes else { throw Error.payloadTooLarge }
        let root: Any
        do {
            root = try JSONSerialization.jsonObject(with: data)
        } catch {
            throw Error.invalidJSON
        }
        let currentRows = currentBucketRows(from: root)
        if !currentRows.isEmpty {
            return mergeRowsByTimestamp(currentRows)
        }
        var rows: [DeepSeekDailyUsage] = []
        collectRows(from: root, depth: 0, into: &rows)
        guard !rows.isEmpty else { throw Error.unrecognizedPayload }
        return rows
    }

    private func currentBucketRows(from root: Any) -> [DeepSeekDailyUsage] {
        var rows: [DeepSeekDailyUsage] = []
        collectCurrentBuckets(from: root, depth: 0, into: &rows)
        return rows
    }

    private func collectCurrentBuckets(from value: Any, depth: Int, into rows: inout [DeepSeekDailyUsage]) {
        guard depth <= 14, rows.count < 2_000 else { return }
        if let object = value as? [String: Any] {
            let values = Dictionary(uniqueKeysWithValues: object.map { (normalize($0.key), $0.value) })
            if let time = firstValue(in: values, keys: ["time", "timestamp"]),
               let date = parseDate(time) {
                if let usage = firstValue(in: values, keys: ["usage"]) as? [String: Any],
                   let row = amountBucket(date: date, usage: usage) {
                    rows.append(row)
                    return
                }
                if let cost = number(firstValue(in: values, keys: ["cost"])), cost.isFinite {
                    rows.append(DeepSeekDailyUsage(
                        date: date,
                        costCNY: max(cost, 0),
                        requestCount: 0,
                        tokenCount: 0
                    ))
                    return
                }
            }
            for child in object.values {
                collectCurrentBuckets(from: child, depth: depth + 1, into: &rows)
            }
        } else if let array = value as? [Any] {
            for child in array {
                collectCurrentBuckets(from: child, depth: depth + 1, into: &rows)
            }
        }
    }

    private func amountBucket(date: Date, usage: [String: Any]) -> DeepSeekDailyUsage? {
        let values = Dictionary(uniqueKeysWithValues: usage.map { (normalize($0.key), $0.value) })
        let cacheHit = integer(firstValue(in: values, keys: ["promptcachehittoken"])) ?? 0
        let cacheMiss = integer(firstValue(in: values, keys: ["promptcachemisstoken"])) ?? 0
        let response = integer(firstValue(in: values, keys: ["responsetoken"])) ?? 0
        let requestCount = integer(firstValue(in: values, keys: ["request"])) ?? 0
        guard cacheHit != 0 || cacheMiss != 0 || response != 0 || requestCount != 0 else { return nil }
        return DeepSeekDailyUsage(
            date: date,
            costCNY: 0,
            requestCount: max(requestCount, 0),
            tokenCount: max(cacheHit, 0) + max(cacheMiss, 0) + max(response, 0)
        )
    }

    private func mergeRowsByTimestamp(_ rows: [DeepSeekDailyUsage]) -> [DeepSeekDailyUsage] {
        let merged = rows.reduce(into: [Date: DeepSeekDailyUsage]()) { result, row in
            let existing = result[row.date]
            result[row.date] = DeepSeekDailyUsage(
                date: row.date,
                costCNY: (existing?.costCNY ?? 0) + row.costCNY,
                requestCount: (existing?.requestCount ?? 0) + row.requestCount,
                tokenCount: (existing?.tokenCount ?? 0) + row.tokenCount
            )
        }
        return merged.values.sorted { $0.date < $1.date }
    }

    private func collectRows(from value: Any, depth: Int, into rows: inout [DeepSeekDailyUsage]) {
        guard depth <= 12, rows.count < 400 else { return }
        if let object = value as? [String: Any] {
            if let row = usageRow(from: object) {
                rows.append(row)
                return
            }
            for child in object.values {
                collectRows(from: child, depth: depth + 1, into: &rows)
            }
        } else if let array = value as? [Any] {
            for child in array {
                collectRows(from: child, depth: depth + 1, into: &rows)
            }
        }
    }

    private func usageRow(from object: [String: Any]) -> DeepSeekDailyUsage? {
        let values = Dictionary(uniqueKeysWithValues: object.map { (normalize($0.key), $0.value) })
        guard let dateValue = firstValue(in: values, keys: ["date", "usagedate", "usagetime", "day", "statdate", "createddate", "createdat", "datetime", "timestamp", "time"]),
              let date = parseDate(dateValue) else {
            return nil
        }
        let cost = number(firstValue(in: values, keys: ["totalcost", "costcny", "cost", "amount", "chargedamount", "totalprice", "price", "spend", "totalspend", "billingamount"])) ?? 0
        let requests = integer(firstValue(in: values, keys: ["requestcount", "requests", "nrequests", "requestnum", "requestnumber", "apirequests", "apicallcount", "calls"])) ?? 0
        let directTokens = integer(firstValue(in: values, keys: ["totaltokens", "tokencount", "tokens"]))
        let inputTokens = integer(firstValue(in: values, keys: ["inputtokens", "prompttokens"])) ?? 0
        let outputTokens = integer(firstValue(in: values, keys: ["outputtokens", "completiontokens"])) ?? 0
        let tokens = directTokens ?? (inputTokens + outputTokens)
        guard cost != 0 || requests != 0 || tokens != 0 || containsUsageField(values) else { return nil }
        return DeepSeekDailyUsage(
            date: date,
            costCNY: max(cost, 0),
            requestCount: max(requests, 0),
            tokenCount: max(tokens, 0)
        )
    }

    private func containsUsageField(_ values: [String: Any]) -> Bool {
        let usageKeys: Set<String> = [
            "totalcost", "costcny", "cost", "amount", "totalprice", "spend",
            "requestcount", "requests", "nrequests", "requestnum", "apirequests",
            "totaltokens", "tokencount", "tokens", "inputtokens", "outputtokens",
        ]
        return !usageKeys.isDisjoint(with: values.keys)
    }

    private func firstValue(in values: [String: Any], keys: [String]) -> Any? {
        keys.lazy.compactMap { values[$0] }.first
    }

    private func number(_ value: Any?) -> Double? {
        if let number = value as? NSNumber { return number.doubleValue }
        if let text = value as? String {
            let normalized = text.replacingOccurrences(of: ",", with: "")
                .replacingOccurrences(of: "¥", with: "")
                .trimmingCharacters(in: .whitespacesAndNewlines)
            return Double(normalized)
        }
        return nil
    }

    private func integer(_ value: Any?) -> Int? {
        guard let number = number(value), number.isFinite else { return nil }
        return Int(number.rounded())
    }

    private func parseDate(_ value: Any) -> Date? {
        if let number = value as? NSNumber {
            let raw = number.doubleValue
            return Date(timeIntervalSince1970: raw > 10_000_000_000 ? raw / 1_000 : raw)
        }
        guard let text = value as? String else { return nil }
        if let iso = ISO8601DateFormatter().date(from: text) { return iso }
        for pattern in ["yyyy-MM-dd", "yyyy/MM/dd", "MM/dd/yyyy"] {
            let formatter = DateFormatter()
            formatter.locale = Locale(identifier: "en_US_POSIX")
            formatter.timeZone = .current
            formatter.dateFormat = pattern
            if let date = formatter.date(from: text) { return date }
        }
        return nil
    }

    private func normalize(_ key: String) -> String {
        key.lowercased().filter(\.isLetter)
    }
}

public enum DeepSeekUsageFacet: Sendable {
    case amount
    case cost

    public init?(responseURL: URL) {
        let path = responseURL.path.lowercased()
        if path.hasSuffix("/amount") {
            self = .amount
        } else if path.hasSuffix("/cost") {
            self = .cost
        } else {
            return nil
        }
    }
}

public struct DeepSeekUsageFacetAccumulator: Sendable {
    private var amountRows: [DeepSeekDailyUsage]?
    private var costRows: [DeepSeekDailyUsage]?

    public init() {}

    public mutating func replace(
        _ facet: DeepSeekUsageFacet,
        rows: [DeepSeekDailyUsage]
    ) -> [DeepSeekDailyUsage]? {
        switch facet {
        case .amount: amountRows = rows
        case .cost: costRows = rows
        }
        guard let amountRows, let costRows else { return nil }
        let allRows = amountRows + costRows
        let merged = allRows.reduce(into: [Date: DeepSeekDailyUsage]()) { result, row in
            let existing = result[row.date]
            result[row.date] = DeepSeekDailyUsage(
                date: row.date,
                costCNY: (existing?.costCNY ?? 0) + row.costCNY,
                requestCount: (existing?.requestCount ?? 0) + row.requestCount,
                tokenCount: (existing?.tokenCount ?? 0) + row.tokenCount
            )
        }
        return merged.values.sorted { $0.date < $1.date }
    }
}
