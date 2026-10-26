import Foundation

public enum Code0WebsiteCaptureError: Error, Equatable, Sendable {
    case invalidPayload
}

/// Accepts only the three filtered quota numbers, never an account response.
public struct Code0WebsiteCapture: Equatable, Sendable {
    public let balance: Double
    public let cumulativeUsage: Double

    private init(balance: Double, cumulativeUsage: Double) {
        self.balance = balance
        self.cumulativeUsage = cumulativeUsage
    }

    /// The bounded wire contract contains exactly quota, usedQuota and divisor.
    public static func decode(filteredPayload: Data) throws -> Self {
        guard filteredPayload.count <= 2048 else { throw Code0WebsiteCaptureError.invalidPayload }
        var parser = FilteredNumbers(bytes: Array(filteredPayload))
        let fields = try parser.parse()
        guard let quotaText = fields["quota"], let usedText = fields["usedQuota"],
              let divisorText = fields["divisor"],
              isSafeInteger(quotaText), isSafeInteger(usedText),
              let quota = Double(quotaText), let used = Double(usedText),
              let divisor = Double(divisorText), divisor.isFinite, divisor > 0,
              divisor <= 9_007_199_254_740_991,
              Self.isWithinMaximum(divisorText, value: divisor) else { throw Code0WebsiteCaptureError.invalidPayload }
        let balance = quota / divisor
        let cumulativeUsage = used / divisor
        guard balance.isFinite, cumulativeUsage.isFinite else { throw Code0WebsiteCaptureError.invalidPayload }
        return Self(balance: balance == 0 ? 0 : balance, cumulativeUsage: cumulativeUsage == 0 ? 0 : cumulativeUsage)
    }

    public func makeSnapshot(fetchedAt: Date) throws -> UsageSnapshot {
        guard fetchedAt.timeIntervalSince1970.isFinite else { throw Code0WebsiteCaptureError.invalidPayload }
        return UsageSnapshot(
            provider: .code0,
            primaryMetric: UsageMetric(label: "Balance", current: balance, limit: nil, unit: .siteCredits, kind: .balance),
            secondaryMetric: UsageMetric(label: "Cumulative usage", current: cumulativeUsage, limit: nil, unit: .siteCredits, kind: .cumulativeUsage),
            fetchedAt: fetchedAt, staleAfter: 300, sourceVersion: "website", collectionStatus: .fresh
        )
    }

    // Compare the mathematical decimal integer before binary floating-point
    // rounding, which could otherwise accept e.g. 9007199254740991.1.
    private static func isSafeInteger(_ text: String) -> Bool {
        guard let number = Double(text), number.isFinite, number >= 0,
              number <= 9_007_199_254_740_991 else { return false }
        let parts = text.lowercased().split(separator: "e", omittingEmptySubsequences: false)
        let mantissa = String(parts[0])
        let fractionCount = mantissa.split(separator: ".", omittingEmptySubsequences: false).dropFirst().first?.count ?? 0
        var digits = String(mantissa.filter(\.isNumber).drop(while: { $0 == "0" }))
        if digits.isEmpty { return true }
        let exponent: Int
        if parts.count == 1 { exponent = 0 }
        else {
            guard let parsed = Int(parts[1]) else { return false }
            exponent = parsed
        }
        guard !text.hasPrefix("-"), (-4096...4096).contains(exponent) else { return false }
        let shift = exponent - fractionCount
        if shift < 0 {
            let count = -shift
            guard count < digits.count, digits.suffix(count).allSatisfy({ $0 == "0" }) else { return false }
            digits.removeLast(count)
        } else {
            guard digits.count + shift <= 16 else { return false }
            digits += String(repeating: "0", count: shift)
        }
        return digits.count < 16 || (digits.count == 16 && digits <= "9007199254740991")
    }

    private static func isWithinMaximum(_ text: String, value: Double) -> Bool {
        guard value == 9_007_199_254_740_991 else { return true }
        let parts = text.lowercased().split(separator: "e", omittingEmptySubsequences: false)
        let mantissa = String(parts[0])
        let fractionCount = mantissa.split(separator: ".", omittingEmptySubsequences: false).dropFirst().first?.count ?? 0
        let digits = String(mantissa.filter(\.isNumber).drop(while: { $0 == "0" }))
        let exponentText = parts.count == 1 ? "0" : String(parts[1])
        guard let exponent = Int(exponentText), (-4096...4096).contains(exponent) else { return false }
        let point = digits.count + exponent - fractionCount
        guard point == 16 else { return point < 16 }
        let integer = String(digits.prefix(16)) + String(repeating: "0", count: max(0, 16 - digits.count))
        if integer != "9007199254740991" { return integer < "9007199254740991" }
        return digits.dropFirst(16).allSatisfy { $0 == "0" }
    }
}

/// A deliberately narrow JSON reader: fixed ASCII keys and JSON numeric tokens.
/// Reading pairs directly keeps duplicate keys observable instead of losing them in a dictionary decoder.
private struct FilteredNumbers {
    let bytes: [UInt8]
    var index = 0

    mutating func parse() throws -> [String: String] {
        guard !bytes.isEmpty, bytes.count <= 2048, take(123) else { throw Code0WebsiteCaptureError.invalidPayload }
        var result: [String: String] = [:]
        while true {
            guard take(34) else { throw Code0WebsiteCaptureError.invalidPayload }
            let start = index
            while index < bytes.count, (65...90).contains(bytes[index]) || (97...122).contains(bytes[index]) { index += 1 }
            let key = String(decoding: bytes[start..<index], as: UTF8.self)
            guard ["quota", "usedQuota", "divisor"].contains(key), result[key] == nil,
                  index < bytes.count, bytes[index] == 34 else { throw Code0WebsiteCaptureError.invalidPayload }
            index += 1
            guard take(58) else { throw Code0WebsiteCaptureError.invalidPayload }
            skipSpace()
            let numberStart = index
            if index < bytes.count, bytes[index] == 45 { index += 1 }
            guard index < bytes.count else { throw Code0WebsiteCaptureError.invalidPayload }
            if bytes[index] == 48 { index += 1 }
            else {
                guard (49...57).contains(bytes[index]) else { throw Code0WebsiteCaptureError.invalidPayload }
                consumeDigits()
            }
            if index < bytes.count, bytes[index] == 46 {
                index += 1
                let fractionStart = index
                consumeDigits()
                guard index > fractionStart else { throw Code0WebsiteCaptureError.invalidPayload }
            }
            if index < bytes.count, bytes[index] == 101 || bytes[index] == 69 {
                index += 1
                if index < bytes.count, bytes[index] == 43 || bytes[index] == 45 { index += 1 }
                let exponentStart = index
                consumeDigits()
                guard index > exponentStart else { throw Code0WebsiteCaptureError.invalidPayload }
            }
            result[key] = String(decoding: bytes[numberStart..<index], as: UTF8.self)
            if take(125) { break }
            guard take(44) else { throw Code0WebsiteCaptureError.invalidPayload }
        }
        skipSpace()
        guard index == bytes.count, result.count == 3 else { throw Code0WebsiteCaptureError.invalidPayload }
        return result
    }

    mutating func take(_ byte: UInt8) -> Bool {
        skipSpace()
        guard index < bytes.count, bytes[index] == byte else { return false }
        index += 1
        return true
    }
    mutating func skipSpace() { while index < bytes.count, [9, 10, 13, 32].contains(bytes[index]) { index += 1 } }
    mutating func consumeDigits() { while index < bytes.count, (48...57).contains(bytes[index]) { index += 1 } }
}
