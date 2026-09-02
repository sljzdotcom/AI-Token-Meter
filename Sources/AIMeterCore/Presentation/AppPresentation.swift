import Foundation

public enum UsageSemantic: String, Codable, Equatable, Sendable {
    case normal
    case warning
    case critical
    case stale
    case unavailable
}

public struct ProviderPresentation: Equatable, Sendable {
    public let provider: UsageProvider
    public let title: String
    public let valueText: String
    public let detailText: String
    public let statusText: String?
    public let primaryResetText: String?
    public let secondaryResetText: String?
    public let fraction: Double?
    public let semantic: UsageSemantic

    public var ringFraction: Double? {
        guard let fraction, fraction > 0 else { return nil }
        return fraction
    }

    public var accessibilityStatusText: String? {
        let semanticText: String? = switch semantic {
        case .normal: nil
        case .warning: "Warning"
        case .critical: "Critical usage"
        case .stale: "Cached data"
        case .unavailable: "Unavailable"
        }
        let components = [semanticText, statusText].compactMap { $0 }
        return components.isEmpty ? nil : components.joined(separator: ". ")
    }

    public init(snapshot: UsageSnapshot) {
        provider = snapshot.provider
        title = Self.title(for: snapshot.provider)
        let percentageMetrics = [snapshot.primaryMetric, snapshot.secondaryMetric]
            .compactMap { $0 }
            .filter { $0.kind == .officialLimit && $0.unit == .percent }
        let officialSummaryMetric = percentageMetrics.max {
            ($0.usedFraction ?? 0) < ($1.usedFraction ?? 0)
        }
        let summaryMetric = snapshot.provider == .deepSeek
            ? snapshot.primaryMetric
            : (officialSummaryMetric ?? snapshot.primaryMetric)
        if snapshot.provider == .deepSeek,
           snapshot.secondaryMetric?.kind == .localBudget {
            fraction = snapshot.secondaryMetric?.usedFraction
        } else {
            fraction = summaryMetric?.unit == .percent ? summaryMetric?.usedFraction : nil
        }
        detailText = SensitiveTextRedactor.redact(
            summaryMetric?.label ?? Self.defaultDetail(for: snapshot.collectionStatus)
        )
        if let statusMessage = snapshot.statusMessage {
            statusText = SensitiveTextRedactor.redact(statusMessage)
        } else if snapshot.availability == .unavailable {
            statusText = "Account unavailable"
        } else {
            statusText = nil
        }
        primaryResetText = Self.resetText(for: snapshot.primaryMetric)
        secondaryResetText = Self.resetText(for: snapshot.secondaryMetric)

        if let summaryMetric {
            valueText = Self.valueText(for: summaryMetric)
        } else {
            valueText = Self.stateValue(for: snapshot.collectionStatus)
        }

        if snapshot.collectionStatus == .cached {
            semantic = .stale
        } else if snapshot.availability != .available {
            semantic = .unavailable
        } else if snapshot.collectionStatus != .fresh {
            semantic = .unavailable
        } else if let fraction, fraction >= 0.90 {
            semantic = .critical
        } else if let fraction, fraction >= 0.70 {
            semantic = .warning
        } else {
            semantic = .normal
        }
    }

    private static func title(for provider: UsageProvider) -> String {
        provider.displayName
    }

    private static func valueText(for metric: UsageMetric) -> String {
        if let fraction = metric.usedFraction {
            return "\(Int((fraction * 100).rounded()))%"
        }
        switch metric.unit {
        case .cny: return String(format: "¥%.2f", metric.current)
        case .usd: return String(format: "$%.2f", metric.current)
        case .percent: return String(format: "%.0f%%", metric.current)
        case .tokens: return String(format: "%.0f tokens", metric.current)
        case .requests: return String(format: "%.0f requests", metric.current)
        }
    }

    private static func stateValue(for status: CollectionStatus) -> String {
        switch status {
        case .authenticationRequired: "Sign in"
        case .setupRequired: "Set up"
        case .notInstalled: "Not installed"
        case .refreshing: "Refreshing"
        case .unrecognizedOutput: "Update needed"
        case .unavailable: "Unavailable"
        case .fresh, .cached: "—"
        }
    }

    private static func defaultDetail(for status: CollectionStatus) -> String {
        switch status {
        case .authenticationRequired: "Account connection required"
        case .setupRequired: "One-time Claude Code workspace approval"
        case .notInstalled: "CLI was not found"
        case .refreshing: "Checking current usage"
        case .unrecognizedOutput: "Usage format changed"
        case .unavailable: "No current data"
        case .fresh, .cached: "Usage"
        }
    }

    private static func resetText(for metric: UsageMetric?) -> String? {
        guard let metric else { return nil }
        if let description = metric.resetDescription {
            return SensitiveTextRedactor.redact(description)
        }
        if let resetAt = metric.resetAt {
            return "Resets \(resetAt.formatted(date: .abbreviated, time: .shortened))"
        }
        return nil
    }
}

public struct MenuBarSummary: Equatable, Sendable {
    public let semantic: UsageSemantic
    public let usageFraction: Double?
    public let valueText: String
    public let accessibilityLabel: String

    public init(snapshots: [UsageSnapshot]) {
        let presentations = snapshots.map(ProviderPresentation.init(snapshot:))
        let availablePresentations = zip(snapshots, presentations)
            .filter { snapshot, _ in snapshot.availability == .available }
            .map(\.1)
        if let highest = availablePresentations
            .compactMap(\.fraction)
            .filter({ $0.isFinite })
            .max() {
            let normalizedHighest = min(max(highest, 0), 1)
            usageFraction = normalizedHighest
            let percent = Int((normalizedHighest * 100).rounded())
            valueText = "\(percent)%"
            accessibilityLabel = "\(AppBrand.displayName), highest usage \(percent) percent"
            if normalizedHighest >= 0.90 {
                semantic = .critical
            } else if normalizedHighest >= 0.70 {
                semantic = .warning
            } else {
                semantic = .normal
            }
        } else {
            usageFraction = nil
            valueText = "—"
            accessibilityLabel = "\(AppBrand.displayName), usage unavailable"
            semantic = presentations.contains(where: { $0.semantic == .stale }) ? .stale : .unavailable
        }
    }
}

public struct CodexLocalActivityPresentation: Equatable, Sendable {
    public let tokenText: String
    public let streakText: String
    public let longestSessionText: String

    public init(summary: CodexLocalActivitySummary) {
        tokenText = Self.compactCount(summary.tokenCount)
        streakText = "\(summary.currentStreakDays) " + (summary.currentStreakDays == 1 ? "day" : "days")
        longestSessionText = Self.duration(summary.longestSessionDuration)
    }

    private static func compactCount(_ count: Int64) -> String {
        let value = Double(max(count, 0))
        let units: [(threshold: Double, suffix: String)] = [
            (1_000_000_000, "B"),
            (1_000_000, "M"),
            (1_000, "K"),
        ]
        guard let unit = units.first(where: { value >= $0.threshold }) else {
            return count.formatted()
        }
        let scaled = value / unit.threshold
        let digits = scaled >= 100 ? 0 : 1
        return String(format: "%.*f", digits, scaled)
            .replacingOccurrences(of: ".0", with: "") + unit.suffix
    }

    private static func duration(_ duration: TimeInterval) -> String {
        let minutes = max(Int(duration / 60), 0)
        let days = minutes / (24 * 60)
        let hours = (minutes % (24 * 60)) / 60
        let remainingMinutes = minutes % 60
        if days > 0 {
            return hours > 0 ? "\(days)d \(hours)h" : "\(days)d"
        }
        if hours > 0 {
            return remainingMinutes > 0 ? "\(hours)h \(remainingMinutes)m" : "\(hours)h"
        }
        return "\(minutes)m"
    }
}

public enum CodexResetCreditExpirationState: Equatable, Sendable {
    case remaining(days: Int)
    case today
    case expired
    case unavailable
}

public struct CodexResetCreditRowPresentation: Equatable, Sendable {
    public let title: String
    public let expiresAt: Date?
    public let statusText: String
    public let expirationState: CodexResetCreditExpirationState
}

public struct CodexResetCreditsPresentation: Equatable, Sendable {
    public let availableText: String
    public let rows: [CodexResetCreditRowPresentation]
    public let showsIncompleteDetails: Bool

    public init(
        summary: CodexResetCreditsSummary,
        now: Date = Date(),
        calendar: Calendar = .current
    ) {
        availableText = "\(summary.availableCount) available"
        showsIncompleteDetails = !summary.hasCompleteDetails
            || summary.availableCount > summary.credits.count

        let ordered = summary.credits.enumerated().sorted { lhs, rhs in
            switch (lhs.element.expiresAt, rhs.element.expiresAt) {
            case let (left?, right?) where left != right:
                return left < right
            case (_?, nil):
                return true
            case (nil, _?):
                return false
            default:
                return lhs.offset < rhs.offset
            }
        }
        let today = calendar.startOfDay(for: now)
        rows = ordered.map { _, credit in
            let expiration = Self.expiration(
                expiresAt: credit.expiresAt,
                today: today,
                calendar: calendar
            )
            return CodexResetCreditRowPresentation(
                title: credit.title ?? "Usage reset",
                expiresAt: credit.expiresAt,
                statusText: expiration.text,
                expirationState: expiration.state
            )
        }
    }

    private static func expiration(
        expiresAt: Date?,
        today: Date,
        calendar: Calendar
    ) -> (text: String, state: CodexResetCreditExpirationState) {
        guard let expiresAt else {
            return ("Expiration unavailable", .unavailable)
        }
        let expirationDay = calendar.startOfDay(for: expiresAt)
        let dayCount = calendar.dateComponents([.day], from: today, to: expirationDay).day ?? 0
        if dayCount < 0 {
            return ("Expired", .expired)
        }
        if dayCount == 0 {
            return ("Expires today", .today)
        }
        let unit = dayCount == 1 ? "day" : "days"
        return ("\(dayCount) \(unit) remaining", .remaining(days: dayCount))
    }
}
