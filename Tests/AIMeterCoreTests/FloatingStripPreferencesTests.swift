import Foundation
import Testing
@testable import AIMeterCore

@Suite("Floating strip preferences and idle folding")
struct FloatingStripPreferencesTests {
    // Catch a fourth row being clipped by the previous three-provider size cap.
    @Test func fourthProviderHasFullHeightAndLegacySizesStayStable() {
        #expect(FloatingStripDensity.compact.width == 56.5)
        #expect(FloatingStripDensity.compact.ringSize == 48)
        #expect((FloatingStripDensity.compact.width - FloatingStripDensity.compact.ringSize) / 2 == 4.25)
        #expect(FloatingStripDensity.comfortable.width == 108)
        for (density, heights) in [(FloatingStripDensity.compact, [170.0, 228, 286, 344]), (.comfortable, [212.0, 284, 356, 428])] {
            for (index, height) in heights.enumerated() {
                #expect(density.height(providerCount: index + 1) == height)
            }
        }
    }

    // Catch migration erasing ordering, visibility, density, or applying twice.
    @Test func legacyMigrationAndRestartPreserveUserChoices() throws {
        let data = Data(#"{"density":"comfortable","foldDelay":15,"orderedProviders":["deepSeek","codex","claude"],"hiddenProviders":["codex"],"hiddenUntil":123}"#.utf8)
        let value = try JSONDecoder().decode(FloatingStripPreferences.self, from: data)
        #expect(value.orderedProviders.map(\.rawValue) == ["deepSeek", "codex", "claude", "gemini"])
        #expect(value.visibleProviders.map(\.rawValue) == ["deepSeek", "claude"])
        #expect(value.density == .comfortable)
        #expect(value.foldDelay == .fifteenSeconds)
        #expect(value.hiddenUntil == 123)
        let gemini = try #require(UsageProvider(rawValue: "gemini"))
        let changed = value.settingVisible(gemini, visible: true)
        let encoded = try JSONEncoder().encode(changed)
        let json = try #require(JSONSerialization.jsonObject(with: encoded) as? [String: Any])
        #expect(json["schemaVersion"] as? Int == 2)
        #expect(try JSONDecoder().decode(FloatingStripPreferences.self, from: encoded) == changed)
    }

    @Test func freshDefaultsAndExistingGeminiChoiceRemainVisible() throws {
        #expect(FloatingStripPreferences().visibleProviders.map(\.rawValue) == ["claude", "codex", "deepSeek", "gemini"])
        let data = Data(#"{"orderedProviders":["gemini","codex","gemini","future"],"hiddenProviders":["claude","deepSeek"]}"#.utf8)
        let value = try JSONDecoder().decode(FloatingStripPreferences.self, from: data)
        #expect(value.visibleProviders.map(\.rawValue) == ["gemini", "codex"])
    }

    @Test func everyOrderAndNonemptyVisibilitySetSurvivesPersistence() throws {
        let providers = UsageProvider.allCases
        #expect(providers.count == 4)
        func permutations(_ remaining: [UsageProvider]) -> [[UsageProvider]] {
            if remaining.isEmpty { return [[]] }
            return remaining.flatMap { first in permutations(remaining.filter { $0 != first }).map { [first] + $0 } }
        }
        let orders = permutations(providers)
        #expect(orders.count == 24)
        for order in orders {
            for mask in 1..<16 {
                var value = FloatingStripPreferences()
                value.orderedProviders = order
                value.hiddenProviders = order.enumerated().filter { mask & (1 << $0.offset) == 0 }.map(\.element)
                value.normalize()
                let restored = try JSONDecoder().decode(FloatingStripPreferences.self, from: JSONEncoder().encode(value))
                #expect(restored.orderedProviders == order)
                #expect(restored.visibleProviders == order.enumerated().filter { mask & (1 << $0.offset) != 0 }.map(\.element))
                if restored.visibleProviders.count == 1, let only = restored.visibleProviders.first {
                    #expect(restored.settingVisible(only, visible: false).visibleProviders == [only])
                }
            }
        }
    }

    @Test func unknownFieldsDoNotEraseRecognizedPreferences() throws {
        let data = Data(#"{"density":"future","foldDelay":5,"orderedProviders":["codex","future","claude"],"hiddenProviders":["claude"]}"#.utf8)
        let value = try JSONDecoder().decode(FloatingStripPreferences.self, from: data)
        #expect(value.density == .compact)
        #expect(value.foldDelay == .fiveSeconds)
        #expect(value.visibleProviders == [.codex, .deepSeek])
    }
    @Test func restoresDefaultsAndSanitizesLayout() throws {
        let suite = UUID().uuidString
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let store = FloatingStripPreferencesStore(defaults: defaults)
        var value = store.load()
        #expect(value.density == .compact)
        #expect(value.foldDelay == .never)
        value.orderedProviders = [.codex, .codex]
        value.hiddenProviders = UsageProvider.allCases
        store.save(value)
        let restored = store.load()
        #expect(restored.orderedProviders.map(\.rawValue) == ["codex", "claude", "deepSeek", "gemini"])
        #expect(restored.visibleProviders == [.codex])
        #expect(!restored.settingVisible(.codex, visible: false).visibleProviders.isEmpty)
    }

    @Test func providerRemovalShrinksOnlyTheMiddle() {
        #expect(FloatingStripDensity.compact.height(providerCount: 2) == 228)
        #expect(FloatingStripDensity.compact.height(providerCount: 1) == 170)
        #expect(FloatingStripDensity.comfortable.height(providerCount: 2) == 284)
    }

    @Test func hiddenDeadlineExpiresWithoutChangingPreference() {
        let value = FloatingStripPreferences(hiddenUntil: 100)
        #expect(value.isTemporarilyHidden(now: 99))
        #expect(!value.isTemporarilyHidden(now: 100))
    }

    @Test func foldDeadlineIsCancelledByInteraction() {
        var state = FloatingStripFoldState()
        state.update(now: 0, delay: 5, locked: false)
        state.update(now: 4, delay: 5, locked: true)
        state.update(now: 6, delay: 5, locked: false)
        state.update(now: 10, delay: 5, locked: false)
        #expect(!state.isFolded)
        state.update(now: 11, delay: 5, locked: false)
        #expect(state.isFolded)
        state.update(now: 12, delay: 5, locked: true)
        #expect(!state.isFolded)
        state.update(now: 30, delay: 0, locked: false)
        #expect(!state.isFolded)
    }
}
