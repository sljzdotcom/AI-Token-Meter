import Foundation

public actor RefreshCoordinator {
    private let collectors: [any UsageCollector]
    private let cache: SnapshotCache
    private var inFlight: Task<[UsageSnapshot], Never>?
    private let backoffURL: URL
    private var backoffs: [UsageProvider: RefreshBackoffState]
    private var lastPresented: [UsageProvider: UsageSnapshot] = [:]

    public init(
        collectors: [any UsageCollector],
        cache: SnapshotCache,
        backoffURL: URL? = nil
    ) {
        self.collectors = collectors
        self.cache = cache
        let url = backoffURL ?? cache.fileURL.deletingLastPathComponent().appendingPathComponent("refresh-backoff.json")
        self.backoffURL = url
        self.backoffs = (try? Data(contentsOf: url)).flatMap { try? JSONDecoder().decode([UsageProvider: RefreshBackoffState].self, from: $0) } ?? [:]
    }

    public func refresh(manual: Bool = true,
                        onOperation: @escaping @Sendable (UsageProvider, Bool) async -> Void = { _, _ in }) async -> [UsageSnapshot] {
        if let inFlight {
            return await inFlight.value
        }

        let now = Date().timeIntervalSince1970
        for provider in Array(backoffs.keys) { backoffs[provider]?.normalizeClock(now: now) }
        saveBackoffs()
        let eligible = collectors.filter { backoffs[$0.provider]?.isEligible(now: now, manual: manual) ?? true }
        let task = Task {
            await self.performRefresh(collectors: eligible, cache: self.cache, onOperation: onOperation)
        }
        inFlight = task
        let refreshed = await withTaskCancellationHandler {
            await task.value
        } onCancel: {
            task.cancel()
        }
        let cacheByProvider = Dictionary(((try? cache.load()) ?? []).map { ($0.provider, $0) }, uniquingKeysWith: { first, _ in first })
        let snapshots = collectors.map { collector in
            if let snapshot = refreshed.first(where: { $0.provider == collector.provider }) { return snapshot }
            if let cached = cacheByProvider[collector.provider] {
                return Self.cachedPresentation(from: cached, message: lastPresented[collector.provider]?.statusMessage ?? "Waiting before next refresh")
            }
            return lastPresented[collector.provider] ?? Self.failurePresentation(for: collector.provider, error:
                backoffs[collector.provider]?.failureKind == .authentication ? .authenticationRequired : .rateLimited)
        }
        lastPresented = Dictionary(snapshots.map { ($0.provider, $0) }, uniquingKeysWith: { first, _ in first })
        inFlight = nil
        return Self.sorted(snapshots)
    }

    public func clearAuthenticationBackoff(for provider: UsageProvider) {
        guard backoffs[provider]?.failureKind == .authentication else { return }
        backoffs.removeValue(forKey: provider)
        saveBackoffs()
    }

    public func providersRequiringAction() -> Set<UsageProvider> {
        Set(backoffs.filter { $0.value.failureKind == .authentication }.map(\.key))
    }

    private func saveBackoffs() {
        do {
            try FileManager.default.createDirectory(at: backoffURL.deletingLastPathComponent(), withIntermediateDirectories: true)
            try JSONEncoder().encode(backoffs).write(to: backoffURL, options: .atomic)
        } catch { NSLog("AI Token Meter: refresh retry state could not be saved") }
    }

    private func performRefresh(
        collectors: [any UsageCollector],
        cache: SnapshotCache,
        onOperation: @escaping @Sendable (UsageProvider, Bool) async -> Void
    ) async -> [UsageSnapshot] {
        let cachedSnapshots = (try? cache.load()) ?? []
        let cachedByProvider = Dictionary(
            cachedSnapshots.map { ($0.provider, $0) },
            uniquingKeysWith: { first, _ in first }
        )

        let outcomes = await withTaskGroup(of: RefreshOutcome.self) { group in
            for collector in collectors {
                group.addTask {
                    await onOperation(collector.provider, true)
                    do {
                        let value = try await collector.collect()
                        await onOperation(collector.provider, false)
                        return RefreshOutcome(
                            provider: collector.provider,
                            result: .success(value)
                        )
                    } catch let error as UsageCollectionError {
                        await onOperation(collector.provider, false)
                        return RefreshOutcome(provider: collector.provider, result: .failure(error))
                    } catch {
                        await onOperation(collector.provider, false)
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

        // A cancelled pass may have cooperative or non-cooperative collectors. Neither
        // is allowed to replace the cache or create retry penalties after cancellation.
        guard !Task.isCancelled else { return [] }

        var lastGoodByProvider = cachedByProvider
        var presented: [UsageSnapshot] = []

        for outcome in outcomes {
            switch outcome.result {
            case let .success(snapshot):
                backoffs.removeValue(forKey: outcome.provider)
                lastGoodByProvider[outcome.provider] = snapshot
                presented.append(snapshot)
            case let .failure(error):
                if !Task.isCancelled {
                    var backoff = backoffs[outcome.provider] ?? RefreshBackoffState()
                    let kind: RefreshFailureKind
                    var delay: TimeInterval = 0
                    switch error {
                    case .rateLimited: kind = .rateLimited
                    case .rateLimitedRetryAfter(let seconds): kind = .rateLimited; delay = seconds
                    case .authenticationRequired, .setupRequired, .notInstalled: kind = .authentication
                    default: kind = outcome.provider == .deepSeek ? .network : .cli
                    }
                    backoff.record(kind, now: Date().timeIntervalSince1970, retryAfter: delay)
                    backoffs[outcome.provider] = backoff
                }
                let failure = Self.failurePresentation(for: outcome.provider, error: error)
                if let cached = cachedByProvider[outcome.provider] {
                    presented.append(Self.cachedPresentation(from: cached, message: failure.statusMessage))
                } else {
                    presented.append(failure)
                }
            }
        }

        if outcomes.contains(where: { if case .success = $0.result { true } else { false } }) {
            try? cache.save(Self.sorted(Array(lastGoodByProvider.values)))
        }
        saveBackoffs()
        return Self.sorted(presented)
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
            codexLocalActivity: snapshot.codexLocalActivity,
            claudeLocalActivity: snapshot.claudeLocalActivity,
            geminiQuotaMetrics: snapshot.geminiQuotaMetrics,
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
        case .geminiUnavailable(let reason):
            status = .unavailable
            message = reason
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
        case .rateLimited, .rateLimitedRetryAfter:
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
