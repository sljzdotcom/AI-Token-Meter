import AIMeterCore
import Testing
@testable import AIMeterApp

@Suite("Floating panel positioning policy")
struct FloatingPanelPositioningPolicyTests {
    @Test("System repositioning never persists a fallback display")
    func systemRepositioningPreservesSavedPlacement() {
        let resolution = FloatingStripScreenResolution(
            selectedIdentifier: "uuid:primary",
            usesFallbackScreen: true,
            migratedIdentifier: nil
        )
        let action = FloatingStripPositionPersistencePolicy.action(
            savedIdentifier: "uuid:missing",
            resolution: resolution
        )

        #expect(action == .preserve)
    }

    @Test("A fallback screen keeps the saved side and vertical position")
    func fallbackScreenKeepsSavedPlacement() {
        let position = FloatingStripPosition(
            preference: .automatic,
            lastResolvedEdge: .left,
            normalizedCenterY: 0.48,
            screenIdentifier: "uuid:missing"
        )
        let resolution = FloatingStripScreenResolution(
            selectedIdentifier: "uuid:primary",
            usesFallbackScreen: true,
            migratedIdentifier: nil
        )

        let placement = FloatingStripPositionPersistencePolicy.resolvedPlacement(
            position: position,
            resolution: resolution
        )

        #expect(placement.edge == .left)
        #expect(placement.normalizedCenterY == 0.48)
        #expect(placement.persistenceAction == .preserve)
    }

    @Test("A legacy display match migrates only its identifier")
    func legacyDisplayMatchMigratesIdentifier() {
        let resolution = FloatingStripScreenResolution(
            selectedIdentifier: "uuid:display",
            usesFallbackScreen: false,
            migratedIdentifier: "uuid:display"
        )
        let action = FloatingStripPositionPersistencePolicy.action(
            savedIdentifier: "3",
            resolution: resolution
        )

        #expect(action == .migrate(from: "3", to: "uuid:display"))
    }
}
