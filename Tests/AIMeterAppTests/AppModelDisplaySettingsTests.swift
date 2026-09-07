import Foundation
import Testing
import AIMeterCore
@testable import AIMeterApp

@Suite("Display settings integration")
@MainActor
struct AppModelDisplaySettingsTests {
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
