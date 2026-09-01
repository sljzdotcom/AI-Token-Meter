import Foundation

public struct WidgetSnapshotBuilder: Sendable {
    public init() {}

    public func build(
        snapshots: [UsageSnapshot],
        generatedAt: Date = Date()
    ) -> WidgetSnapshotEnvelope {
        let snapshotsByProvider = Dictionary(
            snapshots.map { ($0.provider, $0) },
            uniquingKeysWith: { _, latest in latest }
        )
        let providers = WidgetProvider.allCases.map { provider in
            guard let usageProvider = UsageProvider(widgetProvider: provider),
                  let snapshot = snapshotsByProvider[usageProvider] else {
                return Self.unavailable(provider)
            }
            return Self.providerSnapshot(from: snapshot, generatedAt: generatedAt)
        }

        return WidgetSnapshotEnvelope(
            generatedAt: generatedAt,
            providers: providers,
            nextReset: nextReset(from: snapshots, generatedAt: generatedAt),
            codexResetCredits: resetCredits(from: snapshots, generatedAt: generatedAt)
        )
    }

    private static func providerSnapshot(
        from snapshot: UsageSnapshot,
        generatedAt: Date
    ) -> WidgetProviderSnapshot {
        let presentation = ProviderPresentation(snapshot: snapshot)
        let semantic: WidgetSnapshotSemantic
        if snapshot.isStale(at: generatedAt), presentation.semantic != .unavailable {
            semantic = .stale
        } else {
            semantic = WidgetSnapshotSemantic(presentation.semantic)
        }
        return WidgetProviderSnapshot(
            provider: WidgetProvider(snapshot.provider),
            valueText: SensitiveTextRedactor.redact(presentation.valueText),
            detailText: SensitiveTextRedactor.redact(presentation.detailText),
            fraction: presentation.fraction,
            semantic: semantic,
            fetchedAt: snapshot.fetchedAt,
            expiresAt: snapshot.fetchedAt.addingTimeInterval(snapshot.staleAfter)
        )
    }

    private static func unavailable(_ provider: WidgetProvider) -> WidgetProviderSnapshot {
        WidgetProviderSnapshot(
            provider: provider,
            valueText: "Unavailable",
            detailText: "No current data",
            fraction: nil,
            semantic: .unavailable,
            fetchedAt: nil,
            expiresAt: nil
        )
    }

    private func nextReset(
        from snapshots: [UsageSnapshot],
        generatedAt: Date
    ) -> WidgetResetSummary? {
        let orderedSnapshots = [UsageProvider.claude, .codex].compactMap { provider in
            snapshots.first(where: { $0.provider == provider })
        }
        let candidates = orderedSnapshots.flatMap { snapshot in
            [snapshot.primaryMetric, snapshot.secondaryMetric].compactMap { metric -> ResetCandidate? in
                guard let metric else { return nil }
                return ResetCandidate(
                    provider: WidgetProvider(snapshot.provider),
                    label: SensitiveTextRedactor.redact(metric.label),
                    text: metric.resetDescription.map(SensitiveTextRedactor.redact),
                    resetAt: metric.resetAt
                )
            }
        }

        if let dated = candidates
            .filter({ ($0.resetAt ?? .distantPast) > generatedAt })
            .min(by: { ($0.resetAt ?? .distantFuture) < ($1.resetAt ?? .distantFuture) }) {
            return dated.summary
        }
        return candidates.first(where: { !($0.text ?? "").isEmpty })?.summary
    }

    private func resetCredits(
        from snapshots: [UsageSnapshot],
        generatedAt: Date
    ) -> WidgetResetCreditsSummary? {
        guard let summary = snapshots
            .first(where: { $0.provider == .codex })?
            .codexResetCredits else {
            return nil
        }
        let nearestExpiration = summary.credits
            .compactMap(\.expiresAt)
            .filter { $0 >= generatedAt }
            .min()
        return WidgetResetCreditsSummary(
            availableCount: summary.availableCount,
            nearestExpiration: nearestExpiration
        )
    }
}

private struct ResetCandidate {
    let provider: WidgetProvider
    let label: String
    let text: String?
    let resetAt: Date?

    var summary: WidgetResetSummary {
        WidgetResetSummary(
            provider: provider,
            label: label,
            text: text ?? resetAt?.formatted(date: .abbreviated, time: .shortened) ?? "Reset time unavailable",
            resetAt: resetAt
        )
    }
}

private extension WidgetProvider {
    init(_ provider: UsageProvider) {
        switch provider {
        case .claude: self = .claude
        case .codex: self = .codex
        case .deepSeek: self = .deepSeek
        }
    }
}

private extension UsageProvider {
    init?(widgetProvider: WidgetProvider) {
        switch widgetProvider {
        case .claude: self = .claude
        case .codex: self = .codex
        case .deepSeek: self = .deepSeek
        }
    }
}

private extension WidgetSnapshotSemantic {
    init(_ semantic: UsageSemantic) {
        switch semantic {
        case .normal: self = .normal
        case .warning: self = .warning
        case .critical: self = .critical
        case .stale: self = .stale
        case .unavailable: self = .unavailable
        }
    }
}
