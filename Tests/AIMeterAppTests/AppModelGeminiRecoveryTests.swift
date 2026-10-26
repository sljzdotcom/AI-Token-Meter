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
        defer { clean(context) }
        let counters = RecoveryCounters()
        var openedToken: String?
        var cancelledTokens: [String] = []
        var stoppedTokens: [String] = []
        let freshQuota = recoverySnapshot(value: 22, fetchedAt: Date(timeIntervalSince1970: 900))
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(freshQuota),
            openLogin: { openedToken = $0 },
            cancelLogin: { cancelledTokens.append($0) },
            stopLogin: { stoppedTokens.append($0) }
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
        #expect(cancelledTokens == [token])
        #expect(stoppedTokens.isEmpty)

        await model.completeGeminiInteractiveSignIn(token: token, result: .success)
        #expect(await counters.quotaCalls == 1)
        #expect(model.geminiPauseReason == .unknown)
        #expect(model.isGeminiRefreshPaused)
    }

    @Test("Concurrent recovery starts open only one interactive login")
    @MainActor
    func concurrentRecoveryStartsShareOneLogin() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
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
        defer { clean(context) }
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
        defer { clean(failedLaunchContext) }
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

    @Test("A success receipt without confirmed CLI exec stops recovery before quota collection")
    @MainActor
    func unconfirmedExecReceiptCannotStartQuotaCollection() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        var openedToken: String?
        var cancelledTokens: [String] = []
        var stoppedTokens: [String] = []
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 22)),
            openLogin: { openedToken = $0 },
            cancelLogin: { cancelledTokens.append($0) },
            stopLogin: { stoppedTokens.append($0) },
            execConfirmed: { _ in false }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        await model.completeGeminiInteractiveSignIn(token: token, result: .success)

        #expect(await counters.quotaCalls == 0)
        #expect(model.geminiOneTimeRecoveryState == .loginFailed)
        #expect(model.geminiPauseReason == .unknown)
        #expect(model.isGeminiRefreshPaused)
        #expect(cancelledTokens.isEmpty)
        #expect(stoppedTokens == [token])
    }

    @Test("Expired login receipt is discarded and a later success cannot start collection")
    @MainActor
    func expiredReceiptCannotResume() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let timeoutGate = RecoveryTimeoutGate()
        defer { Task { await timeoutGate.cancel() } }
        var openedToken: String?
        var stoppedTokens: [String] = []
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 1)),
            openLogin: { openedToken = $0 },
            stopLogin: { stoppedTokens.append($0) },
            recoveryTimeout: .milliseconds(500),
            signInSleep: { duration in try await timeoutGate.sleep(duration: duration) }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        let gateObservation = await timeoutGate.waitForStartOrCompletion(
            completed: { model.geminiOneTimeRecoveryState == .loginExpired }, timeout: .seconds(2)
        )
        #expect(gateObservation != .timedOut)
        await timeoutGate.expire()
        let expiryDeadline = ContinuousClock.now.advanced(by: .seconds(2))
        while model.geminiOneTimeRecoveryState != .loginExpired, ContinuousClock.now < expiryDeadline {
            await Task.yield()
        }
        await timeoutGate.cancel()
        #expect(model.geminiOneTimeRecoveryState == .loginExpired)
        #expect(stoppedTokens == [token])

        await model.completeGeminiInteractiveSignIn(token: token, result: .success)
        #expect(await counters.quotaCalls == 0)
        #expect(model.geminiPauseReason == .unknown)
    }

    @Test("Expiry before the first monitor sleep remains bounded for both stop outcomes", arguments: [true, false])
    @MainActor
    func expiryBeforeFirstSleepDoesNotHang(acknowledged: Bool) async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let gate = RecoveryTimeoutGate()
        defer { Task { await gate.cancel() } }
        let expected: AppModel.GeminiOneTimeRecoveryState = acknowledged ? .loginExpired : .loginFailed
        let model = makeModel(
            context: context, counters: counters, pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 1)), openLogin: { _ in }, stopAcknowledged: acknowledged,
            recoveryTimeout: .milliseconds(1),
            terminalFailure: { _ in
                // This injected synchronous dependency deliberately consumes the
                // deadline before the monitor can enter its first sleep.
                Thread.sleep(forTimeInterval: 0.02)
                return false
            },
            signInSleep: { duration in try await gate.sleep(duration: duration) }
        )
        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let observed = await gate.waitForStartOrCompletion(
            completed: { model.geminiOneTimeRecoveryState == expected }, timeout: .seconds(2)
        )
        await gate.cancel()
        #expect(observed == .completed)
        #expect(model.geminiOneTimeRecoveryState == expected)
        #expect(model.isGeminiRefreshPaused)
        #expect(await counters.quotaCalls == 0)
    }

    @Test("Timeout test control observes expiry before sleep and cancels an unreleased sleep")
    func timeoutGateHasBoundedStartAndTeardown() async throws {
        let completedGate = RecoveryTimeoutGate()
        #expect(await completedGate.waitForStartOrCompletion(completed: { true }, timeout: .milliseconds(100)) == .completed)
        let unusedGate = RecoveryTimeoutGate()
        #expect(await unusedGate.waitForStartOrCompletion(completed: { false }, timeout: .milliseconds(20)) == .timedOut)
        let sleepingGate = RecoveryTimeoutGate()
        let sleeper = Task { try await sleepingGate.sleep(duration: .milliseconds(100)) }
        #expect(await sleepingGate.waitForStartOrCompletion(completed: { false }, timeout: .seconds(1)) == .started)
        await sleepingGate.cancel()
        do {
            try await sleeper.value
            Issue.record("An unreleased timeout continuation must throw during teardown")
        } catch is CancellationError { }
    }

    @Test("An expired login with no stop acknowledgement is not reported as safely expired")
    @MainActor
    func timeoutWithoutStopAcknowledgementRemainsFailed() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let timeoutGate = RecoveryTimeoutGate()
        defer { Task { await timeoutGate.cancel() } }
        var openedToken: String?
        var stoppedTokens: [String] = []
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 1)),
            openLogin: { openedToken = $0 },
            stopLogin: { stoppedTokens.append($0) },
            stopAcknowledged: false,
            recoveryTimeout: .milliseconds(500),
            signInSleep: { duration in try await timeoutGate.sleep(duration: duration) }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        let gateObservation = await timeoutGate.waitForStartOrCompletion(
            completed: { model.geminiOneTimeRecoveryState == .loginFailed }, timeout: .seconds(2)
        )
        #expect(gateObservation != .timedOut)
        await timeoutGate.expire()
        let expiryDeadline = ContinuousClock.now.advanced(by: .seconds(2))
        while model.isGeminiOneTimeRecoveryInProgress, ContinuousClock.now < expiryDeadline {
            await Task.yield()
        }

        await timeoutGate.cancel()
        #expect(model.geminiOneTimeRecoveryState == .loginFailed)
        #expect(model.settingsNotice == .recoveryLoginFailed)
        #expect(model.isGeminiRefreshPaused)
        #expect(model.geminiPauseReason == .unknown)
        #expect(stoppedTokens == [token])
        #expect(await counters.quotaCalls == 0)
    }

    @Test("A late success receipt is rejected while stop acknowledgement is pending")
    @MainActor
    func lateSuccessReceiptCannotRaceTimeoutStop() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let stopGate = RecoveryStopGate()
        var openedToken: String?
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 1)),
            openLogin: { openedToken = $0 },
            stopOperation: { _ in
                Task {
                    await stopGate.waitUntilReleased()
                    return false
                }
            },
            recoveryTimeout: .milliseconds(5)
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        await stopGate.waitUntilStopStarted()

        await model.completeGeminiInteractiveSignIn(token: token, result: .success)
        #expect(model.geminiOneTimeRecoveryState != .succeeded)
        #expect(await counters.quotaCalls == 0)

        await stopGate.release()
        try await Task.sleep(for: .milliseconds(20))
        #expect(model.geminiOneTimeRecoveryState == .loginFailed)
        #expect(await counters.quotaCalls == 0)
        #expect(model.geminiPauseReason == .unknown)
    }

    @Test("An early Terminal failure ends the login wait without collecting quota")
    @MainActor
    func earlyTerminalFailureEndsRecovery() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        var openedToken: String?
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 5)),
            openLogin: { openedToken = $0 },
            terminalFailure: { $0 == openedToken }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        try await Task.sleep(for: .milliseconds(25))

        #expect(model.geminiOneTimeRecoveryState == .loginFailed)
        #expect(model.settingsNotice == .recoveryLoginFailed)
        #expect(await counters.quotaCalls == 0)
        #expect(model.geminiPauseReason == .unknown)
        await model.completeGeminiInteractiveSignIn(token: token, result: .success)
        #expect(await counters.quotaCalls == 0)
    }

    @Test("Uncertain guardian cleanup requests stop and preserves coordination state")
    @MainActor
    func uncertainGuardianCleanupUsesStopOperation() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        var openedToken: String?
        var cancelledTokens: [String] = []
        var stoppedTokens: [String] = []
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 22)),
            openLogin: { openedToken = $0 },
            cancelLogin: { cancelledTokens.append($0) },
            stopLogin: { stoppedTokens.append($0) },
            terminalFailure: { _ in true },
            cleanupUncertain: { _ in true }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        try await Task.sleep(for: .milliseconds(25))

        #expect(model.geminiOneTimeRecoveryState == .loginFailed)
        #expect(await counters.quotaCalls == 0)
        #expect(model.isGeminiRefreshPaused)
        #expect(cancelledTokens.isEmpty)
        #expect(stoppedTokens == [token])
    }

    @Test("Late guardian stop acknowledgement cannot overwrite a stopped or newer recovery")
    @MainActor
    func lateGuardianStopAcknowledgementCannotOverwriteNewRecovery() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let stopGate = RecoveryStopGate()
        var openedTokens: [String] = []
        var stopTasks: [Task<Bool, Never>] = []
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 22)),
            openLogin: { openedTokens.append($0) },
            stopOperation: { _ in
                let task = Task {
                    await stopGate.waitUntilReleased()
                    return false
                }
                stopTasks.append(task)
                return task
            },
            recoveryTimeout: .seconds(2),
            terminalFailure: { token in openedTokens.first == token },
            cleanupUncertain: { _ in true }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let oldToken = try #require(openedTokens.first)
        await stopGate.waitUntilStopStarted()

        let stoppedRecoveryTask = model.stop()
        #expect(model.geminiOneTimeRecoveryState == .idle)
        #expect(model.isGeminiRefreshPaused)
        #expect(await counters.quotaCalls == 0)

        await model.beginGeminiOneTimeRecovery()
        let newToken = try #require(openedTokens.last)
        #expect(newToken != oldToken)
        #expect(model.geminiOneTimeRecoveryState == .awaitingLogin)
        #expect(model.settingsNotice == .recoveryLoginStarted)

        #expect(stopTasks.count == 2)
        await stopGate.release()
        for task in stopTasks { _ = await task.value }
        await stoppedRecoveryTask?.value

        #expect(model.geminiOneTimeRecoveryState == .awaitingLogin)
        #expect(model.settingsNotice == .recoveryLoginStarted)
        #expect(model.isGeminiRefreshPaused)
        #expect(await counters.quotaCalls == 0)

        await model.completeGeminiInteractiveSignIn(token: newToken, result: .success)
        #expect(await counters.quotaCalls == 1)
        #expect(model.geminiOneTimeRecoveryState == .succeeded)
        #expect(model.isGeminiRefreshPaused)
    }

    @Test("A success receipt is ignored while uncertain guardian cleanup awaits acknowledgement")
    @MainActor
    func uncertainGuardianCleanupRejectsSuccessReceiptUntilAcknowledged() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let stopGate = RecoveryStopGate()
        var openedToken: String?
        var stopTask: Task<Bool, Never>?
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 22)),
            openLogin: { openedToken = $0 },
            stopOperation: { _ in
                let task = Task {
                    await stopGate.waitUntilReleased()
                    return false
                }
                stopTask = task
                return task
            },
            recoveryTimeout: .seconds(2),
            terminalFailure: { $0 == openedToken },
            cleanupUncertain: { _ in true }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        await stopGate.waitUntilStopStarted()

        #expect(model.geminiOneTimeRecoveryState == .stoppingLogin)
        await model.completeGeminiInteractiveSignIn(token: token, result: .success)
        #expect(model.geminiOneTimeRecoveryState == .stoppingLogin)
        #expect(model.isGeminiRefreshPaused)
        #expect(await counters.quotaCalls == 0)

        await stopGate.release()
        _ = await stopTask?.value
        let stoppedRecoveryTask = model.stop()
        await stoppedRecoveryTask?.value

        #expect(model.geminiOneTimeRecoveryState == .idle)
        #expect(model.isGeminiRefreshPaused)
        #expect(await counters.quotaCalls == 0)
    }

    @Test("A late unconfirmed-exec stop acknowledgement cannot overwrite a newer recovery")
    @MainActor
    func lateUnconfirmedExecStopAcknowledgementCannotOverwriteNewRecovery() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let oldStopGate = RecoveryStopGate()
        let newStopGate = RecoveryStopGate()
        var openedTokens: [String] = []
        var oldStopTask: Task<Bool, Never>?
        var newStopTask: Task<Bool, Never>?
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 22)),
            openLogin: { openedTokens.append($0) },
            stopOperation: { token in
                let isOldRecovery = token == openedTokens.first
                let task = Task {
                    if isOldRecovery {
                        await oldStopGate.waitUntilReleased()
                    } else {
                        await newStopGate.waitUntilReleased()
                    }
                    return false
                }
                if isOldRecovery {
                    oldStopTask = task
                } else {
                    newStopTask = task
                }
                return task
            },
            recoveryTimeout: .seconds(2),
            execConfirmed: { _ in false }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let oldToken = try #require(openedTokens.first)
        let oldReceipt = Task {
            await model.completeGeminiInteractiveSignIn(token: oldToken, result: .success)
        }
        await oldStopGate.waitUntilStopStarted()
        #expect(model.geminiOneTimeRecoveryState == .stoppingLogin)
        #expect(await counters.quotaCalls == 0)

        model.stop()
        #expect(model.geminiOneTimeRecoveryState == .idle)
        await model.beginGeminiOneTimeRecovery()
        let newToken = try #require(openedTokens.last)
        #expect(newToken != oldToken)
        let newReceipt = Task {
            await model.completeGeminiInteractiveSignIn(token: newToken, result: .success)
        }
        await newStopGate.waitUntilStopStarted()
        #expect(model.geminiOneTimeRecoveryState == .stoppingLogin)

        await oldStopGate.release()
        _ = await oldStopTask?.value
        await oldReceipt.value

        #expect(model.geminiOneTimeRecoveryState == .stoppingLogin)
        #expect(model.settingsNotice == .recoveryLoginStarted)
        #expect(model.isGeminiRefreshPaused)
        #expect(await counters.quotaCalls == 0)

        await newStopGate.release()
        _ = await newStopTask?.value
        await newReceipt.value
        #expect(model.geminiOneTimeRecoveryState == .loginFailed)
        #expect(model.isGeminiRefreshPaused)
        #expect(await counters.quotaCalls == 0)
    }

    @Test("Confirmed CLI exec is recorded as launch, separately from login completion")
    @MainActor
    func recordsCLIExecConfirmationAsLaunchStage() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let diagnosticStore = GeminiDiagnosticStore(fileURL: nil)
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 22)),
            openLogin: { _ in },
            recoveryTimeout: .seconds(2),
            execConfirmed: { _ in true },
            diagnosticStore: diagnosticStore
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        try await Task.sleep(for: .milliseconds(150))
        let records = await diagnosticStore.records()

        #expect(records.filter { $0.stage == .launch && $0.category == .execConfirmed }.count == 1)
        #expect(!records.contains { $0.stage == .login && $0.category == .succeeded })
        #expect(model.isGeminiRefreshPaused)
    }

    @Test("Stopping during authorization invalidates the successful login before quota collection")
    @MainActor
    func stopDuringAuthorizationCannotStartQuotaCollection() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let authorizationGate = RecoveryAuthorizationGate()
        var openedToken: String?
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 22)),
            openLogin: { openedToken = $0 },
            authorizeLogin: { _ in await authorizationGate.authorize() }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        let completion = Task {
            await model.completeGeminiInteractiveSignIn(token: token, result: .success)
        }
        await authorizationGate.waitUntilAuthorizationStarted()

        model.stop()
        await authorizationGate.releaseAuthorization()
        await completion.value

        #expect(await counters.quotaCalls == 0)
        #expect(model.geminiOneTimeRecoveryState == .idle)
        #expect(model.geminiPauseReason == .unknown)
        #expect(model.isGeminiRefreshPaused)
    }

    @Test("Application stop invalidates the token before any later callback")
    @MainActor
    func appStopInvalidatesPendingReceipt() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        var openedToken: String?
        var stoppedTokens: [String] = []
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 1)),
            openLogin: { openedToken = $0 },
            stopLogin: { stoppedTokens.append($0) }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        model.stop()
        await model.completeGeminiInteractiveSignIn(token: token, result: .success)

        #expect(await counters.quotaCalls == 0)
        #expect(model.geminiOneTimeRecoveryState == .idle)
        #expect(stoppedTokens == [token])
    }

    @Test("Application stop during recovery authorization does not launch a late login")
    @MainActor
    func appStopDuringRecoveryBeginDoesNotLaunchLogin() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        var openedToken: String?
        var stoppedTokens: [String] = []
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 1)),
            openLogin: { openedToken = $0 },
            stopLogin: { stoppedTokens.append($0) },
            recoveryBegin: {
                try? await Task.sleep(for: .milliseconds(40))
                return true
            }
        )

        await model.refresh()
        let begin = Task { await model.beginGeminiOneTimeRecovery() }
        try await Task.sleep(for: .milliseconds(5))
        model.stop()
        await begin.value

        #expect(openedToken == nil)
        #expect(stoppedTokens.count == 1)
        #expect(model.geminiOneTimeRecoveryState == .idle)
        #expect(await counters.quotaCalls == 0)
    }

    @Test("A refresh already in flight cannot replace a newer one-time Gemini result")
    @MainActor
    func inFlightRefreshPreservesRecoveredGeminiSnapshot() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
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
        context: (suiteName: String, defaults: UserDefaults, cacheDirectoryURL: URL),
        counters: RecoveryCounters,
        pauseReason: GeminiPauseReason,
        quota: Result<UsageSnapshot, UsageCollectionError>,
        openLogin: @escaping (String) -> Void,
        cancelLogin: @escaping (String) -> Void = { _ in },
        stopLogin: @escaping (String) -> Void = { _ in },
        stopOperation: ((String) -> Task<Bool, Never>)? = nil,
        stopAcknowledged: Bool = true,
        recoveryTimeout: Duration = .seconds(310),
        openShouldFail: Bool = false,
        terminalFailure: @escaping (String) -> Bool = { _ in false },
        cleanupUncertain: @escaping (String) -> Bool = { _ in false },
        execConfirmed: @escaping (String) -> Bool = { _ in true },
        authorizeLogin: (@Sendable (String) async -> GeminiQuotaRecoveryAuthorization?)? = nil,
        diagnosticStore: GeminiDiagnosticStore = GeminiDiagnosticStore(fileURL: nil),
        refreshOperation: (@Sendable () async -> [UsageSnapshot])? = nil,
        recoveryBegin: (@Sendable () async -> Bool)? = nil,
        signInSleep: (@Sendable (Duration) async throws -> Void)? = nil
    ) -> AppModel {
        AppModel(
            defaults: context.defaults,
            launchAtLoginStatusOperation: { false },
            widgetSnapshotPublisher: nil,
            isDemoMode: false,
            geminiDiagnosticStore: diagnosticStore,
            refreshOperation: refreshOperation ?? {
                await counters.incrementGlobalRefresh()
                return []
            },
            geminiPauseReasonOperation: { pauseReason },
            geminiRecoveryBeginOperation: { _ in
                if let recoveryBegin { return await recoveryBegin() }
                return true
            },
            geminiRecoveryAuthorizeOperation: authorizeLogin ?? { _ in GeminiQuotaRecoveryAuthorization(id: UUID()) },
            geminiQuotaCheckOperation: { _ in
                await counters.incrementQuota()
                return try quota.get()
            },
            geminiAuthenticationOpenOperation: { token in
                if openShouldFail { throw CocoaError(.fileWriteUnknown) }
                openLogin(token)
            },
            geminiAuthenticationCancelOperation: cancelLogin,
            geminiAuthenticationStopOperation: stopOperation ?? { token in
                Task {
                    stopLogin(token)
                    return stopAcknowledged
                }
            },
            geminiAuthenticationFailureOperation: terminalFailure,
            geminiAuthenticationCleanupUncertainOperation: cleanupUncertain,
            geminiAuthenticationExecConfirmedOperation: execConfirmed,
            geminiRecoveryTimeout: recoveryTimeout,
            cacheDirectoryURL: context.cacheDirectoryURL,
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

    private func makeRecoveryContext() -> (suiteName: String, defaults: UserDefaults, cacheDirectoryURL: URL) {
        let suiteName = "AppModelGeminiRecoveryTests.\(UUID().uuidString)"
        let cacheDirectoryURL = FileManager.default.temporaryDirectory
            .appendingPathComponent(suiteName, isDirectory: true)
        try! FileManager.default.createDirectory(at: cacheDirectoryURL, withIntermediateDirectories: true)
        return (suiteName, UserDefaults(suiteName: suiteName)!, cacheDirectoryURL)
    }

    private func clean(_ context: (suiteName: String, defaults: UserDefaults, cacheDirectoryURL: URL)) {
        context.defaults.removePersistentDomain(forName: context.suiteName)
        try? FileManager.default.removeItem(at: context.cacheDirectoryURL)
    }
}

private actor RecoveryAuthorizationGate {
    private var hasStarted = false
    private var isReleased = false
    private var startedContinuation: CheckedContinuation<Void, Never>?
    private var releaseContinuation: CheckedContinuation<Void, Never>?

    func authorize() async -> GeminiQuotaRecoveryAuthorization {
        hasStarted = true
        startedContinuation?.resume()
        startedContinuation = nil
        if !isReleased {
            await withCheckedContinuation { releaseContinuation = $0 }
        }
        return GeminiQuotaRecoveryAuthorization(id: UUID())
    }

    func waitUntilAuthorizationStarted() async {
        guard !hasStarted else { return }
        await withCheckedContinuation { startedContinuation = $0 }
    }

    func releaseAuthorization() {
        isReleased = true
        releaseContinuation?.resume()
        releaseContinuation = nil
    }
}

private actor RecoveryStopGate {
    private var hasStarted = false
    private var isReleased = false
    private var startedContinuation: CheckedContinuation<Void, Never>?
    private var releaseContinuations: [CheckedContinuation<Void, Never>] = []

    func waitUntilReleased() async {
        hasStarted = true
        startedContinuation?.resume()
        startedContinuation = nil
        if !isReleased {
            await withCheckedContinuation { releaseContinuations.append($0) }
        }
    }

    func waitUntilStopStarted() async {
        guard !hasStarted else { return }
        await withCheckedContinuation { startedContinuation = $0 }
    }

    func release() {
        isReleased = true
        let continuations = releaseContinuations
        releaseContinuations.removeAll()
        continuations.forEach { $0.resume() }
    }
}

private actor RecoveryCounters {
    private(set) var quotaCalls = 0
    private(set) var globalRefreshCalls = 0

    func incrementQuota() { quotaCalls += 1 }
    func incrementGlobalRefresh() { globalRefreshCalls += 1 }
}

private actor RecoveryTimeoutGate {
    enum Observation: Equatable, Sendable { case started, completed, timedOut }
    private var continuation: CheckedContinuation<Void, Error>?
    private var hasStarted = false
    private var released = false
    private var cancelled = false

    func sleep(duration: Duration) async throws {
        guard !cancelled else { throw CancellationError() }
        hasStarted = true
        if !released {
            try await withTaskCancellationHandler {
                try await withCheckedThrowingContinuation { continuation = $0 }
            } onCancel: {
                Task { await self.cancel() }
            }
        }
        guard !cancelled else { throw CancellationError() }
        // The explicit gate controls entry; elapsed time must still honor the
        // actual duration requested by the combined monotonic deadline monitor.
        try await Task.sleep(for: duration)
    }

    func waitForStartOrCompletion(
        completed: @escaping @MainActor @Sendable () -> Bool, timeout: Duration
    ) async -> Observation {
        let deadline = ContinuousClock.now.advanced(by: timeout)
        repeat {
            if await completed() { return .completed }
            if hasStarted { return .started }
            guard !cancelled, !Task.isCancelled, ContinuousClock.now < deadline else { return .timedOut }
            await Task.yield()
        } while true
    }

    func expire() {
        released = true
        continuation?.resume()
        continuation = nil
    }

    func cancel() {
        cancelled = true
        continuation?.resume(throwing: CancellationError())
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
