import Foundation

public actor RefreshCoordinator {
    private let collectors: [any UsageCollector]
    private let cache: SnapshotCache
    private var inFlight: Task<[UsageSnapshot], Never>?

    public init(
        collectors: [any UsageCollector],
        cache: SnapshotCache
    ) {
        self.collectors = collectors
        self.cache = cache
    }

    public func refresh() async -> [UsageSnapshot] {
        if let inFlight {
            return await inFlight.value
        }

        let task = Task { [collectors, cache] in
            await Self.performRefresh(collectors: collectors, cache: cache)
        }
        inFlight = task
        let snapshots = await task.value
        inFlight = nil
        return snapshots
    }

    private static func performRefresh(
        collectors: [any UsageCollector],
        cache: SnapshotCache
    ) async -> [UsageSnapshot] {
        let cachedSnapshots = (try? cache.load()) ?? []
        let cachedByProvider = Dictionary(
            cachedSnapshots.map { ($0.provider, $0) },
            uniquingKeysWith: { first, _ in first }
        )

        let outcomes = await withTaskGroup(of: RefreshOutcome.self) { group in
            for collector in collectors {
                group.addTask {
                    do {
                        return RefreshOutcome(
                            provider: collector.provider,
                            result: .success(try await collector.collect())
                        )
                    } catch let error as UsageCollectionError {
                        return RefreshOutcome(provider: collector.provider, result: .failure(error))
                    } catch {
                        return RefreshOutcome(
                            provider: collector.provider,
                            result: .failure(.transportFailure)
                        )
                    }
                }
            }

            var collected: [RefreshOutcome] = []
            for await outcome in group {
                collected.append(outcome)
            }
            return collected
        }

        var lastGoodByProvider = cachedByProvider
        var presented: [UsageSnapshot] = []

        for outcome in outcomes {
            switch outcome.result {
            case let .success(snapshot):
                lastGoodByProvider[outcome.provider] = snapshot
                presented.append(snapshot)
            case let .failure(error):
                let failure = failurePresentation(for: outcome.provider, error: error)
                if let cached = cachedByProvider[outcome.provider] {
                    presented.append(cachedPresentation(from: cached, message: failure.statusMessage))
                } else {
                    presented.append(failure)
                }
            }
        }

        if outcomes.contains(where: { if case .success = $0.result { true } else { false } }) {
            try? cache.save(sorted(Array(lastGoodByProvider.values)))
        }
        return sorted(presented)
    }

    private static func cachedPresentation(
        from snapshot: UsageSnapshot,
        message: String?
    ) -> UsageSnapshot {
        UsageSnapshot(
            provider: snapshot.provider,
            primaryMetric: snapshot.primaryMetric,
            secondaryMetric: snapshot.secondaryMetric,
            availability: snapshot.availability,
            fetchedAt: snapshot.fetchedAt,
            staleAfter: snapshot.staleAfter,
            sourceVersion: snapshot.sourceVersion,
            collectionStatus: .cached,
            statusMessage: message,
            codexResetCredits: snapshot.codexResetCredits,
            deepSeekUsageHistory: snapshot.deepSeekUsageHistory
        )
    }

    private static func failurePresentation(
        for provider: UsageProvider,
        error: UsageCollectionError
    ) -> UsageSnapshot {
        let status: CollectionStatus
        let message: String
        switch error {
        case .notInstalled:
            status = .notInstalled
            message = "CLI not installed"
        case .authenticationRequired:
            status = .authenticationRequired
            message = "Sign in required"
        case .setupRequired:
            status = .setupRequired
            message = "Approve the private usage workspace once"
        case .unrecognizedOutput:
            status = .unrecognizedOutput
            message = "Usage format is not recognized"
        case .rateLimited:
            status = .unavailable
            message = "Rate limited; try again later"
        case .timedOut:
            status = .unavailable
            message = "Request timed out"
        case .transportFailure, .invalidResponse:
            status = .unavailable
            message = "Service temporarily unavailable"
        }
        return UsageSnapshot(
            provider: provider,
            availability: .unavailable,
            collectionStatus: status,
            statusMessage: message
        )
    }

    private static func sorted(_ snapshots: [UsageSnapshot]) -> [UsageSnapshot] {
        snapshots.sorted {
            let lhs = UsageProvider.allCases.firstIndex(of: $0.provider) ?? .max
            let rhs = UsageProvider.allCases.firstIndex(of: $1.provider) ?? .max
            return lhs < rhs
        }
    }
}

private struct RefreshOutcome: Sendable {
    let provider: UsageProvider
    let result: Result<UsageSnapshot, UsageCollectionError>
}
