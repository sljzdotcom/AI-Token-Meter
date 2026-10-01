import Foundation

public enum RefreshFailureKind: String, Codable, Sendable { case rateLimited, network, cli, authentication, suspended }

public struct RefreshBackoffState: Codable, Sendable {
    public private(set) var failureKind: RefreshFailureKind = .network
    public private(set) var consecutiveFailures = 0
    public private(set) var nextEligibleAt: TimeInterval = 0
    public private(set) var recordedAt: TimeInterval = 0
    public init() {}
    public mutating func record(_ kind: RefreshFailureKind, now: TimeInterval, retryAfter: TimeInterval = 0) {
        consecutiveFailures = kind == failureKind ? min(consecutiveFailures + 1, 10) : 1
        failureKind = kind
        recordedAt = now
        if kind == .suspended {
            nextEligibleAt = now
            return
        }
        let base: Double = kind == .rateLimited ? 60 : kind == .authentication ? 900 : 30
        let cap: Double = kind == .rateLimited || kind == .authentication ? 900 : kind == .cli ? 300 : 600
        let wait = kind == .authentication ? 900 : min(base * pow(2, Double(consecutiveFailures - 1)), cap)
        nextEligibleAt = now + min(max(wait, retryAfter.isFinite ? retryAfter : 0), 86400)
    }
    public func isEligible(now: TimeInterval, manual: Bool) -> Bool {
        guard failureKind != .suspended else { return false }
        var normalized = self
        normalized.normalizeClock(now: now)
        if manual && (failureKind == .network || failureKind == .cli) { return true }
        return now >= normalized.nextEligibleAt
    }
    public mutating func normalizeClock(now: TimeInterval) {
        if now < recordedAt {
            nextEligibleAt = now + min(max(nextEligibleAt - recordedAt, 0), 86400)
            recordedAt = now
        }
        if !nextEligibleAt.isFinite || nextEligibleAt - now > 86400 {
            nextEligibleAt = now + 86400
            recordedAt = now
        }
    }
}

public enum RetryAfterParser {
    public static func seconds(_ value: String?, now: Date = Date()) -> TimeInterval {
        guard let value else { return 0 }
        if let seconds = Double(value), seconds.isFinite { return min(max(seconds, 0), 86400) }
        let format = DateFormatter()
        format.locale = Locale(identifier: "en_US_POSIX")
        format.timeZone = TimeZone(secondsFromGMT: 0)
        format.dateFormat = "EEE, dd MMM yyyy HH:mm:ss z"
        return format.date(from: value).map { min(max($0.timeIntervalSince(now), 0), 86400) } ?? 0
    }
}
