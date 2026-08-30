import CoreGraphics
import Testing
@testable import AIMeterCore

@Suite("Floating detail session", .serialized)
struct FloatingDetailSessionTests {
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
}
