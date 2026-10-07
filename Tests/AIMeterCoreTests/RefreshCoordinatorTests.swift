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

    @Test("One-time Gemini quota success updates cache without clearing persistent suspension")
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
        let recovered = try await initial.collectGeminiQuotaOnce()

        #expect(recovered.geminiQuotaMetrics?.first?.current == 77)
        #expect(await initial.geminiPauseReason() == .timeout)
        let saved = try cache.load()
        #expect(saved.first(where: { $0.provider == .claude })?.fetchedAt == claudeCache.fetchedAt)
        #expect(saved.first(where: { $0.provider == .gemini })?.fetchedAt == Date(timeIntervalSince1970: 800))

        let restarted = RefreshCoordinator(collectors: collectors, cache: cache, backoffURL: backoffURL)
        _ = await restarted.refresh(manual: true)
        #expect(await restarted.geminiPauseReason() == .timeout)
        #expect(quotaCollector.normalCalls == 1)
        #expect(quotaCollector.oneTimeCalls == 1)
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

        await #expect(throws: UsageCollectionError.timedOut) {
            try await coordinator.collectGeminiQuotaOnce()
        }
        let after = try cache.load()
        #expect(after.first(where: { $0.provider == .gemini })?.fetchedAt == oldQuota.fetchedAt)
        #expect(after.first(where: { $0.provider == .claude })?.fetchedAt == oldClaude.fetchedAt)
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

        async let first = coordinator.collectGeminiQuotaOnce()
        try await Task.sleep(for: .milliseconds(20))
        await #expect(throws: GeminiOneTimeQuotaError.alreadyRunning) {
            try await coordinator.collectGeminiQuotaOnce()
        }
        _ = try await first
        #expect(collector.oneTimeCalls == 1)
    }

    @Test("A concurrent provider refresh cannot overwrite a newer one-time Gemini quota")
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

        async let quota = coordinator.collectGeminiQuotaOnce()
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
}

private final class ControlledCollector: UsageCollector, @unchecked Sendable {
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
        lock.withLock { normal += 1 }
        throw UsageCollectionError.timedOut
    }

    func collectQuotaOnce() async throws -> UsageSnapshot {
        lock.withLock { oneTime += 1 }
        if oneTimeDelay > 0 { try await Task.sleep(for: .seconds(oneTimeDelay)) }
        let fiveHour = UsageMetric(
            label: "Gemini · Five hour", current: 77, limit: 100,
            unit: .percent, kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 900)
        )
        let weekly = UsageMetric(
            label: "Gemini · Weekly", current: 77, limit: 100,
            unit: .percent, kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 900)
        )
        return UsageSnapshot(
            provider: .gemini,
            primaryMetric: weekly,
            fetchedAt: Date(timeIntervalSince1970: 800),
            collectionStatus: .fresh,
            geminiQuotaMetrics: [fiveHour, weekly]
        )
    }
}

private struct FailingOneTimeGeminiCollector: OneTimeGeminiQuotaCollecting {
    let provider = UsageProvider.gemini

    func collect() async throws -> UsageSnapshot { throw UsageCollectionError.timedOut }
    func collectQuotaOnce() async throws -> UsageSnapshot { throw UsageCollectionError.timedOut }
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
