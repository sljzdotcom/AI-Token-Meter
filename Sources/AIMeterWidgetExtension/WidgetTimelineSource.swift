import AIMeterCore
import Foundation
import WidgetKit

struct WidgetEntry: TimelineEntry {
    let date: Date
    let envelope: WidgetSnapshotEnvelope
}

struct WidgetTimelineSource: TimelineProvider {
    private let load: () -> WidgetSnapshotEnvelope?
    private let now: () -> Date
    private let refreshInterval: TimeInterval

    init(
        load: @escaping () -> WidgetSnapshotEnvelope?,
        now: @escaping () -> Date = Date.init,
        refreshInterval: TimeInterval = 30 * 60
    ) {
        self.load = load
        self.now = now
        self.refreshInterval = refreshInterval
    }

    init(bundle: Bundle = .main, fileManager: FileManager = .default) {
        let identifier = bundle.object(
            forInfoDictionaryKey: AITokenMeterWidgetContract.appGroupInfoKey
        ) as? String
        let store = identifier.flatMap {
            WidgetSnapshotStore(appGroupIdentifier: $0, fileManager: fileManager)
        }
        self.init(load: { try? store?.load() })
    }

    func placeholder(in context: Context) -> WidgetEntry {
        WidgetEntry(date: now(), envelope: Self.previewEnvelope(at: now()))
    }

    func getSnapshot(in context: Context, completion: @escaping (WidgetEntry) -> Void) {
        completion(context.isPreview ? placeholder(in: context) : currentEntry())
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<WidgetEntry>) -> Void) {
        let entry = context.isPreview ? placeholder(in: context) : currentEntry()
        completion(Timeline(
            entries: [entry],
            policy: .after(entry.date.addingTimeInterval(refreshInterval))
        ))
    }

    func currentEntry() -> WidgetEntry {
        let date = now()
        let envelope = load() ?? Self.emptyEnvelope(at: date)
        return WidgetEntry(date: date, envelope: Self.normalizedEnvelope(envelope, at: date))
    }

    static func emptyEnvelope(at date: Date) -> WidgetSnapshotEnvelope {
        WidgetSnapshotEnvelope(
            generatedAt: date,
            providers: WidgetProvider.allCases.map { provider in
                WidgetProviderSnapshot(
                    provider: provider,
                    valueText: "Unavailable",
                    detailText: "Open AI Token Meter to refresh",
                    fraction: nil,
                    semantic: .unavailable,
                    fetchedAt: nil,
                    expiresAt: nil
                )
            },
            nextReset: nil,
            codexResetCredits: nil
        )
    }

    static func previewEnvelope(at date: Date) -> WidgetSnapshotEnvelope {
        WidgetSnapshotEnvelope(
            generatedAt: date,
            providers: [
                WidgetProviderSnapshot(
                    provider: .claude,
                    valueText: "18%",
                    detailText: "Current session",
                    fraction: 0.18,
                    semantic: .normal,
                    fetchedAt: date,
                    expiresAt: date.addingTimeInterval(300)
                ),
                WidgetProviderSnapshot(
                    provider: .codex,
                    valueText: "31%",
                    detailText: "Weekly quota",
                    fraction: 0.31,
                    semantic: .normal,
                    fetchedAt: date,
                    expiresAt: date.addingTimeInterval(300)
                ),
                WidgetProviderSnapshot(
                    provider: .deepSeek,
                    valueText: "¥77.99",
                    detailText: "Available balance",
                    fraction: 0.2201,
                    semantic: .normal,
                    fetchedAt: date,
                    expiresAt: date.addingTimeInterval(300)
                ),
            ],
            nextReset: WidgetResetSummary(
                provider: .claude,
                label: "Current session",
                text: "In 3 hr 42 min",
                resetAt: date.addingTimeInterval(13_320)
            ),
            codexResetCredits: WidgetResetCreditsSummary(
                availableCount: 1,
                nearestExpiration: date.addingTimeInterval(21 * 86_400)
            )
        )
    }

    private static func markExpiredProvidersStale(
        _ providers: [WidgetProviderSnapshot],
        at date: Date
    ) -> [WidgetProviderSnapshot] {
        providers.map { provider in
            guard provider.semantic != .unavailable,
                  let expiresAt = provider.expiresAt,
                  expiresAt <= date else {
                return provider
            }
            return WidgetProviderSnapshot(
                provider: provider.provider,
                valueText: provider.valueText,
                detailText: provider.detailText,
                fraction: provider.fraction,
                semantic: .stale,
                fetchedAt: provider.fetchedAt,
                expiresAt: provider.expiresAt
            )
        }
    }

    private static func normalizedEnvelope(
        _ envelope: WidgetSnapshotEnvelope,
        at date: Date
    ) -> WidgetSnapshotEnvelope {
        let byProvider = Dictionary(
            envelope.providers.map { ($0.provider, $0) },
            uniquingKeysWith: { _, latest in latest }
        )
        let providers = WidgetProvider.allCases.map { provider in
            byProvider[provider] ?? unavailableProvider(provider)
        }
        let nextReset = envelope.nextReset.flatMap { summary in
            guard let resetAt = summary.resetAt else { return summary }
            return resetAt > date ? summary : nil
        }
        let resetCredits = envelope.codexResetCredits.map { summary in
            WidgetResetCreditsSummary(
                availableCount: summary.availableCount,
                nearestExpiration: summary.nearestExpiration.flatMap { $0 > date ? $0 : nil }
            )
        }
        return WidgetSnapshotEnvelope(
            version: envelope.version,
            generatedAt: envelope.generatedAt,
            providers: markExpiredProvidersStale(providers, at: date),
            nextReset: nextReset,
            codexResetCredits: resetCredits
        )
    }

    private static func unavailableProvider(_ provider: WidgetProvider) -> WidgetProviderSnapshot {
        WidgetProviderSnapshot(
            provider: provider,
            valueText: "Unavailable",
            detailText: "Open AI Token Meter to refresh",
            fraction: nil,
            semantic: .unavailable,
            fetchedAt: nil,
            expiresAt: nil
        )
    }
}
