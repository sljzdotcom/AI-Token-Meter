import Foundation
import Testing
@testable import AIMeterCore

@Suite("Floating strip display preferences")
struct FloatingStripDisplaysTests {
    @Test func onlyDraggingOrEditingOfflineFallbackChangesTargetMode() {
        let primary = FloatingStripDisplays()
        #expect(!primary.shouldSelectTarget(after: .edit, actualIdentifier: "main"))
        #expect(primary.shouldSelectTarget(after: .drag, actualIdentifier: "external"))
        let all = FloatingStripDisplays(mode: .all)
        #expect(!all.shouldSelectTarget(after: .drag, actualIdentifier: "external"))
        let selected = FloatingStripDisplays(mode: .selected, selectedIdentifier: "external")
        #expect(!selected.shouldSelectTarget(after: .edit, actualIdentifier: "external"))
        #expect(selected.shouldSelectTarget(after: .edit, actualIdentifier: "main"))
    }
    @Test func fixedEdgeAppliesOnNewAndRestoredDisplaysWithoutChangingSavedEdges() {
        var value = FloatingStripDisplays(mode: .all)
        value.record(identifier: "a", edge: .right, normalizedCenterY: 0.2)
        #expect(value.placement(for: "a", preference: .left).edge == .left)
        #expect(value.placement(for: "new", preference: .left).edge == .left)
        #expect(value.placement(for: "a", preference: .automatic).edge == .right)
        #expect(value.placement(for: "a", preference: .left).normalizedCenterY == 0.2)
    }
    @Test func fallbackDoesNotOverwriteSelection() {
        let value = FloatingStripDisplays(mode: .selected, selectedIdentifier: "external")
        #expect(value.targetIdentifiers(online: ["primary"], primary: "primary") == ["primary"])
        #expect(value.selectedIdentifier == "external")
        #expect(value.targetIdentifiers(online: ["primary", "external"], primary: "primary") == ["external"])
    }

    @Test func allDisplaysDeduplicatesAndEmptyTopologyWaits() {
        let value = FloatingStripDisplays(mode: .all)
        #expect(value.targetIdentifiers(online: ["a", "b", "a"], primary: "b") == ["a", "b"])
        #expect(value.targetIdentifiers(online: [], primary: nil).isEmpty)
        #expect(FloatingStripDisplays().targetIdentifiers(online: ["a", "b"], primary: "b") == ["b"])
    }

    @Test func legacyMigrationAndIndependentPositionsSurviveReload() throws {
        let suite = "display-test-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let legacy = FloatingStripPosition(preference: .left, lastResolvedEdge: .left,
                                           normalizedCenterY: 0.3, screenIdentifier: "old")
        FloatingStripPositionStore(defaults: defaults).save(legacy)
        let store = FloatingStripDisplaysStore(defaults: defaults)
        var value = store.load()
        #expect(value.mode == .selected)
        #expect(value.selectedIdentifier == "old")
        #expect(value.placement(for: "old").edge == .left)
        #expect(value.placement(for: "old").normalizedCenterY == 0.3)
        value.record(identifier: "other", edge: .right, normalizedCenterY: 0.8)
        value.mode = .all
        store.save(value)
        let restored = store.load()
        #expect(restored.mode == .all)
        #expect(restored.placement(for: "old").normalizedCenterY == 0.3)
        #expect(restored.placement(for: "other").normalizedCenterY == 0.8)
        #expect(restored.placement(for: "other").edge == .right)
    }

    @Test func invalidCoordinatesAreNormalizedAndIdentityMigrationPreservesPosition() {
        var value = FloatingStripDisplays(mode: .selected, selectedIdentifier: "legacy")
        value.record(identifier: "legacy", edge: .left, normalizedCenterY: .nan)
        value.migrate(from: "legacy", to: "uuid")
        #expect(value.selectedIdentifier == "uuid")
        #expect(value.placements["legacy"] == nil)
        #expect(value.placement(for: "uuid").normalizedCenterY == 0.5)
        #expect(value.placement(for: "uuid").edge == .left)
        value.record(identifier: "uuid", edge: .right, normalizedCenterY: 2)
        #expect(value.placement(for: "uuid").normalizedCenterY == 1)
    }
}
