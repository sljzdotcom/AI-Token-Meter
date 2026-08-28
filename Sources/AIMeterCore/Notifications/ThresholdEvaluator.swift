import Foundation

public enum ThresholdLevel: Int, Codable, Equatable, Sendable {
    case warning = 70
    case critical = 90
}

public struct ThresholdEvent: Equatable, Sendable {
    public let provider: UsageProvider
    public let metricLabel: String
    public let level: ThresholdLevel
    public let usedFraction: Double

    public init(
        provider: UsageProvider,
        metricLabel: String,
        level: ThresholdLevel,
        usedFraction: Double
    ) {
        self.provider = provider
        self.metricLabel = metricLabel
        self.level = level
        self.usedFraction = usedFraction
    }
}

public struct ThresholdEvaluator: Sendable {
    private var states: [String: ThresholdState] = [:]

    public init() {}

    public mutating func evaluate(_ snapshot: UsageSnapshot) -> [ThresholdEvent] {
        guard snapshot.collectionStatus == .fresh else { return [] }
        return [snapshot.primaryMetric, snapshot.secondaryMetric]
            .compactMap { $0 }
            .compactMap { evaluate(metric: $0, provider: snapshot.provider) }
    }

    private mutating func evaluate(
        metric: UsageMetric,
        provider: UsageProvider
    ) -> ThresholdEvent? {
        guard let fraction = metric.usedFraction else { return nil }
        let key = "\(provider.rawValue)|\(metric.kind.rawValue)|\(metric.label)"
        let cycle = cycleIdentifier(for: metric)
        var state = states[key] ?? ThresholdState(cycleIdentifier: cycle, lastNotified: nil)

        if state.cycleIdentifier != cycle {
            state = ThresholdState(cycleIdentifier: cycle, lastNotified: nil)
        }

        if fraction < 0.10 {
            state.lastNotified = nil
            states[key] = state
            return nil
        }

        let reachedLevel: ThresholdLevel?
        if fraction >= 0.90 {
            reachedLevel = .critical
        } else if fraction >= 0.70 {
            reachedLevel = .warning
        } else {
            reachedLevel = nil
        }

        guard let reachedLevel else {
            states[key] = state
            return nil
        }
        if let lastNotified = state.lastNotified,
           lastNotified.rawValue >= reachedLevel.rawValue {
            states[key] = state
            return nil
        }

        state.lastNotified = reachedLevel
        states[key] = state
        return ThresholdEvent(
            provider: provider,
            metricLabel: metric.label,
            level: reachedLevel,
            usedFraction: fraction
        )
    }

    private func cycleIdentifier(for metric: UsageMetric) -> String? {
        if let resetAt = metric.resetAt {
            return "at:\(resetAt.timeIntervalSince1970)"
        }
        return nil
    }
}

private struct ThresholdState: Sendable {
    let cycleIdentifier: String?
    var lastNotified: ThresholdLevel?
}
