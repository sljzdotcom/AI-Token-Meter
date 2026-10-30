import Foundation
import Testing
@testable import AIMeterApp
@testable import AIMeterCore

@Suite("App model Antigravity one-time recovery", .serialized)
struct AppModelGeminiRecoveryTests {
    @Test("A matching fresh quota check offers explicit resume without automatically unpausing")
    @MainActor
    func freshQuotaOffersExplicitResumeWithoutAutomaticallyUnpausing() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let quotaAuthorization = GeminiQuotaRecoveryAuthorization(id: UUID())
        let resumeAuthorization = GeminiAutomaticRefreshResumeAuthorization(id: UUID())
        let resumeProbe = RecoveryResumeProbe(authorization: resumeAuthorization)
        var openedToken: String?
        let freshQuota = recoverySnapshot(value: 22, sourceVersion: "1.1.28")
        let model = makeModel(
            context: context, counters: counters, pauseReason: .unknown,
            quota: .success(freshQuota), openLogin: { openedToken = $0 },
            authorizeLogin: { _ in quotaAuthorization },
            authorizeResume: { await resumeProbe.authorize(for: $0) },
            resumeAutomaticRefresh: { authorization in
                await resumeProbe.recordResume(authorization)
                return freshQuota
            }
        )

        await model.refresh()
        #expect(!model.canResumeGeminiAutomaticRefresh)
        await model.resumeGeminiAutomaticRefresh()
        #expect(await resumeProbe.resumeAuthorizations.isEmpty)
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        await model.completeGeminiInteractiveSignIn(token: token, result: .success)

        #expect(await resumeProbe.quotaAuthorizations == [quotaAuthorization])
        #expect(model.canResumeGeminiAutomaticRefresh)
        #expect(model.geminiOneTimeRecoveryState == .succeeded)
        #expect(model.geminiPauseReason == .unknown)
        #expect(model.isGeminiRefreshPaused)
        #expect(await resumeProbe.resumeAuthorizations.isEmpty)
        #expect(await counters.quotaCalls == 1)
        #expect(await counters.globalRefreshCalls == 1)
        #expect(model.snapshots.first(where: { $0.provider == .gemini }) == freshQuota)
    }

    @Test("Only a successful explicit resume clears pause, with one submit and no immediate refresh", arguments: [true, false])
    @MainActor
    func explicitResumePublishesOnlyAfterSuccessfulCommit(commitSucceeds: Bool) async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let quotaAuthorization = GeminiQuotaRecoveryAuthorization(id: UUID())
        let resumeAuthorization = GeminiAutomaticRefreshResumeAuthorization(id: UUID())
        let resumeProbe = RecoveryResumeProbe(authorization: resumeAuthorization)
        defer { Task { await resumeProbe.release() } }
        var openedToken: String?
        let freshQuota = recoverySnapshot(value: 83, sourceVersion: "1.1.28")
        let model = makeModel(
            context: context, counters: counters, pauseReason: .unknown,
            quota: .success(freshQuota), openLogin: { openedToken = $0 },
            authorizeLogin: { _ in quotaAuthorization },
            authorizeResume: { await resumeProbe.authorize(for: $0) },
            resumeAutomaticRefresh: { authorization in
                await resumeProbe.submitAndWait(authorization)
                if !commitSucceeds { throw GeminiAutomaticRefreshResumeError.persistenceFailed }
                return freshQuota
            }
        )

        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        await model.completeGeminiInteractiveSignIn(token: token, result: .success)
        #expect(model.isGeminiRefreshPaused)
        #expect(await resumeProbe.resumeAuthorizations.isEmpty)

        let resume = Task { await model.resumeGeminiAutomaticRefresh() }
        let didStart = await resumeProbe.waitUntilStarted(timeout: .seconds(2))
        if !didStart {
            await resumeProbe.release()
            await resume.value
        }
        try #require(didStart, "The explicit action must invoke the injected resume operation")
        #expect(model.isGeminiRefreshPaused)
        #expect(model.geminiPauseReason == .unknown)
        #expect(!model.canResumeGeminiAutomaticRefresh)
        let duplicateResume = Task {
            await model.resumeGeminiAutomaticRefresh()
            await resumeProbe.markDuplicateCompleted()
        }
        let duplicateCompleted = await resumeProbe.waitUntilDuplicateCompleted(timeout: .seconds(2))
        // A missing duplicate guard must fail after releasing both submissions,
        // rather than deadlock the test while the first operation is held.
        if !duplicateCompleted {
            await resumeProbe.release()
            await duplicateResume.value
            await resume.value
        }
        #expect(duplicateCompleted, "A duplicate action must return without waiting for the held resume operation")
        #expect(await resumeProbe.resumeAuthorizations == [resumeAuthorization])
        #expect(await counters.quotaCalls == 1)
        #expect(await counters.globalRefreshCalls == 1)

        await resumeProbe.release()
        await duplicateResume.value
        await resume.value
        #expect(model.isGeminiRefreshPaused == !commitSucceeds)
        #expect(model.geminiPauseReason == (commitSucceeds ? nil : .unknown))
        #expect(!model.canResumeGeminiAutomaticRefresh)
        #expect(model.snapshots.first(where: { $0.provider == .gemini }) == freshQuota)
        #expect(model.settingsNotice == (commitSucceeds ? .recoveryResumeSucceeded : .recoveryResumePersistenceFailed))
        #expect(await resumeProbe.resumeAuthorizations == [resumeAuthorization])
        #expect(await counters.quotaCalls == 1)
        #expect(await counters.globalRefreshCalls == 1)
    }

    @Test("Busy resume keeps the same expiring offer for a later explicit retry")
    @MainActor
    func busyResumeRequiresAnotherExplicitTap() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let authorization = GeminiAutomaticRefreshResumeAuthorization(id: UUID())
        let probe = RecoveryResumeProbe(authorization: authorization)
        let quota = recoverySnapshot(value: 22, sourceVersion: "1.1.28")
        var token: String?
        let model = makeModel(
            context: context, counters: counters, pauseReason: .unknown,
            quota: .success(quota), openLogin: { token = $0 },
            authorizeResume: { await probe.authorize(for: $0) },
            resumeAutomaticRefresh: {
                if await probe.recordResume($0) == 1 { throw GeminiAutomaticRefreshResumeError.alreadyRunning }
                return quota
            }
        )
        defer { model.stop() }
        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        await model.completeGeminiInteractiveSignIn(token: try #require(token), result: .success)
        await model.resumeGeminiAutomaticRefresh()
        #expect(model.isGeminiRefreshPaused)
        #expect(model.canResumeGeminiAutomaticRefresh)
        #expect(model.settingsNotice == .recoveryResumeBusy)
        #expect(await probe.resumeAuthorizations == [authorization])
        await Task.yield()
        #expect(await probe.resumeAuthorizations.count == 1)
        await model.resumeGeminiAutomaticRefresh()
        #expect(!model.isGeminiRefreshPaused)
        #expect(model.settingsNotice == .recoveryResumeSucceeded)
        #expect(await probe.resumeAuthorizations == [authorization, authorization])
        #expect(await counters.quotaCalls == 1)
        #expect(await counters.globalRefreshCalls == 1)
    }

    @Test("An in-flight App refresh denies resume through its presentation tail without queuing it")
    @MainActor
    func appRefreshBusyPreservesOfferForExplicitRetry() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let authorization = GeminiAutomaticRefreshResumeAuthorization(id: UUID())
        let probe = RecoveryResumeProbe(authorization: authorization)
        let refreshGate = RecoveryAsyncGate()
        defer { Task { await refreshGate.release() } }
        let quota = recoverySnapshot(value: 22, sourceVersion: "1.1.28")
        var token: String?
        let model = makeModel(
            context: context, counters: counters, pauseReason: .unknown,
            quota: .success(quota), openLogin: { token = $0 },
            authorizeResume: { await probe.authorize(for: $0) },
            resumeAutomaticRefresh: { await probe.recordResume($0); return quota },
            refreshOperation: {
                await counters.incrementGlobalRefresh()
                if await counters.globalRefreshCalls > 1 { await refreshGate.hold() }
                return [quota]
            }
        )
        defer { model.stop() }
        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        await model.completeGeminiInteractiveSignIn(token: try #require(token), result: .success)
        let refresh = Task { await model.refresh(manual: false) }
        let started = await refreshGate.waitUntilStarted(timeout: .seconds(2))
        if !started { await refreshGate.release(); await refresh.value }
        try #require(started)
        await model.resumeGeminiAutomaticRefresh()
        #expect(model.isGeminiRefreshPaused)
        #expect(model.canResumeGeminiAutomaticRefresh)
        #expect(model.settingsNotice == .recoveryResumeBusy)
        #expect(await probe.resumeAuthorizations.isEmpty)
        await refreshGate.release()
        await refresh.value
        #expect(await probe.resumeAuthorizations.isEmpty)
        await model.resumeGeminiAutomaticRefresh()
        #expect(!model.isGeminiRefreshPaused)
        #expect(await probe.resumeAuthorizations == [authorization])
        #expect(await counters.globalRefreshCalls == 2)
    }

    @Test("An expired offer returns to the check action while the panel remains open")
    @MainActor
    func expiryClearsOfferWithoutSubmittingOrRevokingAnotherGeneration() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let authorization = GeminiAutomaticRefreshResumeAuthorization(
            id: UUID(), expiresAt: ContinuousClock.now.advanced(by: .seconds(1))
        )
        let probe = RecoveryResumeProbe(authorization: authorization)
        var token: String?
        let model = makeModel(
            context: context, counters: counters, pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 22, sourceVersion: "1.1.28")),
            openLogin: { token = $0 }, authorizeResume: { await probe.authorize(for: $0) },
            resumeInvalidation: { Issue.record("Local expiry must not revoke a newer Core generation") }
        )
        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        await model.completeGeminiInteractiveSignIn(token: try #require(token), result: .success)
        #expect(model.canResumeGeminiAutomaticRefresh)
        let deadline = ContinuousClock.now.advanced(by: .seconds(2))
        while model.settingsNotice != .recoveryResumeValidationRequired, ContinuousClock.now < deadline {
            await Task.yield()
        }
        #expect(!model.canResumeGeminiAutomaticRefresh)
        #expect(model.settingsNotice == .recoveryResumeValidationRequired)
        #expect(model.isGeminiRefreshPaused)
        await model.resumeGeminiAutomaticRefresh()
        #expect(await probe.resumeAuthorizations.isEmpty)
        #expect(await counters.quotaCalls == 1)
        #expect(await counters.globalRefreshCalls == 1)
    }

    @Test("A committed resume may publish after its local offer deadline passes")
    @MainActor
    func committedResumeDoesNotDependOnStillHeldLocalOffer() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let authorization = GeminiAutomaticRefreshResumeAuthorization(
            id: UUID(), expiresAt: ContinuousClock.now.advanced(by: .seconds(1))
        )
        let probe = RecoveryResumeProbe(authorization: authorization)
        defer { Task { await probe.release() } }
        let quota = recoverySnapshot(value: 22, sourceVersion: "1.1.28")
        var token: String?
        let model = makeModel(
            context: context, counters: counters, pauseReason: .unknown,
            quota: .success(quota), openLogin: { token = $0 },
            authorizeResume: { await probe.authorize(for: $0) },
            resumeAutomaticRefresh: { await probe.submitAndWait($0); return quota }
        )
        defer { model.stop() }
        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        await model.completeGeminiInteractiveSignIn(token: try #require(token), result: .success)
        let resume = Task { await model.resumeGeminiAutomaticRefresh() }
        let started = await probe.waitUntilStarted(timeout: .seconds(2))
        if !started { await probe.release(); await resume.value }
        try #require(started)
        // The injected operation represents a commit that already won; only its
        // UI callback is delayed beyond the local offer's deadline.
        do {
            try await Task.sleep(until: authorization.expiresAt, clock: .continuous)
        } catch {
            await probe.release()
            await resume.value
            throw error
        }
        await Task.yield()
        #expect(!model.canResumeGeminiAutomaticRefresh)
        await probe.release()
        await resume.value
        #expect(!model.isGeminiRefreshPaused)
        #expect(model.settingsNotice == .recoveryResumeSucceeded)
    }

    @Test("Rejected resume clears its offer and keeps pause without exposing raw errors", arguments: [GeminiAutomaticRefreshResumeError.validationRequired, .notPaused])
    @MainActor
    func rejectedResumeRequiresNewQuotaCheck(error: GeminiAutomaticRefreshResumeError) async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let authorization = GeminiAutomaticRefreshResumeAuthorization(id: UUID())
        let probe = RecoveryResumeProbe(authorization: authorization)
        var token: String?
        let model = makeModel(
            context: context, counters: counters, pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 22)), openLogin: { token = $0 },
            authorizeResume: { await probe.authorize(for: $0) },
            resumeAutomaticRefresh: { await probe.recordResume($0); throw error }
        )
        defer { model.stop() }
        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        await model.completeGeminiInteractiveSignIn(token: try #require(token), result: .success)
        await model.resumeGeminiAutomaticRefresh()
        #expect(model.isGeminiRefreshPaused)
        #expect(!model.canResumeGeminiAutomaticRefresh)
        #expect(model.settingsNotice == .recoveryResumeValidationRequired)
        #expect(await probe.resumeAuthorizations == [authorization])
        #expect(await counters.quotaCalls == 1)
        #expect(await counters.globalRefreshCalls == 1)
    }

    @Test("Stop rejects a late resume result, including after a newer recovery begins", arguments: [true, false], [true, false])
    @MainActor
    func stoppedResumeCannotPublishIntoAnotherSession(commitSucceeds: Bool, beginNewRecovery: Bool) async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let authorization = GeminiAutomaticRefreshResumeAuthorization(id: UUID())
        let probe = RecoveryResumeProbe(authorization: authorization)
        defer { Task { await probe.release() } }
        let quota = recoverySnapshot(value: 22, sourceVersion: "1.1.28")
        var tokens: [String] = []
        let model = makeModel(
            context: context, counters: counters, pauseReason: .unknown,
            quota: .success(quota), openLogin: { tokens.append($0) },
            authorizeResume: { await probe.authorize(for: $0) },
            resumeAutomaticRefresh: {
                await probe.submitAndWait($0)
                await counters.recordResumeCancellation(Task.isCancelled)
                if !commitSucceeds { throw GeminiAutomaticRefreshResumeError.persistenceFailed }
                return quota
            }
        )
        defer { model.stop() }
        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        await model.completeGeminiInteractiveSignIn(token: try #require(tokens.first), result: .success)
        let resume = Task { await model.resumeGeminiAutomaticRefresh() }
        let started = await probe.waitUntilStarted(timeout: .seconds(2))
        if !started { await probe.release(); await resume.value }
        try #require(started)
        model.stop()
        if beginNewRecovery { await model.beginGeminiOneTimeRecovery() }
        let stateAfterStop = model.geminiOneTimeRecoveryState
        let noticeAfterStop = model.settingsNotice
        await probe.release()
        await resume.value
        #expect(model.geminiOneTimeRecoveryState == stateAfterStop)
        #expect(model.settingsNotice == noticeAfterStop)
        #expect(model.isGeminiRefreshPaused)
        #expect(!model.canResumeGeminiAutomaticRefresh)
        #expect(!model.isGeminiAutomaticRefreshResumeInProgress)
        #expect(tokens.count == (beginNewRecovery ? 2 : 1))
        #expect(await counters.resumeCancellations == [true])
    }

    @Test("A new recovery waits for a queued stop revocation before registering its token")
    @MainActor
    func newRecoveryWaitsForStoppedSessionRevocation() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let invalidation = RecoveryAsyncGate()
        defer { Task { await invalidation.release() } }
        var tokens: [String] = []
        let model = makeModel(
            context: context, counters: counters, pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 22)), openLogin: { tokens.append($0) },
            resumeInvalidation: { await invalidation.hold() },
            recoveryBegin: { await counters.incrementRecoveryBegin(); return true }
        )
        await model.refresh()
        model.stop()
        let started = await invalidation.waitUntilStarted(timeout: .seconds(2))
        if !started { await invalidation.release() }
        try #require(started)
        let begin = Task { await model.beginGeminiOneTimeRecovery() }
        await Task.yield()
        #expect(await counters.recoveryBeginCalls == 0)
        #expect(tokens.isEmpty)
        await invalidation.release()
        await begin.value
        #expect(await counters.recoveryBeginCalls == 1)
        #expect(tokens.count == 1)
        model.stop()
    }

    @Test("A late capability lookup cannot offer resume in a newer recovery")
    @MainActor
    func lateCapabilityLookupCannotPublishIntoNewRecovery() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let gate = RecoveryAsyncGate()
        defer { Task { await gate.release() } }
        let authorization = GeminiAutomaticRefreshResumeAuthorization(id: UUID())
        var tokens: [String] = []
        let model = makeModel(
            context: context, counters: counters, pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 22)), openLogin: { tokens.append($0) },
            authorizeResume: { _ in await gate.hold(); return authorization }
        )
        defer { model.stop() }
        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(tokens.first)
        let quota = Task { await model.completeGeminiInteractiveSignIn(token: token, result: .success) }
        let started = await gate.waitUntilStarted(timeout: .seconds(2))
        if !started { await gate.release(); await quota.value }
        try #require(started)
        model.stop()
        await model.beginGeminiOneTimeRecovery()
        await gate.release()
        await quota.value
        #expect(model.geminiOneTimeRecoveryState == .awaitingLogin)
        #expect(model.settingsNotice == .recoveryLoginStarted)
        #expect(model.isGeminiRefreshPaused)
        #expect(!model.canResumeGeminiAutomaticRefresh)
    }

    @Test("A newer regular refresh failure wins over a late successful resume publication")
    @MainActor
    func newerRefreshFailureCannotBeClearedByLateResumeSuccess() async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let counters = RecoveryCounters()
        let authorization = GeminiAutomaticRefreshResumeAuthorization(id: UUID())
        let probe = RecoveryResumeProbe(authorization: authorization)
        defer { Task { await probe.release() } }
        let pause = RecoveryPauseState(.unknown)
        let fresh = recoverySnapshot(value: 22, sourceVersion: "1.1.28")
        let later = recoverySnapshot(value: 49)
        var token: String?
        let model = makeModel(
            context: context, counters: counters, pauseReason: .unknown,
            quota: .success(fresh), openLogin: { token = $0 },
            authorizeResume: { await probe.authorize(for: $0) },
            resumeAutomaticRefresh: { await probe.submitAndWait($0); return fresh },
            pauseReasonOperation: { await pause.reason },
            refreshOperation: { await counters.incrementGlobalRefresh(); return [later] }
        )
        defer { model.stop() }
        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        await model.completeGeminiInteractiveSignIn(token: try #require(token), result: .success)
        let resume = Task { await model.resumeGeminiAutomaticRefresh() }
        let started = await probe.waitUntilStarted(timeout: .seconds(2))
        if !started { await probe.release(); await resume.value }
        try #require(started)
        await pause.set(.unsupportedVersion)
        await model.refresh(manual: false)
        await probe.release()
        await resume.value
        #expect(model.geminiPauseReason == .unsupportedVersion)
        #expect(model.isGeminiRefreshPaused)
        #expect(model.snapshots.first(where: { $0.provider == .gemini }) == later)
        #expect(model.settingsNotice != .recoveryResumeSucceeded)
        #expect(!model.canResumeGeminiAutomaticRefresh)
        #expect(await counters.globalRefreshCalls == 2)
    }

    @Test("Account publication uses committed coordinator actions before a delayed resume callback", arguments: [false, true])
    @MainActor
    func accountRefreshPublishesRealResumeActionSet(newerFailure: Bool) async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let quota = resumeIntegrationSnapshot()
        #expect(quota.normalizedAntigravityQuota().geminiQuotaMetrics?.count == 2)
        let collector = ResumeIntegrationCollector(quota: quota)
        let coordinator = RefreshCoordinator(collectors: [collector], cache: SnapshotCache(directoryURL: context.cacheDirectoryURL))
        let callback = RecoveryAsyncGate()
        defer { Task { await callback.release() } }
        var token: String?
        let model = makeCoordinatorModel(context: context, coordinator: coordinator, openLogin: { token = $0 }) {
            let committed = try await coordinator.resumeGeminiAutomaticRefresh(authorization: $0)
            await callback.hold()
            return committed
        }
        defer { model.stop() }
        await model.refresh()
        #expect(await coordinator.providersRequiringAction().contains(.gemini))
        #expect(model.geminiPauseReason == .timeout)
        #expect(model.isGeminiRefreshPaused)
        await model.beginGeminiOneTimeRecovery()
        await model.completeGeminiInteractiveSignIn(token: try #require(token), result: .success)
        try #require(model.canResumeGeminiAutomaticRefresh)
        let resume = Task { await model.resumeGeminiAutomaticRefresh() }
        let started = await callback.waitUntilStarted(timeout: .seconds(2))
        if !started { await callback.release(); await resume.value }
        try #require(started)
        #expect(!(await coordinator.providersRequiringAction().contains(.gemini)))
        #expect(await coordinator.geminiPauseReason() == nil)
        if newerFailure {
            await collector.failQuota()
            _ = await coordinator.refresh(manual: false)
            #expect(await coordinator.providersRequiringAction().contains(.gemini))
            #expect(await coordinator.geminiPauseReason() == .unsupportedVersion)
        }
        await model.refreshServiceAccounts()
        #expect(model.isGeminiRefreshPaused == newerFailure)
        #expect(model.geminiPauseReason == (newerFailure ? .unsupportedVersion : nil))
        await callback.release()
        await resume.value
        #expect(model.isGeminiRefreshPaused == newerFailure)
        #expect(model.geminiPauseReason == (newerFailure ? .unsupportedVersion : nil))
        #expect(!model.canResumeGeminiAutomaticRefresh)
        #expect(model.settingsNotice == (newerFailure ? .recoveryResumeValidationRequired : .recoveryResumeSucceeded))
        #expect(model.serviceAccounts[.gemini]?.refreshPauseReason == (newerFailure ? .unsupportedVersion : nil))
    }

    @Test("A delayed busy callback keeps the original real permit after a harmless account publication", arguments: [false, true])
    @MainActor
    func busyCallbackRevalidatesMatchingPermitAfterAccountPublication(revokeBeforeCallback: Bool) async throws {
        let context = makeRecoveryContext()
        defer { clean(context) }
        let quota = resumeIntegrationSnapshot()
        #expect(quota.normalizedAntigravityQuota().geminiQuotaMetrics?.count == 2)
        let collector = ResumeIntegrationCollector(quota: quota)
        let otherCollector = ResumeIntegrationOtherCollector()
        let coordinator = RefreshCoordinator(collectors: [collector, otherCollector], cache: SnapshotCache(directoryURL: context.cacheDirectoryURL))
        let callback = RecoveryAsyncGate()
        let otherRefreshGate = RecoveryAsyncGate()
        let attempts = ResumeIntegrationAttempts()
        defer { Task { await callback.release() } }
        defer { Task { await otherRefreshGate.release() } }
        var token: String?
        let model = makeCoordinatorModel(context: context, coordinator: coordinator, openLogin: { token = $0 }) {
            let attempt = await attempts.record($0)
            do {
                return try await coordinator.resumeGeminiAutomaticRefresh(authorization: $0)
            } catch {
                if attempt == 1, error as? GeminiAutomaticRefreshResumeError == .alreadyRunning {
                    await callback.hold()
                }
                throw error
            }
        }
        defer { model.stop() }
        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        await model.completeGeminiInteractiveSignIn(token: try #require(token), result: .success)
        try #require(model.canResumeGeminiAutomaticRefresh)
        await otherCollector.holdNextRefresh(otherRefreshGate)
        let backgroundRefresh = Task { await coordinator.refresh(manual: false) }
        let otherStarted = await otherRefreshGate.waitUntilStarted(timeout: .seconds(2))
        if !otherStarted { await otherRefreshGate.release(); _ = await backgroundRefresh.value }
        try #require(otherStarted)
        let resume = Task { await model.resumeGeminiAutomaticRefresh() }
        let started = await callback.waitUntilStarted(timeout: .seconds(2))
        await otherRefreshGate.release()
        _ = await backgroundRefresh.value
        if !started { await callback.release(); await resume.value }
        try #require(started)
        if revokeBeforeCallback { await coordinator.invalidateGeminiAutomaticRefreshResumeAuthorization() }
        await model.refreshServiceAccounts()
        #expect(await coordinator.geminiPauseReason() == .timeout)
        #expect(await coordinator.providersRequiringAction().contains(.gemini))
        await callback.release()
        await resume.value
        #expect(model.canResumeGeminiAutomaticRefresh == !revokeBeforeCallback)
        #expect(model.isGeminiRefreshPaused)
        #expect(await attempts.authorizations.count == 1)
        #expect(model.settingsNotice == (revokeBeforeCallback ? .recoveryResumeValidationRequired : .recoveryResumeBusy))
        if !revokeBeforeCallback {
            await model.resumeGeminiAutomaticRefresh()
            #expect(!model.isGeminiRefreshPaused)
            let authorizations = await attempts.authorizations
            #expect(authorizations.count == 2)
            #expect(authorizations.first == authorizations.last)
            #expect(!(await coordinator.providersRequiringAction().contains(.gemini)))
        }
    }

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
        let stopDeadline = ContinuousClock.now.advanced(by: .seconds(2))
        while model.isGeminiOneTimeRecoveryInProgress, ContinuousClock.now < stopDeadline {
            await Task.yield()
        }
        try #require(!model.isGeminiOneTimeRecoveryInProgress)

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
        let stopDeadline = ContinuousClock.now.advanced(by: .seconds(2))
        while model.isGeminiOneTimeRecoveryInProgress, ContinuousClock.now < stopDeadline {
            await Task.yield()
        }
        try #require(!model.isGeminiOneTimeRecoveryInProgress)

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
        let probe = RecoveryBeginStopProbe()
        let model = makeModel(
            context: context,
            counters: counters,
            pauseReason: .unknown,
            quota: .success(recoverySnapshot(value: 1)),
            openLogin: { probe.open($0) },
            stopOperation: { probe.stop($0) },
            recoveryBeginWithToken: { await probe.begin($0) }
        )

        await model.refresh()
        let begin = Task { await model.beginGeminiOneTimeRecovery() }
        do {
            let beginStarted = await probe.beginGate.waitUntilStarted(timeout: .seconds(2))
            try #require(beginStarted, "Recovery sequence: \(probe.trace)")
            try #require(probe.beginToken != nil, "Recovery sequence: \(probe.trace)")
            let unopenedBeforeStop = probe.openedToken == nil
            #expect(unopenedBeforeStop, "Recovery sequence: \(probe.trace)")

            probe.record("application stop called")
            model.stop()
            probe.record("application stop returned")
            let stopStarted = await probe.stopGate.waitUntilStarted(timeout: .seconds(2))
            try #require(stopStarted, "Recovery sequence: \(probe.trace)")

            probe.record("begin released")
            await probe.beginGate.release()
            await begin.value
            probe.record("begin joined")
            // The actual stop callback is still held, so joining begin cannot
            // accidentally stand in for joining the stop acknowledgement.
            let stopCallbackStillHeld = probe.stoppedTokens.isEmpty
            #expect(stopCallbackStillHeld, "Recovery sequence: \(probe.trace)")
            probe.record("stop released")
            await probe.stopGate.release()
            await probe.joinStops()
            probe.record("stop joined")

            let unopenedAfterStop = probe.openedToken == nil
            let stoppedMatchingBeginToken = probe.beginToken.map { probe.stoppedTokens == [$0] } ?? false
            #expect(unopenedAfterStop, "Recovery sequence: \(probe.trace)")
            #expect(probe.stoppedTokens.count == 1, "Recovery sequence: \(probe.trace)")
            #expect(stoppedMatchingBeginToken, "Recovery sequence: \(probe.trace)")
            #expect(model.geminiOneTimeRecoveryState == .idle, "Recovery sequence: \(probe.trace)")
            #expect(await counters.quotaCalls == 0, "Recovery sequence: \(probe.trace)")
        } catch {
            await probe.releaseAndJoin(begin: begin, model: model)
            throw error
        }
        await probe.releaseAndJoin(begin: begin, model: model)
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
    private func makeCoordinatorModel(
        context: (suiteName: String, defaults: UserDefaults, cacheDirectoryURL: URL),
        coordinator: RefreshCoordinator,
        openLogin: @escaping (String) -> Void,
        resume: @escaping @Sendable (GeminiAutomaticRefreshResumeAuthorization) async throws -> UsageSnapshot
    ) -> AppModel {
        AppModel(
            defaults: context.defaults, secretStore: RecoverySecretStore(), launchAtLoginStatusOperation: { false },
            widgetSnapshotPublisher: nil, isDemoMode: false, geminiDiagnosticStore: GeminiDiagnosticStore(fileURL: nil),
            refreshCoordinator: coordinator, geminiAutomaticRefreshResumeOperation: resume,
            serviceAccountRefreshOperation: { _ in [] }, geminiAuthenticationOpenOperation: openLogin,
            geminiAuthenticationCancelOperation: { _ in }, geminiAuthenticationStopOperation: { _ in Task { true } },
            geminiAuthenticationFailureOperation: { _ in false }, geminiAuthenticationExecConfirmedOperation: { _ in true },
            cacheDirectoryURL: context.cacheDirectoryURL
        )
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
        authorizeResume: @escaping @Sendable (GeminiQuotaRecoveryAuthorization) async -> GeminiAutomaticRefreshResumeAuthorization? = { _ in nil },
        resumeAutomaticRefresh: @escaping @Sendable (GeminiAutomaticRefreshResumeAuthorization) async throws -> UsageSnapshot = { _ in
            throw GeminiAutomaticRefreshResumeError.validationRequired
        },
        resumeInvalidation: @escaping @Sendable () async -> Void = {},
        pauseReasonOperation: (@Sendable () async -> GeminiPauseReason?)? = nil,
        diagnosticStore: GeminiDiagnosticStore = GeminiDiagnosticStore(fileURL: nil),
        refreshOperation: (@Sendable () async -> [UsageSnapshot])? = nil,
        recoveryBegin: (@Sendable () async -> Bool)? = nil,
        recoveryBeginWithToken: (@Sendable (String) async -> Bool)? = nil,
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
            geminiPauseReasonOperation: pauseReasonOperation ?? { pauseReason },
            geminiRecoveryBeginOperation: { token in
                if let recoveryBeginWithToken { return await recoveryBeginWithToken(token) }
                if let recoveryBegin { return await recoveryBegin() }
                return true
            },
            geminiRecoveryAuthorizeOperation: authorizeLogin ?? { _ in GeminiQuotaRecoveryAuthorization(id: UUID()) },
            geminiQuotaCheckOperation: { _ in
                await counters.incrementQuota()
                return try quota.get()
            },
            geminiResumeAuthorizationOperation: authorizeResume,
            geminiAutomaticRefreshResumeOperation: resumeAutomaticRefresh,
            geminiResumeInvalidationOperation: resumeInvalidation,
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

    private func resumeIntegrationSnapshot() -> UsageSnapshot {
        let fiveHour = UsageMetric(
            label: "Gemini · Five hour", current: 22, limit: 100, unit: .percent,
            kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 900)
        )
        let weekly = UsageMetric(
            label: "Gemini · Weekly", current: 35, limit: 100, unit: .percent,
            kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 1_000)
        )
        return UsageSnapshot(
            provider: .gemini, primaryMetric: weekly, secondaryMetric: fiveHour,
            fetchedAt: Date(timeIntervalSince1970: 800), sourceVersion: "1.1.28",
            collectionStatus: .fresh, geminiQuotaMetrics: [fiveHour, weekly]
        )
    }

    private func recoverySnapshot(value: Double, fetchedAt: Date = Date(timeIntervalSince1970: 800), sourceVersion: String? = nil) -> UsageSnapshot {
        let metric = UsageMetric(label: "Gemini · Weekly", current: value, limit: 100, unit: .percent)
        return UsageSnapshot(
            provider: .gemini,
            primaryMetric: metric,
            fetchedAt: fetchedAt,
            sourceVersion: sourceVersion,
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

private struct RecoverySecretStore: SecretStore {
    func read() throws -> String? { nil }
    func save(_ secret: String) throws {}
    func delete() throws {}
}

private actor ResumeIntegrationCollector: UsageCollector, OneTimeGeminiQuotaCollecting {
    nonisolated let provider = UsageProvider.gemini
    private let quota: UsageSnapshot
    private var shouldFailQuota = false
    init(quota: UsageSnapshot) { self.quota = quota }
    func collect() async throws -> UsageSnapshot { throw UsageCollectionError.timedOut }
    func collectQuotaOnce() async throws -> UsageSnapshot {
        if shouldFailQuota { throw UsageCollectionError.unsupportedVersion }
        return quota
    }
    func failQuota() { shouldFailQuota = true }
}

private actor ResumeIntegrationAttempts {
    private(set) var authorizations: [GeminiAutomaticRefreshResumeAuthorization] = []
    func record(_ authorization: GeminiAutomaticRefreshResumeAuthorization) -> Int {
        authorizations.append(authorization)
        return authorizations.count
    }
}

private actor ResumeIntegrationOtherCollector: UsageCollector {
    nonisolated let provider = UsageProvider.claude
    private var nextGate: RecoveryAsyncGate?
    func holdNextRefresh(_ gate: RecoveryAsyncGate) { nextGate = gate }
    func collect() async throws -> UsageSnapshot {
        if let gate = nextGate {
            nextGate = nil
            await gate.hold()
        }
        return UsageSnapshot(provider: .claude, primaryMetric: UsageMetric(label: "Fixture", current: 1, limit: 100, unit: .percent))
    }
}

private actor RecoveryResumeProbe {
    let authorization: GeminiAutomaticRefreshResumeAuthorization
    private(set) var quotaAuthorizations: [GeminiQuotaRecoveryAuthorization] = []
    private(set) var resumeAuthorizations: [GeminiAutomaticRefreshResumeAuthorization] = []
    private var releaseContinuations: [CheckedContinuation<Void, Never>] = []
    private var isReleased = false
    private var duplicateCompleted = false

    init(authorization: GeminiAutomaticRefreshResumeAuthorization) {
        self.authorization = authorization
    }

    func authorize(for quotaAuthorization: GeminiQuotaRecoveryAuthorization) -> GeminiAutomaticRefreshResumeAuthorization {
        quotaAuthorizations.append(quotaAuthorization)
        return authorization
    }

    @discardableResult
    func recordResume(_ authorization: GeminiAutomaticRefreshResumeAuthorization) -> Int {
        resumeAuthorizations.append(authorization)
        return resumeAuthorizations.count
    }

    func submitAndWait(_ authorization: GeminiAutomaticRefreshResumeAuthorization) async {
        recordResume(authorization)
        if !isReleased {
            await withCheckedContinuation { releaseContinuations.append($0) }
        }
    }

    func markDuplicateCompleted() {
        duplicateCompleted = true
    }

    func waitUntilDuplicateCompleted(timeout: Duration) async -> Bool {
        let deadline = ContinuousClock.now.advanced(by: timeout)
        while !duplicateCompleted {
            guard !Task.isCancelled, ContinuousClock.now < deadline else { return false }
            await Task.yield()
        }
        return true
    }

    func waitUntilStarted(timeout: Duration) async -> Bool {
        let deadline = ContinuousClock.now.advanced(by: timeout)
        while resumeAuthorizations.isEmpty {
            guard !Task.isCancelled, ContinuousClock.now < deadline else { return false }
            await Task.yield()
        }
        return true
    }

    func release() {
        isReleased = true
        let continuations = releaseContinuations
        releaseContinuations.removeAll()
        continuations.forEach { $0.resume() }
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
    private(set) var recoveryBeginCalls = 0
    private(set) var resumeCancellations: [Bool] = []

    func incrementQuota() { quotaCalls += 1 }
    func incrementGlobalRefresh() { globalRefreshCalls += 1 }
    func incrementRecoveryBegin() { recoveryBeginCalls += 1 }
    func recordResumeCancellation(_ isCancelled: Bool) { resumeCancellations.append(isCancelled) }
}

private actor RecoveryPauseState {
    private(set) var reason: GeminiPauseReason?
    init(_ reason: GeminiPauseReason?) { self.reason = reason }
    func set(_ reason: GeminiPauseReason?) { self.reason = reason }
}

@MainActor
private final class RecoveryBeginStopProbe {
    let beginGate = RecoveryAsyncGate()
    let stopGate = RecoveryAsyncGate()
    private(set) var beginToken: String?
    private(set) var openedToken: String?
    private(set) var stoppedTokens: [String] = []
    private var stopTasks: [Task<Bool, Never>] = []
    private var events: [String] = []
    var trace: String { events.joined(separator: " -> ") }

    func record(_ event: String) {
        if events.count < 20 { events.append(event) }
    }

    func begin(_ token: String) async -> Bool {
        beginToken = token
        record("begin entered")
        await beginGate.hold()
        record("begin returned true")
        return true
    }

    func open(_ token: String) {
        openedToken = token
        record("login opened")
    }

    func stop(_ token: String) -> Task<Bool, Never> {
        record("stop requested")
        let task = Task {
            self.record("stop entered")
            await self.stopGate.hold()
            self.stoppedTokens.append(token)
            self.record("stop completed")
            return true
        }
        stopTasks.append(task)
        return task
    }

    func joinStops() async {
        for task in stopTasks { _ = await task.value }
    }

    func releaseAndJoin(begin: Task<Void, Never>, model: AppModel) async {
        // Run on both normal and throwing exits, before the context is removed.
        // Release both gates first, including gates not entered before a timeout.
        await beginGate.release()
        await stopGate.release()
        await begin.value
        let timeoutTask = model.stop()
        await timeoutTask?.value
        await joinStops()
    }
}

private actor RecoveryAsyncGate {
    private var hasStarted = false
    private var isReleased = false
    private var continuations: [CheckedContinuation<Void, Never>] = []

    func hold() async {
        hasStarted = true
        if !isReleased { await withCheckedContinuation { continuations.append($0) } }
    }

    func waitUntilStarted(timeout: Duration) async -> Bool {
        let deadline = ContinuousClock.now.advanced(by: timeout)
        while !hasStarted {
            guard !Task.isCancelled, ContinuousClock.now < deadline else { return false }
            await Task.yield()
        }
        return true
    }

    func release() {
        isReleased = true
        let waiting = continuations
        continuations.removeAll()
        waiting.forEach { $0.resume() }
    }
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
