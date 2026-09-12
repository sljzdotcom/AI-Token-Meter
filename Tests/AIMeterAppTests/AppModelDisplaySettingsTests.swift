import Foundation
import AppKit
import Testing
import AIMeterCore
@testable import AIMeterApp

@Suite("Display settings integration")
@MainActor
struct AppModelDisplaySettingsTests {
    @Test func automaticCollapseKeepsContentTimingWithoutASecondGeometryPath() {
        let expanding = FloatingStripVisibilityTransitionPlan.make(
            destination: .expanded,
            reduceMotion: false
        )
        let folding = FloatingStripVisibilityTransitionPlan.make(
            destination: .folded,
            reduceMotion: false
        )

        #expect(expanding.contentDelay == .milliseconds(180))
        #expect(folding.contentDelay == .milliseconds(140))
    }

    @Test func changingDensityPublishesTheSavedWidthSynchronously() throws {
        let suite = "DensityAppearance-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let model = AppModel(defaults: defaults, secretStore: DisplaySettingsSecretStore(), widgetSnapshotPublisher: nil, isDemoMode: true)
        var observedWidth: CGFloat?
        model.floatingAppearanceHandler = { observedWidth = model.stripPreferences.density.width }
        var preferences = model.stripPreferences
        preferences.density = .mini

        model.setStripPreferences(preferences)

        #expect(observedWidth == 65)
        #expect(FloatingStripPreferencesStore(defaults: defaults).load().density == .mini)
    }

    @Test(.enabled(if: ProcessInfo.processInfo.environment["AI_METER_SCREEN_TESTS"] == "1"))
    func densitySettingImmediatelyResizesExpandedPanel() throws {
        let suite = "DensityPanel-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let screen = try #require(NSScreen.screens.first)
        let identifier = try #require(FloatingStripScreenIdentifier.identity(for: screen, mainScreen: screen)).stableIdentifier
        let model = AppModel(defaults: defaults, secretStore: DisplaySettingsSecretStore(), widgetSnapshotPublisher: nil, isDemoMode: true)
        let controller = FloatingPanelController(model: model, screenIdentifier: identifier)
        defer { controller.close() }
        model.floatingAppearanceHandler = { controller.applyAppearance() }
        let before = controller.stripFrameForTesting
        var preferences = model.stripPreferences
        preferences.density = .mini

        model.setStripPreferences(preferences)

        let after = controller.stripFrameForTesting
        #expect(before.width == 78)
        #expect(after.width == 65)
        #expect(abs(before.midY - after.midY) < 0.001)
        #expect(after.minX == screen.visibleFrame.minX || after.maxX == screen.visibleFrame.maxX)
    }

    @Test(.enabled(if: ProcessInfo.processInfo.environment["AI_METER_SCREEN_TESTS"] == "1"))
    func everyDensityImmediatelyResizesTheVisibleExpandedPanel() throws {
        let suite = "VisibleDensityPanel-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let screen = try #require(NSScreen.screens.first)
        let identifier = try #require(
            FloatingStripScreenIdentifier.identity(for: screen, mainScreen: screen)
        ).stableIdentifier
        let model = AppModel(
            defaults: defaults,
            secretStore: DisplaySettingsSecretStore(),
            widgetSnapshotPublisher: nil,
            isDemoMode: true
        )
        var preferences = model.stripPreferences
        preferences.automaticallyCollapses = false
        model.setStripPreferences(preferences)
        let controller = FloatingPanelController(model: model, screenIdentifier: identifier)
        defer { controller.close() }
        model.floatingAppearanceHandler = { controller.applyAppearance() }
        controller.show()

        for density in [FloatingStripDensity.comfortable, .compact, .mini] {
            var next = model.stripPreferences
            next.density = density
            model.setStripPreferences(next)

            let frame = controller.stripFrameForTesting
            #expect(abs(frame.width - density.width) < 0.001)
            #expect(controller.stripContentBoundsForTesting.size == frame.size)
            #expect(frame.minX == screen.visibleFrame.minX || frame.maxX == screen.visibleFrame.maxX)
        }
    }

    @Test(.enabled(if: ProcessInfo.processInfo.environment["AI_METER_SCREEN_TESTS"] == "1"))
    func automaticCollapseExpansionSettlesToTheSameFrameForEveryDensity() async throws {
        let suite = "AutomaticCollapseFrame-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let screen = try #require(NSScreen.screens.first)
        let identifier = try #require(
            FloatingStripScreenIdentifier.identity(for: screen, mainScreen: screen)
        ).stableIdentifier
        let model = AppModel(
            defaults: defaults,
            secretStore: DisplaySettingsSecretStore(),
            widgetSnapshotPublisher: nil,
            isDemoMode: true
        )
        let controller = FloatingPanelController(model: model, screenIdentifier: identifier)
        defer { controller.close() }
        model.floatingAppearanceHandler = { controller.applyAppearance() }
        controller.show()
        controller.suspendFoldPollingForTesting()

        for density in FloatingStripDensity.allCases {
            var next = model.stripPreferences
            next.density = density
            next.automaticallyCollapses = true
            model.setStripPreferences(next)

            controller.transitionStripForTesting(toFolded: true)
            try await Task.sleep(for: .milliseconds(300))
            #expect(controller.stripIsFoldedForTesting)
            #expect(controller.stripFrameForTesting.size == FloatingStripLayout.foldedSize)

            controller.transitionStripForTesting(toFolded: false)
            let immediateFrame = controller.stripFrameForTesting
            #expect(abs(immediateFrame.width - density.width) < 0.001)
            #expect(controller.stripContentBoundsForTesting.size == immediateFrame.size)
            try await Task.sleep(for: .milliseconds(300))

            let settledFrame = controller.stripFrameForTesting
            #expect(abs(settledFrame.width - density.width) < 0.001)
            #expect(settledFrame == immediateFrame)
            #expect(controller.stripContentBoundsForTesting.size == settledFrame.size)
            #expect(!controller.stripIsFoldedForTesting)
        }
    }

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
