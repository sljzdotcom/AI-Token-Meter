import Testing
@testable import AIMeterApp

@Suite("Floating panel positioning policy")
struct FloatingPanelPositioningPolicyTests {
    @Test("Active Space repositioning preserves a missing screen's saved placement")
    func activeSpaceChangePreservesSavedPlacement() {
        let action = FloatingStripRecoveryPolicy.action(
            for: .activeSpaceChange,
            usesDefaultPlacement: true
        )

        #expect(action == .preserveSavedPlacement)
    }

    @Test("Ordinary repositioning recovers a missing saved screen")
    func ordinaryRepositioningRecoversMissingScreen() {
        let action = FloatingStripRecoveryPolicy.action(
            for: .ordinary,
            usesDefaultPlacement: true
        )

        #expect(action == .persistScreenLossRecovery)
    }
}
