import Foundation
import AppKit
import Testing
import AIMeterCore
@testable import AIMeterApp

@Suite("Display settings integration")
@MainActor
struct AppModelDisplaySettingsTests {
    @Test func migratingSelectedIdentityDoesNotDependOnLastEditedDisplay() throws {
        let suite = "IndependentMigration-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        FloatingStripPositionStore(defaults: defaults).save(
            .init(lastResolvedEdge: .left, normalizedCenterY: 0.3, screenIdentifier: "77"))
        let model = AppModel(defaults: defaults, secretStore: DisplaySettingsSecretStore(), widgetSnapshotPublisher: nil, isDemoMode: true)
        model.saveFloatingStripPlacement(edge: .right, normalizedCenterY: 0.6, screenIdentifier: "other")
        model.migrateFloatingStripScreenIdentifier(from: "77", to: "uuid:external")
        #expect(model.floatingStripDisplays.selectedIdentifier == "uuid:external")
        #expect(model.floatingStripDisplays.placements["77"] == nil)
        #expect(model.floatingStripDisplays.placement(for: "uuid:external").normalizedCenterY == 0.3)
        #expect(model.floatingStripDisplays.placement(for: "other").normalizedCenterY == 0.6)
        #expect(model.floatingStripPosition.screenIdentifier == "other")
    }
    @Test(.enabled(if: ProcessInfo.processInfo.environment["AI_METER_SCREEN_TESTS"] == "1"))
    func fixedEdgeSettingsDoNotEraseAutomaticPosition() throws {
        let suite = "FixedEdge-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let screen = try #require(NSScreen.screens.first)
        let id = try #require(FloatingStripScreenIdentifier.identity(for: screen, mainScreen: screen)).stableIdentifier
        let model = AppModel(defaults: defaults, secretStore: DisplaySettingsSecretStore(), widgetSnapshotPublisher: nil, isDemoMode: true)
        model.saveFloatingStripPlacement(edge: .right, normalizedCenterY: 0.4, screenIdentifier: id)
        let controller = FloatingPanelController(model: model, screenIdentifier: id)
        defer { controller.close() }
        model.setFloatingStripEdgePreference(.left)
        controller.applyUserPositionPreference()
        model.setFloatingStripEdgePreference(.automatic)
        controller.applyUserPositionPreference()
        #expect(model.floatingStripDisplays.placement(for: id).edge == .right)
        #expect(model.floatingStripDisplays.mode == .primary)
    }

    @Test(.enabled(if: ProcessInfo.processInfo.environment["AI_METER_SCREEN_TESTS"] == "1"))
    func changingEdgeOnFallbackDisplayMakesItTheExplicitTarget() throws {
        let suite = "FallbackEdge-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let screen = try #require(NSScreen.screens.first)
        let identity = try #require(FloatingStripScreenIdentifier.identity(for: screen, mainScreen: screen))
        let model = AppModel(defaults: defaults, secretStore: DisplaySettingsSecretStore(), widgetSnapshotPublisher: nil, isDemoMode: true)
        model.saveFloatingStripPlacement(edge: .right, normalizedCenterY: 0.7, screenIdentifier: "offline")
        model.selectFloatingStripDisplay("offline")
        let controller = FloatingPanelController(model: model, screenIdentifier: identity.stableIdentifier,
            onPlacementSaved: { identifier, intent in
                if model.floatingStripDisplays.shouldSelectTarget(after: intent, actualIdentifier: identifier) {
                    model.selectFloatingStripDisplay(identifier)
                }
            })
        defer { controller.close() }
        model.setFloatingStripEdgePreference(.left)
        controller.applyUserPositionPreference()
        #expect(model.floatingStripDisplays.selectedIdentifier == identity.stableIdentifier)
        #expect(model.floatingStripDisplays.placement(for: identity.stableIdentifier, preference: .left).edge == .left)
        #expect(model.floatingStripDisplays.placement(for: "offline").edge == .right)
    }

    @Test func choosingDisplayAndModePersistsWithoutLosingPositions() throws {
        let suite = "DisplaySettings-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let model = AppModel(defaults: defaults, secretStore: DisplaySettingsSecretStore(), widgetSnapshotPublisher: nil, isDemoMode: true)
        model.updateAvailableStripDisplays([
            .init(id: "a", name: "Built in", isPrimary: true, isBuiltIn: true),
            .init(id: "b", name: "External", isPrimary: false, isBuiltIn: false),
        ])
        model.saveFloatingStripPlacement(edge: .left, normalizedCenterY: 0.2, screenIdentifier: "a")
        model.selectFloatingStripDisplay("a")
        model.saveFloatingStripPlacement(edge: .right, normalizedCenterY: 0.7, screenIdentifier: "b")
        model.selectFloatingStripDisplay("b")
        model.setFloatingStripDisplayMode(.all)
        #expect(FloatingStripDisplaysStore(defaults: defaults).load().mode == .all)
        model.setFloatingStripDisplayMode(.primary)
        let restored = FloatingStripDisplaysStore(defaults: defaults).load()
        #expect(restored.mode == .primary)
        #expect(restored.placement(for: "a").normalizedCenterY == 0.2)
        #expect(restored.placement(for: "b").normalizedCenterY == 0.7)
        // Offline enumeration is only presentation metadata; never rewrites selection.
        model.selectFloatingStripDisplay("b")
        model.updateAvailableStripDisplays([.init(id: "a", name: "Built in", isPrimary: true, isBuiltIn: true)])
        #expect(model.floatingStripDisplays.selectedIdentifier == "b")
    }
}

private struct DisplaySettingsSecretStore: SecretStore {
    func read() throws -> String? { nil }
    func save(_ secret: String) throws {}
    func delete() throws {}
}
