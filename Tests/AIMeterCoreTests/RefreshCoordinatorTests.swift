import Foundation
import Testing
@testable import AIMeterCore

@Suite("Refresh coordinator", .serialized)
struct RefreshCoordinatorTests {
    @Test func cancelledRefreshDoesNotCreateRetryPenalty() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = ControlledCollector(provider: .deepSeek, delay: 0.05, result: .failure(.rateLimited))
        let coordinator = RefreshCoordinator(collectors: [collector], cache: SnapshotCache(directoryURL: directory))
        let task = Task { await coordinator.refresh() }
        while collector.callCount == 0 { await Task.yield() }
        task.cancel()
        _ = await task.value
        _ = await coordinator.refresh(manual: false)
        #expect(collector.callCount == 2)
    }

    @Test("Cancelled Antigravity refresh stays suspended across restart")
    func cancelledGeminiRefreshSuspendsUntilExplicitSignIn() async {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = ControlledCollector(provider: .gemini, delay: 5, result: .failure(.timedOut))
        let cache = SnapshotCache(directoryURL: directory)
        let path = directory.appendingPathComponent("backoff.json")
        let first = RefreshCoordinator(collectors: [collector], cache: cache, backoffURL: path)
        let task = Task { await first.refresh() }
        while collector.callCount == 0 { await Task.yield() }
        task.cancel()
        _ = await task.value

        let restarted = RefreshCoordinator(collectors: [collector], cache: cache, backoffURL: path)
        _ = await restarted.refresh(manual: true)
        _ = await restarted.refresh(manual: false)

        #expect(collector.callCount == 1)
        #expect(await restarted.providersRequiringAction().contains(.gemini))
    }
    @Test("Rate limited provider is not recollected after a coordinator restart")
    func persistedBackoffSkipsOnlyFailedProvider() async {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let limited = ControlledCollector(provider: .deepSeek, result: .failure(.rateLimited))
        let healthy = ControlledCollector(provider: .claude, result: .success(snapshot(for: .claude)))
        let path = directory.appendingPathComponent("backoff.json")
        let first = RefreshCoordinator(collectors: [limited, healthy], cache: SnapshotCache(directoryURL: directory), backoffURL: path)
        _ = await first.refresh()
        let second = RefreshCoordinator(collectors: [limited, healthy], cache: SnapshotCache(directoryURL: directory), backoffURL: path)
        let values = await second.refresh()
        #expect(limited.callCount == 1)
        #expect(healthy.callCount == 2)
        #expect(values.count == 2)
    }

    @Test("Antigravity stays suspended across restart and manual refresh")
    func geminiFailureSuspendsUntilExplicitSignIn() async {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = ControlledCollector(provider: .gemini, result: .failure(.timedOut))
        let cache = SnapshotCache(directoryURL: directory)
        let path = directory.appendingPathComponent("backoff.json")
        let first = RefreshCoordinator(collectors: [collector], cache: cache, backoffURL: path)

        _ = await first.refresh(manual: true)
        let restarted = RefreshCoordinator(collectors: [collector], cache: cache, backoffURL: path)
        _ = await restarted.refresh(manual: true)
        _ = await restarted.refresh(manual: false)

        #expect(collector.callCount == 1)
        #expect(await restarted.providersRequiringAction().contains(.gemini))
        #expect(await restarted.geminiPauseReason() == .timeout)

        _ = await restarted.refresh(manual: false)
        _ = await restarted.refresh(manual: true)
        #expect(collector.callCount == 1)
    }

    @Test("A headless authentication failure stays paused across restart")
    func headlessAuthenticationFailureNeverRetriesInteractively() async {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = ControlledCollector(provider: .gemini, result: .failure(.authenticationRequired))
        let cache = SnapshotCache(directoryURL: directory)
        let path = directory.appendingPathComponent("backoff.json")
        let first = RefreshCoordinator(collectors: [collector], cache: cache, backoffURL: path)

        _ = await first.refresh(manual: false)

        let restarted = RefreshCoordinator(collectors: [collector], cache: cache, backoffURL: path)
        _ = await restarted.refresh(manual: true)

        #expect(collector.callCount == 1)
        #expect(await restarted.geminiPauseReason() == .authenticationRequired)
    }

    @Test("One-time Gemini quota success updates cache but keeps persistent suspension")
    func oneTimeGeminiCheckKeepsPauseAndOtherProviderCache() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let cache = SnapshotCache(directoryURL: directory)
        let backoffURL = directory.appendingPathComponent("backoff.json")
        let claudeCache = UsageSnapshot(
            provider: .claude,
            primaryMetric: metric(value: 42),
            fetchedAt: Date(timeIntervalSince1970: 400)
        )
        try cache.save([claudeCache])
        let quotaCollector = OneTimeGeminiCollector()
        let collectors: [any UsageCollector] = [quotaCollector]
        let initial = RefreshCoordinator(collectors: collectors, cache: cache, backoffURL: backoffURL)

        _ = await initial.refresh()
        await #expect(throws: GeminiOneTimeQuotaError.unauthorized) {
            try await initial.collectGeminiQuotaOnce(authorization: GeminiQuotaRecoveryAuthorization(id: UUID()))
        }
        #expect(await initial.geminiPauseReason() == .timeout)
        let authorization = try await authorizeRecovery(on: initial)
        let recovered = try await initial.collectGeminiQuotaOnce(authorization: authorization)

        #expect(recovered.geminiQuotaMetrics?.first?.current == 77)
        #expect(await initial.geminiPauseReason() == .timeout)
        let saved = try cache.load()
        #expect(saved.first(where: { $0.provider == .claude })?.fetchedAt == claudeCache.fetchedAt)
        #expect(saved.first(where: { $0.provider == .gemini })?.fetchedAt == Date(timeIntervalSince1970: 800))
        #expect(saved.first(where: { $0.provider == .gemini })?.antigravitySharedQuotaMetrics?.map(\.label) == [
            "Claude/GPT · Five hour", "Claude/GPT · Weekly",
        ])

        let restarted = RefreshCoordinator(collectors: collectors, cache: cache, backoffURL: backoffURL)
        _ = await restarted.refresh(manual: false)
        #expect(await restarted.geminiPauseReason() == .timeout)
        #expect(await restarted.providersRequiringAction().contains(.gemini))
        #expect(quotaCollector.normalCalls == 1)
        #expect(quotaCollector.oneTimeCalls == 1)
    }

    @Test("Fresh supported quota requires explicit resume before automatic refresh becomes durable")
    func freshQuotaAllowsExplicitDurableResumeWithoutClearingOtherProviderBackoff() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let cache = SnapshotCache(directoryURL: directory)
        let backoffURL = directory.appendingPathComponent("backoff.json")
        let gemini = ExplicitResumeGeminiCollector()
        let other = ControlledCollector(provider: .deepSeek, result: .failure(.authenticationRequired))
        let collectors: [any UsageCollector] = [gemini, other]
        let coordinator = RefreshCoordinator(collectors: collectors, cache: cache, backoffURL: backoffURL)

        _ = await coordinator.refresh(manual: true)
        let originalBackoffs = try JSONDecoder().decode(
            [UsageProvider: RefreshBackoffState].self,
            from: Data(contentsOf: backoffURL)
        )
        let encoder = JSONEncoder()
        encoder.outputFormatting = .sortedKeys
        let otherBefore = try encoder.encode(#require(originalBackoffs[.deepSeek]))
        let quotaAuthorization = try await authorizeRecovery(on: coordinator)
        #expect(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: quotaAuthorization) == nil)
        let recovered = try await coordinator.collectGeminiQuotaOnce(authorization: quotaAuthorization)

        #expect(recovered.collectionStatus == .fresh)
        #expect(recovered.sourceVersion == "1.3.1")
        #expect(await coordinator.geminiPauseReason() == .timeout)
        #expect(await coordinator.providersRequiringAction() == [.gemini, .deepSeek])
        let beforeResumeRestart = RefreshCoordinator(collectors: collectors, cache: cache, backoffURL: backoffURL)
        #expect(await beforeResumeRestart.providersRequiringAction() == [.gemini, .deepSeek])
        #expect(await beforeResumeRestart.geminiAutomaticRefreshResumeAuthorization(for: quotaAuthorization) == nil)

        let permit = try #require(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: quotaAuthorization))
        let resumed = try await coordinator.resumeGeminiAutomaticRefresh(authorization: permit)

        #expect(resumed == recovered)
        #expect(await coordinator.geminiPauseReason() == nil)
        #expect(await coordinator.providersRequiringAction() == [.deepSeek])
        #expect(gemini.normalCalls == 1)
        #expect(gemini.quotaCalls == 1)
        #expect(other.callCount == 1)
        #expect(try cache.load().first(where: { $0.provider == .gemini }) == recovered)
        let resumedBackoffs = try JSONDecoder().decode(
            [UsageProvider: RefreshBackoffState].self,
            from: Data(contentsOf: backoffURL)
        )
        #expect(resumedBackoffs[.gemini] == nil)
        #expect(try encoder.encode(#require(resumedBackoffs[.deepSeek])) == otherBefore)
        await #expect(throws: GeminiAutomaticRefreshResumeError.validationRequired) {
            try await coordinator.resumeGeminiAutomaticRefresh(authorization: permit)
        }

        let restarted = RefreshCoordinator(collectors: collectors, cache: cache, backoffURL: backoffURL)
        #expect(await restarted.geminiAutomaticRefreshResumeAuthorization(for: quotaAuthorization) == nil)
        let automatic = await restarted.refresh(manual: false)
        #expect(automatic.first(where: { $0.provider == .gemini })?.collectionStatus == .fresh)
        #expect(await restarted.providersRequiringAction() == [.deepSeek])
        #expect(gemini.normalCalls + gemini.quotaCalls == 3)
        #expect(other.callCount == 1)
    }

    @Test("Resume without a validated quota capability preserves the persistent pause")
    func automaticResumeWithoutValidatedQuotaIsDenied() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let backoffURL = directory.appendingPathComponent("backoff.json")
        let collector = ExplicitResumeGeminiCollector()
        let coordinator = RefreshCoordinator(
            collectors: [collector], cache: SnapshotCache(directoryURL: directory), backoffURL: backoffURL
        )
        _ = await coordinator.refresh(manual: true)
        let persistedPause = try Data(contentsOf: backoffURL)

        await #expect(throws: GeminiAutomaticRefreshResumeError.validationRequired) {
            try await coordinator.resumeGeminiAutomaticRefresh(
                authorization: GeminiAutomaticRefreshResumeAuthorization(id: UUID())
            )
        }

        #expect(await coordinator.geminiPauseReason() == .timeout)
        #expect(await coordinator.providersRequiringAction() == [.gemini])
        #expect(try Data(contentsOf: backoffURL) == persistedPause)
        #expect(collector.normalCalls == 1)
        #expect(collector.quotaCalls == 0)
    }

    @Test("Resume capabilities match only their completed recovery and are revoked by a new session")
    func resumeCapabilityAssociationAndSessionInvalidation() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = ExplicitResumeGeminiCollector()
        let coordinator = RefreshCoordinator(collectors: [collector], cache: SnapshotCache(directoryURL: directory))
        _ = await coordinator.refresh()
        let first = try await authorizeRecovery(on: coordinator)
        _ = try await coordinator.collectGeminiQuotaOnce(authorization: first)
        let old = try #require(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: first))
        #expect(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: GeminiQuotaRecoveryAuthorization(id: UUID())) == nil)

        let second = try await authorizeRecovery(on: coordinator)
        await #expect(throws: GeminiAutomaticRefreshResumeError.validationRequired) {
            try await coordinator.resumeGeminiAutomaticRefresh(authorization: old)
        }
        #expect(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: first) == nil)
        _ = try await coordinator.collectGeminiQuotaOnce(authorization: second)
        let current = try #require(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: second))
        #expect(current.id != old.id)
        #expect(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: first) == nil)
        await coordinator.invalidateGeminiAutomaticRefreshResumeAuthorization()
        await #expect(throws: GeminiAutomaticRefreshResumeError.validationRequired) {
            try await coordinator.resumeGeminiAutomaticRefresh(authorization: current)
        }
        #expect(await coordinator.geminiPauseReason() == .timeout)
    }

    @Test("The five-minute continuous deadline and backwards clock deny resume", arguments: [299, 300, 301, -1])
    func resumeCapabilityContinuousDeadline(_ seconds: Int) async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let clock = ResumeTestClock()
        let collector = ExplicitResumeGeminiCollector()
        let coordinator = RefreshCoordinator(
            collectors: [collector], cache: SnapshotCache(directoryURL: directory), geminiResumeClock: { clock.now() }
        )
        _ = await coordinator.refresh()
        let auth = try await authorizeRecovery(on: coordinator)
        _ = try await coordinator.collectGeminiQuotaOnce(authorization: auth)
        let permit = try #require(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: auth))
        #expect(permit.expiresAt == clock.origin.advanced(by: .seconds(300)))
        clock.setOffset(.seconds(seconds))
        if seconds == 299 {
            #expect(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: auth) == permit)
            _ = try await coordinator.resumeGeminiAutomaticRefresh(authorization: permit)
            #expect(await coordinator.geminiPauseReason() == nil)
        } else {
            // A caller cannot extend the coordinator's deadline by copying its capability ID.
            let forgedDeadline = GeminiAutomaticRefreshResumeAuthorization(
                id: permit.id, expiresAt: clock.origin.advanced(by: .seconds(999))
            )
            await #expect(throws: GeminiAutomaticRefreshResumeError.validationRequired) {
                try await coordinator.resumeGeminiAutomaticRefresh(authorization: forgedDeadline)
            }
            #expect(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: auth) == nil)
            clock.setOffset(.seconds(1))
            await #expect(throws: GeminiAutomaticRefreshResumeError.validationRequired) {
                try await coordinator.resumeGeminiAutomaticRefresh(authorization: permit)
            }
            #expect(await coordinator.geminiPauseReason() == .timeout)
        }
        #expect(collector.normalCalls == 1)
        #expect(collector.quotaCalls == 1)
    }

    @Test("Cached, unsupported, missing-version, or invalid quotas never issue resume authority", arguments: ["cached", "unsupported", "missing", "wrong-provider", "empty", "malformed"])
    func unvalidatedQuotaCannotIssueResumeCapability(_ kind: String) async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = ExplicitResumeGeminiCollector()
        let cache = SnapshotCache(directoryURL: directory)
        // A supported old cache is not current recovery authority.
        try cache.save([resumeQuota()])
        let coordinator = RefreshCoordinator(collectors: [collector], cache: cache)
        _ = await coordinator.refresh()
        let authorization = try await authorizeRecovery(on: coordinator)
        #expect(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: authorization) == nil)
        collector.setQuotaResult(.success(resumeQuota(
            provider: kind == "wrong-provider" ? .claude : .gemini,
            status: kind == "cached" ? .cached : .fresh,
            version: kind == "unsupported" ? "2.0.0" : kind == "missing" ? nil : "1.3.1",
            metrics: kind != "empty", malformed: kind == "malformed"
        )))
        if kind == "wrong-provider" || kind == "empty" {
            await #expect(throws: UsageCollectionError.invalidResponse) {
                try await coordinator.collectGeminiQuotaOnce(authorization: authorization)
            }
        } else {
            _ = try await coordinator.collectGeminiQuotaOnce(authorization: authorization)
        }
        #expect(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: authorization) == nil)
        #expect(await coordinator.geminiPauseReason() == .timeout)
    }

    @Test("Resume persistence failure consumes authority while preserving pause and quota")
    func resumePersistenceFailureConsumesCapability() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let cache = SnapshotCache(directoryURL: directory)
        let stateDirectory = directory.appendingPathComponent("state")
        let backupDirectory = directory.appendingPathComponent("state-preserved")
        let backoffURL = stateDirectory.appendingPathComponent("backoff.json")
        let collector = ExplicitResumeGeminiCollector()
        let coordinator = RefreshCoordinator(collectors: [collector], cache: cache, backoffURL: backoffURL)
        _ = await coordinator.refresh()
        let auth = try await authorizeRecovery(on: coordinator)
        let quota = try await coordinator.collectGeminiQuotaOnce(authorization: auth)
        let permit = try #require(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: auth))
        let originalPause = try Data(contentsOf: backoffURL)
        // Block only this temporary persistence path, preserving the original file for restart.
        try FileManager.default.moveItem(at: stateDirectory, to: backupDirectory)
        try Data("occupied".utf8).write(to: stateDirectory)

        await #expect(throws: GeminiAutomaticRefreshResumeError.persistenceFailed) {
            try await coordinator.resumeGeminiAutomaticRefresh(authorization: permit)
        }
        #expect(await coordinator.geminiPauseReason() == .timeout)
        #expect(await coordinator.providersRequiringAction() == [.gemini])
        #expect(try cache.load().first == quota)
        #expect(try Data(contentsOf: backupDirectory.appendingPathComponent("backoff.json")) == originalPause)
        try FileManager.default.removeItem(at: stateDirectory)
        try FileManager.default.moveItem(at: backupDirectory, to: stateDirectory)
        await #expect(throws: GeminiAutomaticRefreshResumeError.validationRequired) {
            try await coordinator.resumeGeminiAutomaticRefresh(authorization: permit)
        }
        let restarted = RefreshCoordinator(collectors: [collector], cache: cache, backoffURL: backoffURL)
        #expect(await restarted.geminiPauseReason() == .timeout)
        #expect(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: auth) == nil)
        #expect(collector.normalCalls == 1)
        #expect(collector.quotaCalls == 1)
    }

    @Test("An entire concurrent refresh must finish before the same capability can be explicitly retried")
    func busyResumeRetainsCapabilityForExplicitRetryAfterFullRefresh() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let gemini = ExplicitResumeGeminiCollector()
        let gate = ResumeCollectionGate()
        let coordinator = RefreshCoordinator(
            collectors: [gemini, GatedResumeOtherCollector(gate: gate)], cache: SnapshotCache(directoryURL: directory)
        )
        _ = await coordinator.refresh()
        let auth = try await authorizeRecovery(on: coordinator)
        let quota = try await coordinator.collectGeminiQuotaOnce(authorization: auth)
        let permit = try #require(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: auth))
        await gate.block()
        let refresh = Task { await coordinator.refresh(manual: false) }
        let entered = await waitForGate(gate)
        #expect(entered)
        await #expect(throws: GeminiAutomaticRefreshResumeError.alreadyRunning) {
            try await coordinator.resumeGeminiAutomaticRefresh(authorization: permit)
        }
        #expect(await coordinator.geminiPauseReason() == .timeout)
        #expect(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: auth) == permit)
        #expect(gemini.normalCalls == 1)
        #expect(gemini.quotaCalls == 1)
        await gate.release()
        let fullResult = await refresh.value
        #expect(fullResult.first(where: { $0.provider == .gemini })?.collectionStatus == .cached)
        #expect(fullResult.first(where: { $0.provider == .gemini })?.fetchedAt == quota.fetchedAt)
        #expect(try await coordinator.resumeGeminiAutomaticRefresh(authorization: permit) == quota)
        #expect(await coordinator.geminiPauseReason() == nil)
        let next = await coordinator.refresh(manual: false)
        #expect(next.first(where: { $0.provider == .gemini })?.statusMessage == nil)
        #expect(gemini.normalCalls == 1)
        #expect(gemini.quotaCalls == 2)
    }

    @Test("Cancellation or epoch invalidation before entry denies resume; a committed choice remains durable")
    func resumeCancellationAndCommitLifecycleOrdering() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let cache = SnapshotCache(directoryURL: directory)
        let collector = ExplicitResumeGeminiCollector()
        let coordinator = RefreshCoordinator(collectors: [collector], cache: cache)
        _ = await coordinator.refresh()
        var auth = try await authorizeRecovery(on: coordinator)
        _ = try await coordinator.collectGeminiQuotaOnce(authorization: auth)
        var permit = try #require(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: auth))
        let cancelledPermit = permit
        let gate = ResumeCollectionGate()
        await gate.block()
        let pending = Task {
            await gate.waitIfBlocked()
            return try await coordinator.resumeGeminiAutomaticRefresh(authorization: cancelledPermit)
        }
        #expect(await waitForGate(gate))
        pending.cancel()
        await gate.release()
        await #expect(throws: CancellationError.self) { try await pending.value }
        #expect(await coordinator.geminiPauseReason() == .timeout)
        #expect(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: auth) == nil)

        auth = try await authorizeRecovery(on: coordinator)
        _ = try await coordinator.collectGeminiQuotaOnce(authorization: auth)
        permit = try #require(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: auth))
        await coordinator.invalidateGeminiAutomaticRefreshResumeAuthorization()
        await #expect(throws: GeminiAutomaticRefreshResumeError.validationRequired) {
            try await coordinator.resumeGeminiAutomaticRefresh(authorization: permit)
        }
        auth = try await authorizeRecovery(on: coordinator)
        _ = try await coordinator.collectGeminiQuotaOnce(authorization: auth)
        permit = try #require(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: auth))
        _ = try await coordinator.resumeGeminiAutomaticRefresh(authorization: permit)
        await coordinator.invalidateGeminiAutomaticRefreshResumeAuthorization()
        let restarted = RefreshCoordinator(collectors: [collector], cache: cache)
        #expect(await restarted.geminiPauseReason() == nil)
        #expect(await restarted.geminiAutomaticRefreshResumeAuthorization(for: auth) == nil)
    }

    @Test("An in-progress quota cannot commit a capability after cancellation or session invalidation", arguments: [false, true])
    func quotaCancellationAndInvalidationCannotIssueCapability(_ invalidate: Bool) async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let cache = SnapshotCache(directoryURL: directory)
        let collector = ExplicitResumeGeminiCollector()
        let coordinator = RefreshCoordinator(collectors: [collector], cache: cache)
        _ = await coordinator.refresh()
        let auth = try await authorizeRecovery(on: coordinator)
        await collector.gate.block()
        let quota = Task { try await coordinator.collectGeminiQuotaOnce(authorization: auth) }
        #expect(await waitForGate(collector.gate))
        if invalidate { await coordinator.invalidateGeminiAutomaticRefreshResumeAuthorization() }
        else { quota.cancel() }
        await collector.gate.release()
        if invalidate {
            await #expect(throws: GeminiOneTimeQuotaError.unauthorized) { try await quota.value }
        } else {
            await #expect(throws: CancellationError.self) { try await quota.value }
        }
        #expect(try cache.load().isEmpty)
        #expect(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: auth) == nil)
        #expect(await coordinator.geminiPauseReason() == .timeout)
    }

    @Test("A failed supported quota cache commit issues no resume capability")
    func failedSupportedQuotaCacheCommitCannotIssueCapability() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let blockedCache = directory.appendingPathComponent("occupied")
        try Data("occupied".utf8).write(to: blockedCache)
        let collector = ExplicitResumeGeminiCollector()
        let coordinator = RefreshCoordinator(
            collectors: [collector], cache: SnapshotCache(directoryURL: blockedCache),
            backoffURL: directory.appendingPathComponent("backoff.json")
        )
        _ = await coordinator.refresh()
        let authorization = try await authorizeRecovery(on: coordinator)
        await #expect(throws: (any Error).self) {
            try await coordinator.collectGeminiQuotaOnce(authorization: authorization)
        }
        #expect(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: authorization) == nil)
        #expect(await coordinator.geminiPauseReason() == .timeout)
    }

    @Test("Failed recovery and stop invalidate login and quota authorization without enabling refresh")
    func recoveryErrorsAndInvalidatedLoginCannotGrantResume() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = ExplicitResumeGeminiCollector()
        let coordinator = RefreshCoordinator(collectors: [collector], cache: SnapshotCache(directoryURL: directory))
        _ = await coordinator.refresh()
        let first = try await authorizeRecovery(on: coordinator)
        _ = try await coordinator.collectGeminiQuotaOnce(authorization: first)
        let permit = try #require(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: first))
        let token = UUID().uuidString
        #expect(await coordinator.beginGeminiQuotaRecovery(token: token))
        #expect(await coordinator.authorizeGeminiQuotaRecoveryAfterSignIn(token: "wrong") == nil)
        await coordinator.invalidateGeminiAutomaticRefreshResumeAuthorization()
        #expect(await coordinator.authorizeGeminiQuotaRecoveryAfterSignIn(token: token) == nil)
        await #expect(throws: GeminiAutomaticRefreshResumeError.validationRequired) {
            try await coordinator.resumeGeminiAutomaticRefresh(authorization: permit)
        }
        let stoppedQuota = try await authorizeRecovery(on: coordinator)
        await coordinator.invalidateGeminiAutomaticRefreshResumeAuthorization()
        await #expect(throws: GeminiOneTimeQuotaError.unauthorized) {
            try await coordinator.collectGeminiQuotaOnce(authorization: stoppedQuota)
        }
        let failure = try await authorizeRecovery(on: coordinator)
        collector.setQuotaResult(.failure(.transportFailure))
        await #expect(throws: UsageCollectionError.transportFailure) {
            try await coordinator.collectGeminiQuotaOnce(authorization: failure)
        }
        #expect(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: failure) == nil)
        #expect(await coordinator.geminiPauseReason() == .timeout)
        #expect(collector.quotaCalls == 2)
    }

    @Test("Cancelling the next automatic quota after resume persists a new cancellation pause")
    func cancelledAutomaticQuotaAfterResumePersistsPauseAgain() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = ExplicitResumeGeminiCollector()
        let cache = SnapshotCache(directoryURL: directory)
        let coordinator = RefreshCoordinator(collectors: [collector], cache: cache)
        _ = await coordinator.refresh()
        let auth = try await authorizeRecovery(on: coordinator)
        _ = try await coordinator.collectGeminiQuotaOnce(authorization: auth)
        let permit = try #require(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: auth))
        _ = try await coordinator.resumeGeminiAutomaticRefresh(authorization: permit)
        await collector.gate.block()
        let automatic = Task { await coordinator.refresh(manual: false) }
        #expect(await waitForGate(collector.gate))
        automatic.cancel()
        await collector.gate.release()
        _ = await automatic.value
        #expect(await coordinator.geminiPauseReason() == .cancelled)
        let restarted = RefreshCoordinator(collectors: [collector], cache: cache)
        #expect(await restarted.geminiPauseReason() == .cancelled)
        _ = await restarted.refresh(manual: false)
        #expect(collector.normalCalls == 1)
        #expect(collector.quotaCalls == 2)
    }

    @Test("Automatic Gemini refresh requires the quota-only protocol and never falls back to collect")
    func automaticGeminiWithoutQuotaProtocolFailsClosed() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = FullOnlyGeminiCollector()
        let cache = SnapshotCache(directoryURL: directory)
        let coordinator = RefreshCoordinator(collectors: [collector], cache: cache)
        _ = await coordinator.refresh(manual: false)
        #expect(collector.callCount == 0)
        #expect(await coordinator.geminiPauseReason() == .invalidResponse)
        let restarted = RefreshCoordinator(collectors: [collector], cache: cache)
        _ = await restarted.refresh(manual: false)
        #expect(collector.callCount == 0)
        #expect(await restarted.providersRequiringAction() == [.gemini])
        let manual = RefreshCoordinator(
            collectors: [collector], cache: cache, backoffURL: directory.appendingPathComponent("manual-backoff.json")
        )
        _ = await manual.refresh(manual: true)
        #expect(collector.callCount == 1)
    }

    @Test("Automatic quota errors after explicit resume suspend again and revoke the old capability", arguments: [UsageCollectionError.authenticationRequired, .transportFailure, .timedOut, .unsupportedVersion, .environmentRejected])
    func laterAutomaticFailurePersistsPauseAgain(_ error: UsageCollectionError) async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = ExplicitResumeGeminiCollector()
        let cache = SnapshotCache(directoryURL: directory)
        let coordinator = RefreshCoordinator(collectors: [collector], cache: cache)
        _ = await coordinator.refresh()
        let auth = try await authorizeRecovery(on: coordinator)
        _ = try await coordinator.collectGeminiQuotaOnce(authorization: auth)
        let permit = try #require(await coordinator.geminiAutomaticRefreshResumeAuthorization(for: auth))
        _ = try await coordinator.resumeGeminiAutomaticRefresh(authorization: permit)
        collector.setQuotaResult(.failure(error))
        _ = await coordinator.refresh(manual: false)
        #expect(await coordinator.providersRequiringAction() == [.gemini])
        await #expect(throws: GeminiAutomaticRefreshResumeError.validationRequired) {
            try await coordinator.resumeGeminiAutomaticRefresh(authorization: permit)
        }
        let restarted = RefreshCoordinator(collectors: [collector], cache: cache)
        _ = await restarted.refresh(manual: false)
        #expect(await restarted.providersRequiringAction() == [.gemini])
        #expect(collector.normalCalls == 1)
        #expect(collector.quotaCalls == 2)
    }

    @Test("A failed one-time quota check leaves every cached snapshot untouched")
    func failedOneTimeGeminiCheckPreservesCache() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let cache = SnapshotCache(directoryURL: directory)
        let oldQuota = UsageSnapshot(
            provider: .gemini,
            primaryMetric: metric(value: 35),
            fetchedAt: Date(timeIntervalSince1970: 300),
            collectionStatus: .fresh,
            geminiQuotaMetrics: [metric(value: 35)]
        )
        let oldClaude = UsageSnapshot(provider: .claude, primaryMetric: metric(value: 61))
        try cache.save([oldQuota, oldClaude])
        let collector = FailingOneTimeGeminiCollector()
        let coordinator = RefreshCoordinator(
            collectors: [collector],
            cache: cache,
            backoffURL: directory.appendingPathComponent("backoff.json")
        )
        _ = await coordinator.refresh()
        let authorization = try await authorizeRecovery(on: coordinator)

        await #expect(throws: UsageCollectionError.timedOut) {
            try await coordinator.collectGeminiQuotaOnce(authorization: authorization)
        }
        let after = try cache.load()
        #expect(after.first(where: { $0.provider == .gemini })?.fetchedAt == oldQuota.fetchedAt)
        #expect(after.first(where: { $0.provider == .claude })?.fetchedAt == oldClaude.fetchedAt)
        #expect(await coordinator.geminiPauseReason() == .timeout)
    }

    @Test("An invalid one-time Gemini response cannot clear the suspension")
    func invalidOneTimeGeminiResponseKeepsPause() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = InvalidOneTimeGeminiCollector()
        let coordinator = RefreshCoordinator(
            collectors: [collector],
            cache: SnapshotCache(directoryURL: directory),
            backoffURL: directory.appendingPathComponent("backoff.json")
        )
        _ = await coordinator.refresh(manual: false)
        let authorization = try await authorizeRecovery(on: coordinator)

        await #expect(throws: UsageCollectionError.invalidResponse) {
            try await coordinator.collectGeminiQuotaOnce(authorization: authorization)
        }

        #expect(await coordinator.geminiPauseReason() == .timeout)
        #expect(collector.oneTimeCalls == 1)
    }

    @Test("A failed one-time cache write keeps suspension in place")
    func failedOneTimeCacheWriteKeepsSuspension() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let blockedDirectory = directory.appendingPathComponent("not-a-directory")
        try Data("occupied".utf8).write(to: blockedDirectory)
        let collector = OneTimeGeminiCollector()
        let cache = SnapshotCache(directoryURL: blockedDirectory)
        let coordinator = RefreshCoordinator(
            collectors: [collector],
            cache: cache,
            backoffURL: directory.appendingPathComponent("backoff.json")
        )

        _ = await coordinator.refresh(manual: false)
        let authorization = try await authorizeRecovery(on: coordinator)
        await #expect(throws: (any Error).self) {
            try await coordinator.collectGeminiQuotaOnce(authorization: authorization)
        }

        #expect(await coordinator.geminiPauseReason() == .timeout)
    }

    @Test("Cancelling a one-time quota check preserves cache and suspension")
    func cancelledOneTimeGeminiCheckPreservesCacheAndPause() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let cache = SnapshotCache(directoryURL: directory)
        let previous = UsageSnapshot(
            provider: .gemini,
            primaryMetric: metric(value: 19),
            fetchedAt: Date(timeIntervalSince1970: 250),
            collectionStatus: .fresh,
            geminiQuotaMetrics: [metric(value: 19)]
        )
        try cache.save([previous])
        let collector = OneTimeGeminiCollector(oneTimeDelay: 5)
        let coordinator = RefreshCoordinator(
            collectors: [collector],
            cache: cache,
            backoffURL: directory.appendingPathComponent("backoff.json")
        )
        _ = await coordinator.refresh(manual: false)
        let authorization = try await authorizeRecovery(on: coordinator)

        let task = Task { try await coordinator.collectGeminiQuotaOnce(authorization: authorization) }
        while collector.oneTimeCalls == 0 { await Task.yield() }
        task.cancel()
        await #expect(throws: CancellationError.self) { try await task.value }

        #expect(try cache.load().first(where: { $0.provider == .gemini })?.fetchedAt == previous.fetchedAt)
        #expect(await coordinator.geminiPauseReason() == .timeout)
    }

    @Test("Concurrent Gemini one-time requests cannot start a second collector pass")
    func oneTimeGeminiCheckRejectsConcurrentRequest() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = OneTimeGeminiCollector(oneTimeDelay: 0.15)
        let coordinator = RefreshCoordinator(
            collectors: [collector],
            cache: SnapshotCache(directoryURL: directory),
            backoffURL: directory.appendingPathComponent("backoff.json")
        )
        _ = await coordinator.refresh()
        let authorization = try await authorizeRecovery(on: coordinator)

        async let first = coordinator.collectGeminiQuotaOnce(authorization: authorization)
        try await Task.sleep(for: .milliseconds(20))
        await #expect(throws: GeminiOneTimeQuotaError.alreadyRunning) {
            try await coordinator.collectGeminiQuotaOnce(authorization: authorization)
        }
        _ = try await first
        #expect(collector.oneTimeCalls == 1)
    }

    @Test("A concurrent provider refresh preserves the newer quota and suspension")
    func concurrentRefreshPreservesOneTimeGeminiQuota() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let cache = SnapshotCache(directoryURL: directory)
        let oldFiveHour = UsageMetric(
            label: "Gemini · Five hour", current: 35, limit: 100,
            unit: .percent, kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 900)
        )
        let oldWeekly = UsageMetric(
            label: "Gemini · Weekly", current: 35, limit: 100,
            unit: .percent, kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 900)
        )
        let oldGemini = UsageSnapshot(
            provider: .gemini,
            primaryMetric: oldWeekly,
            secondaryMetric: oldFiveHour,
            fetchedAt: Date(timeIntervalSince1970: 700),
            geminiQuotaMetrics: [oldFiveHour, oldWeekly]
        )
        try cache.save([oldGemini])
        let quotaCollector = OneTimeGeminiCollector(oneTimeDelay: 0.15)
        let claudeCollector = ControlledCollector(provider: .claude, delay: 0.3, result: .success(snapshot(for: .claude)))
        let coordinator = RefreshCoordinator(
            collectors: [quotaCollector, claudeCollector],
            cache: cache,
            backoffURL: directory.appendingPathComponent("backoff.json")
        )
        _ = await coordinator.refresh()
        let authorization = try await authorizeRecovery(on: coordinator)

        async let quota = coordinator.collectGeminiQuotaOnce(authorization: authorization)
        try await Task.sleep(for: .milliseconds(25))
        let refresh = Task { await coordinator.refresh() }
        _ = try await quota
        _ = await refresh.value

        let savedGemini = try #require(cache.load().first(where: { $0.provider == .gemini }))
        #expect(savedGemini.geminiQuotaMetrics?.first?.current == 77)
        #expect(savedGemini.fetchedAt == Date(timeIntervalSince1970: 800))
        #expect(await coordinator.geminiPauseReason() == .timeout)
    }

    @Test("A cached Gemini quota shows the specific pause reason immediately")
    func cachedGeminiQuotaShowsPauseReason() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let cache = SnapshotCache(directoryURL: directory)
        let metrics = [
            UsageMetric(label: "Gemini · Five hour", current: 20, limit: 100, unit: .percent, resetAt: Date()),
            UsageMetric(label: "Gemini · Weekly", current: 35, limit: 100, unit: .percent, resetAt: Date()),
        ]
        try cache.save([UsageSnapshot(provider: .gemini, primaryMetric: metrics[1], secondaryMetric: metrics[0], geminiQuotaMetrics: metrics)])
        let collector = ControlledCollector(provider: .gemini, result: .failure(.timedOut))
        let coordinator = RefreshCoordinator(collectors: [collector], cache: cache)

        let result = await coordinator.refresh()

        #expect(result.first?.collectionStatus == .cached)
        #expect(result.first?.statusMessage == "Antigravity refresh paused after a timeout")
        #expect(result.first?.primaryMetric != nil)
    }

    @Test("Legacy Antigravity authentication pause migrates to unknown and stays closed")
    func legacyAuthenticationPauseDoesNotInventReasonOrRetry() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = ControlledCollector(provider: .gemini, result: .failure(.timedOut))
        let cache = SnapshotCache(directoryURL: directory)
        let path = directory.appendingPathComponent("backoff.json")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let legacy = #"["gemini",{"failureKind":"authentication","consecutiveFailures":1,"nextEligibleAt":0,"recordedAt":0}]"#.data(using: .utf8)!
        try legacy.write(to: path)

        let coordinator = RefreshCoordinator(collectors: [collector], cache: cache, backoffURL: path)
        _ = await coordinator.refresh(manual: true)

        #expect(collector.callCount == 0)
        #expect(await coordinator.geminiPauseReason() == .unknown)
        let saved = try Data(contentsOf: path)
        #expect(String(decoding: saved, as: UTF8.self).contains("unknown"))
    }
    @Test("Runs independent provider collectors concurrently and sorts their results")
    func refreshesConcurrently() async {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let concurrencyProbe = ConcurrentCollectionProbe()
        let collectors = UsageProvider.allCases.reversed().map {
            ControlledCollector(
                provider: $0,
                delay: 0.2,
                result: .success(snapshot(for: $0)),
                concurrencyProbe: concurrencyProbe
            )
        }
        let coordinator = RefreshCoordinator(
            collectors: collectors,
            cache: SnapshotCache(directoryURL: directory)
        )

        let snapshots = await coordinator.refresh()

        // All four independently delayed collectors must overlap; result order is literal,
        // so returning the reversed registration order cannot satisfy this check.
        #expect(concurrencyProbe.maxConcurrentCalls == 4)
        #expect(snapshots.map(\.provider) == [.claude, .codex, .deepSeek, .gemini])
        #expect(snapshots.allSatisfy { $0.collectionStatus == .fresh })
    }

    @Test("A failed provider falls back to its last good cache without blocking others")
    func fallsBackToCache() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let cache = SnapshotCache(directoryURL: directory)
        let oldCodex = UsageSnapshot(
            provider: .codex,
            primaryMetric: metric(value: 44),
            fetchedAt: Date(timeIntervalSince1970: 100),
            staleAfter: 60,
            collectionStatus: .fresh,
            codexResetCredits: CodexResetCreditsSummary(
                availableCount: 1,
                credits: [CodexResetCreditDisplay(
                    title: "Bonus reset",
                    expiresAt: Date(timeIntervalSince1970: 1_900_000_000)
                )],
                hasCompleteDetails: true
            )
        )
        try cache.save([oldCodex])
        let coordinator = RefreshCoordinator(
            collectors: [
                ControlledCollector(provider: .codex, result: .failure(.authenticationRequired)),
                ControlledCollector(provider: .claude, result: .success(snapshot(for: .claude))),
            ],
            cache: cache
        )

        let snapshots = await coordinator.refresh()
        let claude = try #require(snapshots.first(where: { $0.provider == .claude }))
        let codex = try #require(snapshots.first(where: { $0.provider == .codex }))

        #expect(claude.collectionStatus == .fresh)
        #expect(codex.collectionStatus == .cached)
        #expect(codex.primaryMetric?.current == 44)
        #expect(codex.fetchedAt == Date(timeIntervalSince1970: 100))
        #expect(codex.statusMessage == "Sign in required")
        #expect(codex.codexResetCredits?.availableCount == 1)
    }

    @Test("Cached Claude fallback preserves local activity aggregates")
    func preservesClaudeLocalActivityInCacheFallback() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let reference = Date(timeIntervalSince1970: 1_900_000_000)
        let activity = ClaudeLocalActivitySummary(
            days: [ClaudeDailyActivity(date: reference, inputTokens: 10, outputTokens: 20, cacheTokens: 30)],
            sessionCount: 1,
            activeDayCount: 1,
            models: [ClaudeModelActivity(modelID: "claude-sonnet-4-6", tokenCount: 60)],
            updatedAt: reference
        )
        let cache = SnapshotCache(directoryURL: directory)
        try cache.save([UsageSnapshot(provider: .claude, claudeLocalActivity: activity)])
        let coordinator = RefreshCoordinator(
            collectors: [ControlledCollector(provider: .claude, result: .failure(.timedOut))],
            cache: cache
        )

        let snapshot = try #require(await coordinator.refresh().first)

        #expect(snapshot.collectionStatus == .cached)
        #expect(snapshot.claudeLocalActivity == activity)
    }

    @Test("Two overlapping refresh requests share one collection pass")
    func mergesOverlappingRefreshes() async {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = ControlledCollector(
            provider: .claude,
            delay: 0.15,
            result: .success(snapshot(for: .claude))
        )
        let coordinator = RefreshCoordinator(
            collectors: [collector],
            cache: SnapshotCache(directoryURL: directory)
        )

        async let first = coordinator.refresh()
        async let second = coordinator.refresh()
        let results = await (first, second)

        #expect(results.0 == results.1)
        #expect(collector.callCount == 1)
    }

    @Test("A failure without cache returns a sanitized actionable state")
    func reportsFailureWithoutCache() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let coordinator = RefreshCoordinator(
            collectors: [ControlledCollector(provider: .deepSeek, result: .failure(.rateLimited))],
            cache: SnapshotCache(directoryURL: directory)
        )

        let snapshot = try #require(await coordinator.refresh().first)

        #expect(snapshot.provider == .deepSeek)
        #expect(snapshot.collectionStatus == .unavailable)
        #expect(snapshot.statusMessage == "Rate limited; try again later")
        #expect(snapshot.primaryMetric == nil)
    }

    @Test("Claude workspace approval is distinct from a service outage")
    func reportsClaudeWorkspaceSetup() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let coordinator = RefreshCoordinator(
            collectors: [ControlledCollector(provider: .claude, result: .failure(.setupRequired))],
            cache: SnapshotCache(directoryURL: directory)
        )

        let snapshot = try #require(await coordinator.refresh().first)

        #expect(snapshot.provider == .claude)
        #expect(snapshot.collectionStatus == .setupRequired)
        #expect(snapshot.statusMessage == "Approve the private usage workspace once")
        #expect(snapshot.primaryMetric == nil)
    }

    private func temporaryDirectory() -> URL {
        FileManager.default.temporaryDirectory
            .appendingPathComponent("AI-Meter-RefreshTests-\(UUID().uuidString)", isDirectory: true)
    }

    private func waitForGate(_ gate: ResumeCollectionGate) async -> Bool {
        let deadline = ContinuousClock.now.advanced(by: .seconds(2))
        while ContinuousClock.now < deadline {
            if await gate.hasEntered() { return true }
            await Task.yield()
        }
        return false
    }

    private func resumeQuota(
        provider: UsageProvider = .gemini, status: CollectionStatus = .fresh,
        version: String? = "1.3.1", metrics: Bool = true, malformed: Bool = false
    ) -> UsageSnapshot {
        let fiveHour = UsageMetric(
            label: "Gemini · Five hour", current: 60, limit: 100,
            unit: .percent, kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 900)
        )
        let weekly = UsageMetric(
            label: "Gemini · Weekly", current: 77, limit: 100,
            unit: .percent, kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 900)
        )
        return UsageSnapshot(
            provider: provider, primaryMetric: weekly, secondaryMetric: fiveHour,
            fetchedAt: Date(timeIntervalSince1970: 800), sourceVersion: version,
            collectionStatus: status, geminiQuotaMetrics: metrics ? malformed ? [weekly] : [fiveHour, weekly] : []
        )
    }

    private func snapshot(for provider: UsageProvider) -> UsageSnapshot {
        UsageSnapshot(
            provider: provider,
            primaryMetric: metric(value: Double(10 + UsageProvider.allCases.firstIndex(of: provider)!)),
            fetchedAt: Date(timeIntervalSince1970: 500),
            collectionStatus: .fresh
        )
    }

    private func metric(value: Double) -> UsageMetric {
        UsageMetric(label: "Usage", current: value, limit: 100, unit: .percent)
    }

    private func authorizeRecovery(on coordinator: RefreshCoordinator) async throws -> GeminiQuotaRecoveryAuthorization {
        let token = UUID().uuidString
        #expect(await coordinator.beginGeminiQuotaRecovery(token: token))
        #expect(await coordinator.authorizeGeminiQuotaRecoveryAfterSignIn(token: "wrong-token") == nil)
        return try #require(await coordinator.authorizeGeminiQuotaRecoveryAfterSignIn(token: token))
    }
}

private final class ControlledCollector: OneTimeGeminiQuotaCollecting, @unchecked Sendable {
    let provider: UsageProvider
    let delay: TimeInterval
    let result: Result<UsageSnapshot, UsageCollectionError>
    let concurrencyProbe: ConcurrentCollectionProbe?

    private let lock = NSLock()
    private var calls = 0

    init(
        provider: UsageProvider,
        delay: TimeInterval = 0,
        result: Result<UsageSnapshot, UsageCollectionError>,
        concurrencyProbe: ConcurrentCollectionProbe? = nil
    ) {
        self.provider = provider
        self.delay = delay
        self.result = result
        self.concurrencyProbe = concurrencyProbe
    }

    var callCount: Int { lock.withLock { calls } }

    func collect() async throws -> UsageSnapshot {
        lock.withLock { calls += 1 }
        concurrencyProbe?.enter()
        defer { concurrencyProbe?.leave() }
        if delay > 0 {
            try await Task.sleep(for: .seconds(delay))
        }
        return try result.get()
    }

    // Preserve each existing synthetic outcome when Gemini uses the reduced automatic route.
    func collectQuotaOnce() async throws -> UsageSnapshot { try await collect() }
}

private final class OneTimeGeminiCollector: OneTimeGeminiQuotaCollecting, @unchecked Sendable {
    let provider = UsageProvider.gemini
    private let oneTimeDelay: TimeInterval
    private let lock = NSLock()
    private var normal = 0
    private var oneTime = 0

    init(oneTimeDelay: TimeInterval = 0) { self.oneTimeDelay = oneTimeDelay }

    var normalCalls: Int { lock.withLock { normal } }
    var oneTimeCalls: Int { lock.withLock { oneTime } }

    func collect() async throws -> UsageSnapshot {
        let call = lock.withLock { normal += 1; return normal }
        guard call > 1 else { throw UsageCollectionError.timedOut }
        return quotaSnapshot(value: 77)
    }

    func collectQuotaOnce() async throws -> UsageSnapshot {
        if normalCalls == 0 { return try await collect() }
        lock.withLock { oneTime += 1 }
        if oneTimeDelay > 0 { try await Task.sleep(for: .seconds(oneTimeDelay)) }
        return quotaSnapshot(value: 77)
    }

    private func quotaSnapshot(value: Double) -> UsageSnapshot {
        let fiveHour = UsageMetric(
            label: "Gemini · Five hour", current: value, limit: 100,
            unit: .percent, kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 900)
        )
        let weekly = UsageMetric(
            label: "Gemini · Weekly", current: value, limit: 100,
            unit: .percent, kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 900)
        )
        let sharedFiveHour = UsageMetric(
            label: "Claude/GPT · Five hour", current: 80, limit: 100,
            unit: .percent, kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 900)
        )
        let sharedWeekly = UsageMetric(
            label: "Claude/GPT · Weekly", current: 20, limit: 100,
            unit: .percent, kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 900)
        )
        return UsageSnapshot(
            provider: .gemini,
            primaryMetric: weekly,
            fetchedAt: Date(timeIntervalSince1970: 800),
            collectionStatus: .fresh,
            geminiQuotaMetrics: [fiveHour, weekly],
            antigravitySharedQuotaMetrics: [sharedFiveHour, sharedWeekly]
        )
    }
}

/// Synthetic-only fixture: the initial full refresh pauses, then quota reads are fresh and supported.
private final class ExplicitResumeGeminiCollector: OneTimeGeminiQuotaCollecting, @unchecked Sendable {
    let provider = UsageProvider.gemini
    private let lock = NSLock()
    private var normal = 0
    private var quota = 0
    private var quotaResult: Result<UsageSnapshot, UsageCollectionError>?
    let gate = ResumeCollectionGate()

    var normalCalls: Int { lock.withLock { normal } }
    var quotaCalls: Int { lock.withLock { quota } }

    func setQuotaResult(_ result: Result<UsageSnapshot, UsageCollectionError>) {
        lock.withLock { quotaResult = result }
    }

    func collect() async throws -> UsageSnapshot {
        let call = lock.withLock { normal += 1; return normal }
        guard call > 1 else { throw UsageCollectionError.timedOut }
        return freshQuota()
    }

    func collectQuotaOnce() async throws -> UsageSnapshot {
        lock.withLock { quota += 1 }
        await gate.waitIfBlocked()
        return try lock.withLock { quotaResult ?? .success(freshQuota()) }.get()
    }

    private func freshQuota() -> UsageSnapshot {
        let fiveHour = UsageMetric(
            label: "Gemini · Five hour", current: 60, limit: 100,
            unit: .percent, kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 900)
        )
        let quota = UsageMetric(
            label: "Gemini · Weekly", current: 77, limit: 100,
            unit: .percent, kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 900)
        )
        return UsageSnapshot(
            provider: .gemini, primaryMetric: quota, secondaryMetric: fiveHour,
            fetchedAt: Date(timeIntervalSince1970: 800), sourceVersion: "1.3.1",
            collectionStatus: .fresh, geminiQuotaMetrics: [fiveHour, quota]
        )
    }
}

private final class ResumeTestClock: @unchecked Sendable {
    let origin = ContinuousClock.now
    private let lock = NSLock()
    private var offset: Duration = .zero

    func now() -> ContinuousClock.Instant { lock.withLock { origin.advanced(by: offset) } }
    func setOffset(_ value: Duration) { lock.withLock { offset = value } }
}

private actor ResumeCollectionGate {
    private var blocked = false
    private var entered = false
    private var continuation: CheckedContinuation<Void, Never>?

    func block() { blocked = true; entered = false }
    func hasEntered() -> Bool { entered }
    func waitIfBlocked() async {
        guard blocked else { return }
        entered = true
        await withCheckedContinuation { continuation = $0 }
    }
    func release() {
        blocked = false
        continuation?.resume()
        continuation = nil
    }
}

private struct GatedResumeOtherCollector: UsageCollector {
    let provider = UsageProvider.claude
    let gate: ResumeCollectionGate
    func collect() async throws -> UsageSnapshot {
        await gate.waitIfBlocked()
        return UsageSnapshot(provider: provider)
    }
}

private final class FullOnlyGeminiCollector: UsageCollector, @unchecked Sendable {
    let provider = UsageProvider.gemini
    private let lock = NSLock()
    private var calls = 0
    var callCount: Int { lock.withLock { calls } }
    func collect() async throws -> UsageSnapshot {
        lock.withLock { calls += 1 }
        return UsageSnapshot(provider: provider)
    }
}

private struct FailingOneTimeGeminiCollector: OneTimeGeminiQuotaCollecting {
    let provider = UsageProvider.gemini

    func collect() async throws -> UsageSnapshot { throw UsageCollectionError.timedOut }
    func collectQuotaOnce() async throws -> UsageSnapshot { throw UsageCollectionError.timedOut }
}

private final class InvalidOneTimeGeminiCollector: OneTimeGeminiQuotaCollecting, @unchecked Sendable {
    let provider = UsageProvider.gemini
    private let lock = NSLock()
    private var oneTime = 0
    private var initialFailureRecorded = false

    var oneTimeCalls: Int { lock.withLock { oneTime } }

    func collect() async throws -> UsageSnapshot {
        lock.withLock { initialFailureRecorded = true }
        throw UsageCollectionError.timedOut
    }
    func collectQuotaOnce() async throws -> UsageSnapshot {
        if !lock.withLock({ initialFailureRecorded }) { return try await collect() }
        lock.withLock { oneTime += 1 }
        return UsageSnapshot(provider: .gemini)
    }
}

private final class ConcurrentCollectionProbe: @unchecked Sendable {
    private let lock = NSLock()
    private var activeCalls = 0
    private var maximumCalls = 0

    var maxConcurrentCalls: Int { lock.withLock { maximumCalls } }

    func enter() {
        lock.withLock {
            activeCalls += 1
            maximumCalls = max(maximumCalls, activeCalls)
        }
    }

    func leave() {
        lock.withLock { activeCalls -= 1 }
    }
}
