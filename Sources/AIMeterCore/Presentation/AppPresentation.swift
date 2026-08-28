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

    public init(snapshot: UsageSnapshot) {
        provider = snapshot.provider
        title = Self.title(for: snapshot.provider)
        fraction = [snapshot.primaryMetric, snapshot.secondaryMetric]
            .compactMap { $0?.usedFraction }
            .max()
        detailText = SensitiveTextRedactor.redact(
            snapshot.primaryMetric?.label ?? Self.defaultDetail(for: snapshot.collectionStatus)
        )
        statusText = snapshot.statusMessage.map(SensitiveTextRedactor.redact)
        primaryResetText = Self.resetText(for: snapshot.primaryMetric)
        secondaryResetText = Self.resetText(for: snapshot.secondaryMetric)

        if let primaryMetric = snapshot.primaryMetric {
            valueText = Self.valueText(for: primaryMetric)
        } else {
            valueText = Self.stateValue(for: snapshot.collectionStatus)
        }

        if snapshot.collectionStatus == .cached {
            semantic = .stale
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
        switch provider {
        case .claude: "Claude"
        case .codex: "Codex"
        case .deepSeek: "DeepSeek"
        }
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
    public let valueText: String
    public let accessibilityLabel: String

    public init(snapshots: [UsageSnapshot]) {
        let presentations = snapshots.map(ProviderPresentation.init(snapshot:))
        if let highest = presentations.compactMap(\.fraction).max() {
            let percent = Int((highest * 100).rounded())
            valueText = "\(percent)%"
            accessibilityLabel = "AI Meter, highest usage \(percent) percent"
            if highest >= 0.90 {
                semantic = .critical
            } else if highest >= 0.70 {
                semantic = .warning
            } else {
                semantic = .normal
            }
        } else {
            valueText = "—"
            accessibilityLabel = "AI Meter, usage unavailable"
            semantic = presentations.contains(where: { $0.semantic == .stale }) ? .stale : .unavailable
        }
    }
}
