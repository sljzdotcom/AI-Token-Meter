import Foundation

public enum GeminiOneTimeQuotaError: Error, Equatable, Sendable {
    case notPaused
    case unauthorized
    case alreadyRunning
    case collectorUnavailable
}

/// Single-use capability issued only after the registered interactive login succeeds.
public struct GeminiQuotaRecoveryAuthorization: Equatable, Sendable {
    let id: UUID

    init(id: UUID) { self.id = id }
}

/// In-memory capability for a separate, explicit automatic-refresh resume action.
public struct GeminiAutomaticRefreshResumeAuthorization: Equatable, Sendable {
    let id: UUID
    /// Display hint only. The coordinator's stored deadline remains authoritative.
    public let expiresAt: ContinuousClock.Instant

    init(id: UUID, expiresAt: ContinuousClock.Instant = ContinuousClock.now.advanced(by: .seconds(300))) {
        self.id = id
        self.expiresAt = expiresAt
    }
}

public enum GeminiAutomaticRefreshResumeError: Error, Equatable, Sendable {
    case validationRequired
    case alreadyRunning
    case persistenceFailed
    case notPaused
}

public actor RefreshCoordinator {
    private let collectors: [any UsageCollector]
    private let cache: SnapshotCache
    private var inFlight: Task<[UsageSnapshot], Never>?
    private let backoffURL: URL
    private var backoffs: [UsageProvider: RefreshBackoffState]
    private var lastPresented: [UsageProvider: UsageSnapshot] = [:]
    private var isGeminiQuotaOnceRunning = false
    private var pendingGeminiLoginToken: String?
    private var pendingGeminiQuotaAuthorization: UUID?
    private let geminiResumeClock: @Sendable () -> ContinuousClock.Instant
    private var geminiRecoveryEpoch = UUID()
    private var geminiResumePermit: GeminiResumePermit?

    private struct GeminiResumePermit {
        let authorization: GeminiAutomaticRefreshResumeAuthorization
        let quotaAuthorizationID: UUID
        let epoch: UUID
        let issuedAt: ContinuousClock.Instant
        let snapshot: UsageSnapshot
    }

    public init(
        collectors: [any UsageCollector],
        cache: SnapshotCache,
        backoffURL: URL? = nil,
        geminiResumeClock: @escaping @Sendable () -> ContinuousClock.Instant = { ContinuousClock.now }
    ) {
        self.collectors = collectors
        self.cache = cache
        self.geminiResumeClock = geminiResumeClock
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
        if let gemini = backoffs[.gemini],
           [.authentication, .suspended].contains(gemini.failureKind),
           gemini.pauseReason == nil {
            var migrated = gemini
            migrated.record(.suspended, now: now, pauseReason: .unknown)
            backoffs[.gemini] = migrated
        }
        saveBackoffs()
        let eligible = collectors.filter { backoffs[$0.provider]?.isEligible(now: now, manual: manual) ?? true }
        let task = Task {
            await self.performRefresh(collectors: eligible, cache: self.cache, manual: manual, onOperation: onOperation)
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
                let fallback = backoffs[collector.provider]?.failureKind == .suspended
                    ? Self.pauseStatusMessage(backoffs[collector.provider]?.pauseReason ?? .unknown)
                    : "Waiting before next refresh"
                return Self.cachedPresentation(from: cached, message: lastPresented[collector.provider]?.statusMessage ?? fallback)
            }
            if let previous = lastPresented[collector.provider] { return previous }
            if backoffs[collector.provider]?.failureKind == .suspended {
                return Self.suspendedPresentation(for: collector.provider, reason: backoffs[collector.provider]?.pauseReason ?? .unknown)
            }
            return Self.failurePresentation(for: collector.provider, error:
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

    public func beginGeminiQuotaRecovery(token: String) -> Bool {
        guard backoffs[.gemini]?.failureKind == .suspended,
              UUID(uuidString: token) != nil,
              !isGeminiQuotaOnceRunning else { return false }
        invalidateGeminiAutomaticRefreshResumeAuthorization()
        pendingGeminiLoginToken = token
        pendingGeminiQuotaAuthorization = nil
        return true
    }

    public func authorizeGeminiQuotaRecoveryAfterSignIn(token: String) -> GeminiQuotaRecoveryAuthorization? {
        guard backoffs[.gemini]?.failureKind == .suspended,
              pendingGeminiLoginToken == token,
              UUID(uuidString: token) != nil else { return nil }
        pendingGeminiLoginToken = nil
        let authorization = UUID()
        pendingGeminiQuotaAuthorization = authorization
        return GeminiQuotaRecoveryAuthorization(id: authorization)
    }

    public func collectGeminiQuotaOnce(authorization: GeminiQuotaRecoveryAuthorization) async throws -> UsageSnapshot {
        guard !isGeminiQuotaOnceRunning else { throw GeminiOneTimeQuotaError.alreadyRunning }
        let epoch = geminiRecoveryEpoch
        do {
            guard backoffs[.gemini]?.failureKind == .suspended else {
                throw GeminiOneTimeQuotaError.notPaused
            }
            guard pendingGeminiQuotaAuthorization == authorization.id else {
                throw GeminiOneTimeQuotaError.unauthorized
            }
            guard let collector = collectors.first(where: { $0.provider == .gemini }) as? any OneTimeGeminiQuotaCollecting else {
                throw GeminiOneTimeQuotaError.collectorUnavailable
            }

            isGeminiQuotaOnceRunning = true
            pendingGeminiQuotaAuthorization = nil
            defer { isGeminiQuotaOnceRunning = false }
            if let inFlight { _ = await inFlight.value }
            try Task.checkCancellation()
            guard epoch == geminiRecoveryEpoch else { throw GeminiOneTimeQuotaError.unauthorized }
            guard backoffs[.gemini]?.failureKind == .suspended else {
                throw GeminiOneTimeQuotaError.notPaused
            }

            let snapshot = try await collector.collectQuotaOnce()
            try Task.checkCancellation()
            guard epoch == geminiRecoveryEpoch else { throw GeminiOneTimeQuotaError.unauthorized }
            guard snapshot.provider == .gemini, snapshot.geminiQuotaMetrics?.isEmpty == false else {
                throw UsageCollectionError.invalidResponse
            }
            try Task.checkCancellation()
            try cache.saveReplacing(snapshot)
            try Task.checkCancellation()
            guard epoch == geminiRecoveryEpoch else { throw GeminiOneTimeQuotaError.unauthorized }
            if snapshot.collectionStatus == .fresh,
               snapshot.normalizedAntigravityQuota().geminiQuotaMetrics?.isEmpty == false,
               let version = snapshot.sourceVersion,
               GeminiCollector.supports(version: version) {
                let now = geminiResumeClock()
                geminiResumePermit = GeminiResumePermit(
                    authorization: GeminiAutomaticRefreshResumeAuthorization(id: UUID(), expiresAt: now.advanced(by: .seconds(300))),
                    quotaAuthorizationID: authorization.id, epoch: epoch, issuedAt: now, snapshot: snapshot
                )
            }
            // A successful user-initiated read refreshes the cached quota only.
            // Automatic refresh requires a separate explicit, validated resume.
            return snapshot
        } catch {
            if epoch == geminiRecoveryEpoch { invalidateGeminiAutomaticRefreshResumeAuthorization() }
            throw error
        }
    }

    public func geminiAutomaticRefreshResumeAuthorization(
        for authorization: GeminiQuotaRecoveryAuthorization
    ) -> GeminiAutomaticRefreshResumeAuthorization? {
        guard let permit = currentGeminiResumePermit(),
              permit.quotaAuthorizationID == authorization.id else { return nil }
        return permit.authorization
    }

    public func resumeGeminiAutomaticRefresh(
        authorization: GeminiAutomaticRefreshResumeAuthorization
    ) throws -> UsageSnapshot {
        do { try Task.checkCancellation() } catch {
            invalidateGeminiAutomaticRefreshResumeAuthorization()
            throw error
        }
        guard let permit = currentGeminiResumePermit(), permit.authorization.id == authorization.id else {
            throw GeminiAutomaticRefreshResumeError.validationRequired
        }
        guard inFlight == nil, !isGeminiQuotaOnceRunning,
              pendingGeminiLoginToken == nil, pendingGeminiQuotaAuthorization == nil else {
            throw GeminiAutomaticRefreshResumeError.alreadyRunning
        }
        guard backoffs[.gemini]?.failureKind == .suspended else {
            invalidateGeminiAutomaticRefreshResumeAuthorization()
            throw GeminiAutomaticRefreshResumeError.notPaused
        }
        do { try Task.checkCancellation() } catch {
            invalidateGeminiAutomaticRefreshResumeAuthorization()
            throw error
        }
        geminiResumePermit = nil
        var candidate = backoffs
        candidate.removeValue(forKey: .gemini)
        guard persistBackoffs(candidate) else {
            throw GeminiAutomaticRefreshResumeError.persistenceFailed
        }
        backoffs = candidate
        lastPresented[.gemini] = permit.snapshot
        return permit.snapshot
    }

    public func invalidateGeminiAutomaticRefreshResumeAuthorization() {
        geminiRecoveryEpoch = UUID()
        geminiResumePermit = nil
        pendingGeminiLoginToken = nil
        pendingGeminiQuotaAuthorization = nil
    }

    private func currentGeminiResumePermit() -> GeminiResumePermit? {
        guard let permit = geminiResumePermit else { return nil }
        let now = geminiResumeClock()
        guard permit.epoch == geminiRecoveryEpoch,
              now >= permit.issuedAt, now < permit.authorization.expiresAt else {
            invalidateGeminiAutomaticRefreshResumeAuthorization()
            return nil
        }
        return permit
    }

    public func providersRequiringAction() -> Set<UsageProvider> {
        Set(backoffs.filter { [.authentication, .suspended].contains($0.value.failureKind) }.map(\.key))
    }

    public func geminiPauseReason() -> GeminiPauseReason? {
        guard let backoff = backoffs[.gemini],
              [.authentication, .suspended].contains(backoff.failureKind) else { return nil }
        return backoff.pauseReason ?? .unknown
    }

    private func saveBackoffs() {
        _ = persistBackoffs(backoffs)
    }

    private func persistBackoffs(_ state: [UsageProvider: RefreshBackoffState]) -> Bool {
        do {
            try FileManager.default.createDirectory(at: backoffURL.deletingLastPathComponent(), withIntermediateDirectories: true)
            try JSONEncoder().encode(state).write(to: backoffURL, options: .atomic)
            return true
        } catch {
            NSLog("AI Token Meter: refresh retry state could not be saved")
            return false
        }
    }

    private func performRefresh(
        collectors: [any UsageCollector],
        cache: SnapshotCache,
        manual: Bool,
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
                        let value: UsageSnapshot
                        if collector.provider == .gemini && !manual {
                            guard let quotaCollector = collector as? any OneTimeGeminiQuotaCollecting else {
                                throw UsageCollectionError.invalidResponse
                            }
                            value = try await quotaCollector.collectQuotaOnce()
                        } else {
                            value = try await collector.collect()
                        }
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

        // A cancelled Antigravity pass must remain paused after restart so app shutdown
        // cannot cause another headless CLI launch. Other providers keep their existing
        // cancellation behavior and do not receive a retry penalty.
        guard !Task.isCancelled else {
            if collectors.contains(where: { $0.provider == .gemini }) {
                invalidateGeminiAutomaticRefreshResumeAuthorization()
                var backoff = backoffs[.gemini] ?? RefreshBackoffState()
                backoff.record(.suspended, now: Date().timeIntervalSince1970, pauseReason: .cancelled)
                backoffs[.gemini] = backoff
                saveBackoffs()
            }
            return []
        }

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
                    if outcome.provider == .gemini {
                        invalidateGeminiAutomaticRefreshResumeAuthorization()
                        kind = .suspended
                        backoff.record(kind, now: Date().timeIntervalSince1970, pauseReason: Self.pauseReason(for: error))
                    } else {
                        switch error {
                        case .rateLimited: kind = .rateLimited
                        case .rateLimitedRetryAfter(let seconds): kind = .rateLimited; delay = seconds
                        case .authenticationRequired, .setupRequired, .notInstalled: kind = .authentication
                        default: kind = outcome.provider == .deepSeek ? .network : .cli
                        }
                    }
                    if outcome.provider != .gemini {
                        backoff.record(kind, now: Date().timeIntervalSince1970, retryAfter: delay)
                    }
                    backoffs[outcome.provider] = backoff
                }
                let failure = Self.failurePresentation(for: outcome.provider, error: error)
                if let cached = cachedByProvider[outcome.provider] {
                    let message = outcome.provider == .gemini
                        ? Self.pauseStatusMessage(Self.pauseReason(for: error))
                        : failure.statusMessage
                    presented.append(Self.cachedPresentation(from: cached, message: message))
                } else {
                    presented.append(failure)
                }
            }
        }

        if outcomes.contains(where: { if case .success = $0.result { true } else { false } }) {
            // A one-time Antigravity quota read may finish while this refresh is
            // collecting other providers. Preserve that newer Gemini cache entry
            // rather than writing the snapshot captured at refresh start.
            let refreshedGemini = outcomes.contains { outcome in
                guard outcome.provider == .gemini else { return false }
                if case .success = outcome.result { return true }
                return false
            }
            if !refreshedGemini,
               let latestGemini = (try? cache.load())?.first(where: { $0.provider == .gemini }) {
                lastGoodByProvider[.gemini] = latestGemini
            }
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
            antigravitySharedQuotaMetrics: snapshot.antigravitySharedQuotaMetrics,
            antigravityCLIInfo: snapshot.antigravityCLIInfo,
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
        case .outputLimitExceeded:
            status = .unrecognizedOutput
            message = "Antigravity CLI output exceeded the safe diagnostic limit"
        case .unsupportedVersion:
            status = .unavailable
            message = "Antigravity CLI version is not supported (requires 1.1.28 or later in major version 1)"
        case .environmentRejected:
            status = .unavailable
            message = "Antigravity CLI environment uses an unsupported override"
        case .executableUnavailable:
            status = .unavailable
            message = "Antigravity CLI is not executable"
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

    private static func suspendedPresentation(for provider: UsageProvider, reason: GeminiPauseReason) -> UsageSnapshot {
        UsageSnapshot(
            provider: provider,
            availability: .unavailable,
            collectionStatus: .unavailable,
            statusMessage: pauseStatusMessage(reason)
        )
    }

    private static func pauseReason(for error: UsageCollectionError) -> GeminiPauseReason {
        switch error {
        case .authenticationRequired: .authenticationRequired
        case .timedOut: .timeout
        case .transportFailure: .networkOrProcessFailure
        case .notInstalled, .executableUnavailable: .notInstalled
        case .rateLimited, .rateLimitedRetryAfter: .rateLimited
        case .outputLimitExceeded: .outputTruncated
        case .invalidResponse: .invalidResponse
        case .unrecognizedOutput: .unrecognizedOutput
        case .unsupportedVersion: .unsupportedVersion
        case .environmentRejected: .environmentRejected
        case .geminiUnavailable, .setupRequired: .unknown
        }
    }

    private static func pauseStatusMessage(_ reason: GeminiPauseReason) -> String {
        reason.statusMessage
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
