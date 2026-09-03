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

    @Test("A Settings edge change binds first launch to the resolved display")
    func settingsEdgeChangeBindsFirstLaunchDisplay() {
        let position = FloatingStripPosition(
            preference: .left,
            lastResolvedEdge: .left,
            normalizedCenterY: 0.5,
            screenIdentifier: nil
        )
        let resolution = FloatingStripScreenResolution(
            selectedIdentifier: "uuid:primary",
            usesFallbackScreen: false,
            migratedIdentifier: nil
        )

        let placement = FloatingStripPositionPersistencePolicy.resolvedPlacement(
            position: position,
            resolution: resolution,
            userInitiated: true
        )

        #expect(placement.persistenceAction == .save(to: "uuid:primary"))
    }

    @Test("A Settings edge change replaces an offline target with the fallback display")
    func settingsEdgeChangeRebindsFallbackDisplay() {
        let position = FloatingStripPosition(
            preference: .right,
            lastResolvedEdge: .right,
            normalizedCenterY: 0.48,
            screenIdentifier: "uuid:offline"
        )
        let resolution = FloatingStripScreenResolution(
            selectedIdentifier: "uuid:primary",
            usesFallbackScreen: true,
            migratedIdentifier: nil
        )

        let placement = FloatingStripPositionPersistencePolicy.resolvedPlacement(
            position: position,
            resolution: resolution,
            userInitiated: true
        )

        #expect(placement.edge == .right)
        #expect(placement.normalizedCenterY == 0.48)
        #expect(placement.persistenceAction == .save(to: "uuid:primary"))
    }
}
