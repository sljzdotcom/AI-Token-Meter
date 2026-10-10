import Foundation
import Testing
@testable import AIMeterApp
@testable import AIMeterCore

@Suite("App model Antigravity one-time recovery", .serialized)
struct AppModelGeminiRecoveryTests {
    @Test("Unknown pause exposes one login token and accepts only one success receipt")
    @MainActor
    func unknownPauseUsesSingleValidReceiptWithoutGlobalRefresh() async throws {
        let context = makeRecoveryContext()
        defer {
            context.defaults.removePersistentDomain(forName: context.suiteName)
            try? FileManager.default.removeItem(at: context.applicationSupport)
        }
        let counters = RecoveryCounters()
        var openedToken: String?
        let freshQuota = recoverySnapshot(value: 22, fetchedAt: Date(timeIntervalSince1970: 900))
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(freshQuota),
            openLogin: { openedToken = $0 }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        await model.beginGeminiOneTimeRecovery()
        #expect(openedToken == token)
        #expect(model.isGeminiOneTimeRecoveryInProgress)

        await model.refresh(manual: false)
        #expect(await counters.quotaCalls == 0)
        #expect(model.geminiPauseReason == .unknown)

        await model.completeGeminiInteractiveSignIn(token: UUID().uuidString, result: .success)
        #expect(await counters.quotaCalls == 0)

        await model.completeGeminiInteractiveSignIn(token: token, result: .success)
        #expect(await counters.quotaCalls == 1)
        #expect(await counters.globalRefreshCalls == 2)
        #expect(model.geminiPauseReason == .unknown)
        #expect(model.isGeminiRefreshPaused)
        #expect(model.geminiOneTimeRecoveryState == .succeeded)
        #expect(model.settingsNotice == .recoveryQuotaUpdated)
        #expect(model.snapshots.first(where: { $0.provider == .gemini })?.fetchedAt == freshQuota.fetchedAt)

        await model.completeGeminiInteractiveSignIn(token: token, result: .success)
        #expect(await counters.quotaCalls == 1)
        #expect(model.geminiPauseReason == .unknown)
        #expect(model.isGeminiRefreshPaused)
    }

    @Test("Concurrent recovery starts open only one interactive login")
    @MainActor
    func concurrentRecoveryStartsShareOneLogin() async throws {
        let context = makeRecoveryContext()
        defer {
            context.defaults.removePersistentDomain(forName: context.suiteName)
            try? FileManager.default.removeItem(at: context.applicationSupport)
        }
        let counters = RecoveryCounters()
        var openedTokens: [String] = []
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 22)),
            openLogin: { openedTokens.append($0) },
            recoveryBegin: {
                try? await Task.sleep(for: .milliseconds(30))
                return true
            }
        )

        await model.refresh()
        async let first: Void = model.beginGeminiOneTimeRecovery()
        async let second: Void = model.beginGeminiOneTimeRecovery()
        _ = await (first, second)

        #expect(openedTokens.count == 1)
        #expect(model.isGeminiOneTimeRecoveryInProgress)
    }

    @Test("Nonzero login receipt and CLI launch failure never start a quota check")
    @MainActor
    func failedLoginRemainsPaused() async throws {
        let context = makeRecoveryContext()
        defer {
            context.defaults.removePersistentDomain(forName: context.suiteName)
            try? FileManager.default.removeItem(at: context.applicationSupport)
        }
        let counters = RecoveryCounters()
        var openedToken: String?
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .authenticationRequired,
            quota: .success(recoverySnapshot(value: 1)),
            openLogin: { openedToken = $0 }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        await model.completeGeminiInteractiveSignIn(token: token, result: .failure)

        #expect(await counters.quotaCalls == 0)
        #expect(model.geminiPauseReason == .authenticationRequired)
        #expect(model.geminiOneTimeRecoveryState == .loginFailed)

        let failedLaunchContext = makeRecoveryContext()
        defer {
            failedLaunchContext.defaults.removePersistentDomain(forName: failedLaunchContext.suiteName)
            try? FileManager.default.removeItem(at: failedLaunchContext.applicationSupport)
        }
        let failedLaunchCounters = RecoveryCounters()
        let failedLaunchModel = makeModel(
            context: failedLaunchContext,
            counters: failedLaunchCounters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 2)),
            openLogin: { _ in },
            openShouldFail: true
        )
        await failedLaunchModel.refresh()
        await failedLaunchModel.beginGeminiOneTimeRecovery()
        #expect(failedLaunchModel.geminiOneTimeRecoveryState == .loginFailed)
        #expect(await failedLaunchCounters.quotaCalls == 0)
    }

    @Test("Expired login receipt is discarded and a later success cannot start collection")
    @MainActor
    func expiredReceiptCannotResume() async throws {
        let context = makeRecoveryContext()
        defer {
            context.defaults.removePersistentDomain(forName: context.suiteName)
            try? FileManager.default.removeItem(at: context.applicationSupport)
        }
        let counters = RecoveryCounters()
        let timeoutGate = RecoveryTimeoutGate()
        var openedToken: String?
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 1)),
            openLogin: { openedToken = $0 },
            recoveryTimeout: .milliseconds(5),
            signInSleep: { _ in try await timeoutGate.sleep() }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        await timeoutGate.waitUntilStarted()
        await timeoutGate.expire()
        for _ in 0..<100 where model.geminiOneTimeRecoveryState != .loginExpired {
            await Task.yield()
        }
        #expect(model.geminiOneTimeRecoveryState == .loginExpired)

        await model.completeGeminiInteractiveSignIn(token: token, result: .success)
        #expect(await counters.quotaCalls == 0)
        #expect(model.geminiPauseReason == .unknown)
    }

    @Test("Application stop invalidates the token before any later callback")
    @MainActor
    func appStopInvalidatesPendingReceipt() async throws {
        let context = makeRecoveryContext()
        defer {
            context.defaults.removePersistentDomain(forName: context.suiteName)
            try? FileManager.default.removeItem(at: context.applicationSupport)
        }
        let counters = RecoveryCounters()
        var openedToken: String?
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 1)),
            openLogin: { openedToken = $0 }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        model.stop()
        await model.completeGeminiInteractiveSignIn(token: token, result: .success)

        #expect(await counters.quotaCalls == 0)
        #expect(model.geminiOneTimeRecoveryState == .idle)
    }

    @Test("A refresh already in flight cannot replace a newer one-time Gemini result")
    @MainActor
    func inFlightRefreshPreservesRecoveredGeminiSnapshot() async throws {
        let context = makeRecoveryContext()
        defer {
            context.defaults.removePersistentDomain(forName: context.suiteName)
            try? FileManager.default.removeItem(at: context.applicationSupport)
        }
        let counters = RecoveryCounters()
        var openedToken: String?
        let stale = recoverySnapshot(value: 18, fetchedAt: Date(timeIntervalSince1970: 700))
        let recovered = recoverySnapshot(value: 83, fetchedAt: Date(timeIntervalSince1970: 900))
        let refreshSequence = RecoveryRefreshSequence(stale: stale)
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recovered),
            openLogin: { openedToken = $0 },
            refreshOperation: {
                await refreshSequence.next()
            }
        )

        await model.refresh()
        let refresh = Task { await model.refresh() }
        try await Task.sleep(for: .milliseconds(20))
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        await model.completeGeminiInteractiveSignIn(token: token, result: .success)
        #expect(model.snapshots.first(where: { $0.provider == .gemini })?.fetchedAt == recovered.fetchedAt)

        await refresh.value

        #expect(model.snapshots.first(where: { $0.provider == .gemini })?.fetchedAt == recovered.fetchedAt)
        #expect(model.snapshots.first(where: { $0.provider == .gemini })?.primaryMetric?.current == 83)
        #expect(model.geminiPauseReason == .unknown)
    }

    @MainActor
    private func makeModel(
        context: (suiteName: String, defaults: UserDefaults, applicationSupport: URL),
        counters: RecoveryCounters,
        pauseReason: GeminiPauseReason,
        quota: Result<UsageSnapshot, UsageCollectionError>,
        openLogin: @escaping (String) -> Void,
        recoveryTimeout: Duration = .seconds(310),
        openShouldFail: Bool = false,
        refreshOperation: (@Sendable () async -> [UsageSnapshot])? = nil,
        recoveryBegin: (@Sendable () async -> Bool)? = nil,
        signInSleep: (@Sendable (Duration) async throws -> Void)? = nil
    ) -> AppModel {
        AppModel(
            defaults: context.defaults,
            applicationSupportDirectoryURL: context.applicationSupport,
            widgetSnapshotPublisher: nil,
            isDemoMode: false,
            refreshOperation: refreshOperation ?? {
                await counters.incrementGlobalRefresh()
                return []
            },
            geminiPauseReasonOperation: { pauseReason },
            geminiRecoveryBeginOperation: { _ in
                if let recoveryBegin { return await recoveryBegin() }
                return true
            },
            geminiRecoveryAuthorizeOperation: { _ in GeminiQuotaRecoveryAuthorization(id: UUID()) },
            geminiQuotaCheckOperation: { _ in
                await counters.incrementQuota()
                return try quota.get()
            },
            geminiAuthenticationOpenOperation: { token in
                if openShouldFail { throw CocoaError(.fileWriteUnknown) }
                openLogin(token)
            },
            geminiRecoveryTimeout: recoveryTimeout,
            signInSleep: signInSleep ?? { try await Task.sleep(for: $0) }
        )
    }

    private func recoverySnapshot(value: Double, fetchedAt: Date = Date(timeIntervalSince1970: 800)) -> UsageSnapshot {
        let metric = UsageMetric(label: "Gemini · Weekly", current: value, limit: 100, unit: .percent)
        return UsageSnapshot(
            provider: .gemini,
            primaryMetric: metric,
            fetchedAt: fetchedAt,
            collectionStatus: .fresh,
            geminiQuotaMetrics: [metric]
        )
    }

    private func makeRecoveryContext() -> (suiteName: String, defaults: UserDefaults, applicationSupport: URL) {
        let suiteName = "AppModelGeminiRecoveryTests.\(UUID().uuidString)"
        let support = FileManager.default.temporaryDirectory
            .appendingPathComponent(suiteName, isDirectory: true)
        return (suiteName, UserDefaults(suiteName: suiteName)!, support)
    }
}

private actor RecoveryCounters {
    private(set) var quotaCalls = 0
    private(set) var globalRefreshCalls = 0

    func incrementQuota() { quotaCalls += 1 }
    func incrementGlobalRefresh() { globalRefreshCalls += 1 }
}

private actor RecoveryTimeoutGate {
    private var continuation: CheckedContinuation<Void, Error>?
    private var startedContinuation: CheckedContinuation<Void, Never>?

    func sleep() async throws {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            self.continuation = continuation
            startedContinuation?.resume()
            startedContinuation = nil
        }
    }

    func waitUntilStarted() async {
        if continuation != nil { return }
        await withCheckedContinuation { continuation in
            startedContinuation = continuation
        }
    }

    func expire() {
        continuation?.resume()
        continuation = nil
    }
}

private actor RecoveryRefreshSequence {
    private let stale: UsageSnapshot
    private var calls = 0

    init(stale: UsageSnapshot) { self.stale = stale }

    func next() async -> [UsageSnapshot] {
        calls += 1
        if calls > 1 { try? await Task.sleep(for: .milliseconds(150)) }
        return [stale]
    }
}
