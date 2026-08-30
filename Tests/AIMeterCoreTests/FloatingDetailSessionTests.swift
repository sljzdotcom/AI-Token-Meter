import CoreGraphics
import Testing
@testable import AIMeterCore

@Suite("Floating detail session", .serialized)
struct FloatingDetailSessionTests {
    @Test("Accessibility reports whether each provider detail is open")
    @MainActor
    func accessibilityValueTracksSelection() {
        let session = FloatingDetailSession()

        #expect(session.accessibilityValue(for: .codex) == "Detail closed")
        session.present(.codex, autoHideAfter: .seconds(30))
        #expect(session.accessibilityValue(for: .codex) == "Detail open")
        #expect(session.accessibilityValue(for: .claude) == "Detail closed")
        session.dismiss()
        #expect(session.accessibilityValue(for: .codex) == "Detail closed")
    }

    @Test("Tapping providers toggles and switches one shared selection")
    @MainActor
    func togglesSelection() {
        let session = FloatingDetailSession()
        session.toggle(.claude, autoHideAfter: .seconds(30))
        #expect(session.selectedProvider == .claude)
        session.toggle(.codex, autoHideAfter: .seconds(30))
        #expect(session.selectedProvider == .codex)
        session.toggle(.codex, autoHideAfter: .seconds(30))
        #expect(session.selectedProvider == nil)
    }

    @Test("A stale timeout cannot dismiss a newly selected provider")
    @MainActor
    func staleTimeoutDoesNotDismissNewSelection() async throws {
        let session = FloatingDetailSession()
        session.present(.claude, autoHideAfter: .milliseconds(20))
        try await Task.sleep(for: .milliseconds(10))
        session.present(.codex, autoHideAfter: .seconds(1))
        try await Task.sleep(for: .milliseconds(30))
        #expect(session.selectedProvider == .codex)
        session.dismiss()
    }

    @Test("A stale timeout cannot dismiss a refreshed selection for the same provider")
    @MainActor
    func staleTimeoutDoesNotDismissRefreshedSelection() async {
        let session = FloatingDetailSession()
        session.present(.claude, autoHideAfter: .milliseconds(20))
        await Task.yield()

        let deadline = ContinuousClock.now.advanced(by: .milliseconds(30))
        while ContinuousClock.now < deadline {}

        session.present(.claude, autoHideAfter: .seconds(1))
        await Task.yield()
        #expect(session.selectedProvider == .claude)
        session.dismiss()
    }

    @Test("The active provider is cleared when its timeout expires")
    @MainActor
    func timeoutDismisses() async throws {
        let session = FloatingDetailSession()
        session.present(.deepSeek, autoHideAfter: .milliseconds(20))
        try await Task.sleep(for: .milliseconds(50))
        #expect(session.selectedProvider == nil)
    }

    @Test("Only clicks outside both panels request dismissal")
    func hitPolicy() {
        let strip = CGRect(x: 300, y: 100, width: 84, height: 300)
        let detail = CGRect(x: 28, y: 138, width: 262, height: 224)
        #expect(!FloatingPanelHitPolicy.isOutside(CGPoint(x: 320, y: 150), strip: strip, detail: detail))
        #expect(!FloatingPanelHitPolicy.isOutside(CGPoint(x: 100, y: 200), strip: strip, detail: detail))
        #expect(FloatingPanelHitPolicy.isOutside(CGPoint(x: 10, y: 10), strip: strip, detail: detail))
    }

    @Test("A delayed outside click uses its captured point and selection")
    @MainActor
    func delayedOutsideClickUsesCapturedState() throws {
        let strip = CGRect(x: 300, y: 100, width: 84, height: 300)
        let detail = CGRect(x: 28, y: 138, width: 262, height: 224)
        let session = FloatingDetailSession()
        session.present(.claude, autoHideAfter: .seconds(30))

        let clickedSelection = try #require(session.selectionID)
        let request = FloatingPanelDismissalRequest(
            screenPoint: CGPoint(x: 10, y: 10),
            eventTimestamp: .greatestFiniteMagnitude,
            selectionID: clickedSelection
        )

        let laterPointerPosition = CGPoint(x: 100, y: 200)
        #expect(!FloatingPanelHitPolicy.isOutside(laterPointerPosition, strip: strip, detail: detail))
        #expect(request.requestsDismissal(
            currentSelectionID: clickedSelection,
            strip: strip,
            detail: detail
        ))

        session.present(.claude, autoHideAfter: .seconds(30))
        #expect(!request.requestsDismissal(
            currentSelectionID: session.selectionID,
            strip: strip,
            detail: detail
        ))
        session.dismiss(ifCurrent: request.selectionID)
        #expect(session.selectedProvider == .claude)
        session.dismiss()
    }

    @Test("A globally delayed click cannot target a selection presented after the click")
    @MainActor
    func globallyDelayedClickCannotTargetNewSelection() throws {
        let strip = CGRect(x: 300, y: 100, width: 84, height: 300)
        let detail = CGRect(x: 28, y: 138, width: 262, height: 224)
        let session = FloatingDetailSession()
        session.present(.claude, autoHideAfter: .seconds(30))

        let newSelection = try #require(session.selectionID)
        let delayedRequest = FloatingPanelDismissalRequest(
            screenPoint: CGPoint(x: 10, y: 10),
            eventTimestamp: 0,
            selectionID: newSelection
        )

        #expect(!delayedRequest.requestsDismissal(
            currentSelectionID: session.selectionID,
            strip: strip,
            detail: detail
        ))
        session.dismiss()
    }

    @Test("Shutdown immediately cancels auto-hide and invalidates the session")
    @MainActor
    func shutdownCancelsAndInvalidates() async {
        let probe = AutoHideSleepProbe()
        let session = FloatingDetailSession { duration in
            try await probe.sleep(for: duration)
        }
        var selections: [UsageProvider?] = []
        session.onSelectionChange = { selections.append($0) }
        session.present(.claude, autoHideAfter: .seconds(30))

        #expect(await eventually { await probe.hasStarted })
        session.shutdown()

        #expect(session.selectedProvider == nil)
        #expect(selections == [.claude, nil])
        #expect(await eventually { await probe.wasCancelled })

        session.present(.codex, autoHideAfter: .seconds(30))
        #expect(session.selectedProvider == nil)
        #expect(selections == [.claude, nil])
    }

    @Test("Dismiss cancels auto-hide while keeping the session reusable")
    @MainActor
    func dismissCancelsAndAllowsReuse() async {
        let probe = AutoHideSleepProbe()
        let session = FloatingDetailSession { duration in
            try await probe.sleep(for: duration)
        }
        session.present(.claude, autoHideAfter: .seconds(30))

        #expect(await eventually { await probe.hasStarted })
        session.dismiss()

        #expect(session.selectedProvider == nil)
        #expect(await eventually { await probe.wasCancelled })
        session.present(.codex, autoHideAfter: .seconds(30))
        #expect(session.selectedProvider == .codex)
        session.dismiss()
    }

    @Test("Shutdown reentered from selection callback prevents auto-hide from starting")
    @MainActor
    func callbackShutdownPreventsTaskStart() async {
        let probe = AutoHideSleepProbe()
        let session = FloatingDetailSession { duration in
            try await probe.sleep(for: duration)
        }
        session.onSelectionChange = { provider in
            if provider != nil {
                session.shutdown()
            }
        }

        session.present(.claude, autoHideAfter: .seconds(30))
        for _ in 0..<10 {
            await Task.yield()
        }

        #expect(session.selectedProvider == nil)
        #expect(!(await probe.hasStarted))
    }
}

private actor AutoHideSleepProbe {
    private(set) var hasStarted = false
    private(set) var wasCancelled = false

    func sleep(for duration: Duration) async throws {
        hasStarted = true
        do {
            try await Task.sleep(for: duration)
        } catch {
            wasCancelled = true
            throw error
        }
    }
}

private func eventually(
    _ condition: @escaping @Sendable () async -> Bool
) async -> Bool {
    for _ in 0..<1_000 {
        if await condition() { return true }
        await Task.yield()
    }
    return false
}
