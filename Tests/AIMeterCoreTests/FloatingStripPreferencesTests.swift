import Foundation
import Testing
@testable import AIMeterCore

@Suite("Floating strip preferences and idle folding")
struct FloatingStripPreferencesTests {
    @Test func foldSchedulerCanHonorEveryFiftyMillisecondPreferenceStep() {
        #expect(FloatingStripFoldState.pollingInterval <= 0.025)
    }

    // Catch a fourth row being clipped by the previous three-provider size cap.
    @Test func fourthProviderHasFullHeightAndLegacySizesStayStable() {
        #expect(FloatingStripDensity.allCases == [.comfortable, .compact, .mini])
        #expect(FloatingStripDensity.mini.width == 65)
        #expect(FloatingStripDensity.mini.ringSize == 48)
        #expect((FloatingStripDensity.mini.width - FloatingStripDensity.mini.ringSize) / 2 == 8.5)
        #expect(FloatingStripDensity.compact.width == 78)
        #expect(FloatingStripDensity.compact.ringSize == 48)
        #expect((FloatingStripDensity.compact.width - FloatingStripDensity.compact.ringSize) / 2 == 15)
        #expect(FloatingStripDensity.comfortable.width == 108)
        for (density, heights) in [(FloatingStripDensity.mini, [212.0, 270, 328, 386]), (.compact, [212.0, 270, 328, 386]), (.comfortable, [260.0, 332, 404, 476])] {
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
        #expect(value.revealDelayMilliseconds == 150)
        #expect(value.collapseDelayMilliseconds == 5_000)
        #expect(value.hiddenUntil == 123)
        let gemini = try #require(UsageProvider(rawValue: "gemini"))
        let changed = value.settingVisible(gemini, visible: true)
        let encoded = try JSONEncoder().encode(changed)
        let json = try #require(JSONSerialization.jsonObject(with: encoded) as? [String: Any])
        #expect(json["schemaVersion"] as? Int == 4)
        #expect(json["automaticallyCollapses"] as? Bool == true)
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
        #expect(value.automaticallyCollapses)
        #expect(value.revealDelayMilliseconds == 150)
        #expect(value.collapseDelayMilliseconds == 5_000)
        #expect(value.visibleProviders == [.codex, .deepSeek])
    }
    @Test func restoresDefaultsAndSanitizesLayout() throws {
        let suite = UUID().uuidString
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let store = FloatingStripPreferencesStore(defaults: defaults)
        var value = store.load()
        #expect(value.density == .compact)
        #expect(value.automaticallyCollapses)
        #expect(value.revealDelayMilliseconds == 150)
        #expect(value.collapseDelayMilliseconds == 800)
        value.orderedProviders = [.codex, .codex]
        value.hiddenProviders = UsageProvider.allCases
        store.save(value)
        let restored = store.load()
        #expect(restored.orderedProviders.map(\.rawValue) == ["codex", "claude", "deepSeek", "gemini"])
        #expect(restored.visibleProviders == [.codex])
        #expect(!restored.settingVisible(.codex, visible: false).visibleProviders.isEmpty)
    }

    @Test func providerRemovalShrinksOnlyTheMiddle() {
        #expect(FloatingStripDensity.mini.height(providerCount: 2) == 270)
        #expect(FloatingStripDensity.compact.height(providerCount: 2) == 270)
        #expect(FloatingStripDensity.compact.height(providerCount: 1) == 212)
        #expect(FloatingStripDensity.comfortable.height(providerCount: 2) == 332)
    }

    @Test func hiddenDeadlineExpiresWithoutChangingPreference() {
        let value = FloatingStripPreferences(hiddenUntil: 100)
        #expect(value.isTemporarilyHidden(now: 99))
        #expect(!value.isTemporarilyHidden(now: 100))
    }

    @Test func revealAndCollapseDeadlinesAreCancelledByRapidPointerChanges() {
        var state = FloatingStripFoldState()
        state.update(now: 0, revealDelay: 0.15, collapseDelay: 0.8, hovering: false, lockedOpen: false)
        state.update(now: 0.79, revealDelay: 0.15, collapseDelay: 0.8, hovering: true, lockedOpen: false)
        #expect(!state.isFolded)
        state.update(now: 0.80, revealDelay: 0.15, collapseDelay: 0.8, hovering: false, lockedOpen: false)
        state.update(now: 1.59, revealDelay: 0.15, collapseDelay: 0.8, hovering: false, lockedOpen: false)
        #expect(!state.isFolded)
        state.update(now: 1.60, revealDelay: 0.15, collapseDelay: 0.8, hovering: false, lockedOpen: false)
        #expect(state.isFolded)
        state.update(now: 1.61, revealDelay: 0.15, collapseDelay: 0.8, hovering: true, lockedOpen: false)
        state.update(now: 1.70, revealDelay: 0.15, collapseDelay: 0.8, hovering: false, lockedOpen: false)
        state.update(now: 2.00, revealDelay: 0.15, collapseDelay: 0.8, hovering: true, lockedOpen: false)
        #expect(state.isFolded)
        state.update(now: 2.15, revealDelay: 0.15, collapseDelay: 0.8, hovering: true, lockedOpen: false)
        #expect(!state.isFolded)
        state.update(now: 30, revealDelay: 0.15, collapseDelay: 0.8, hovering: false, lockedOpen: true)
        #expect(!state.isFolded)
    }

    @Test func schemaFourPersistsAutomaticCollapseAndIndependentBoundedDelays() throws {
        var value = FloatingStripPreferences(revealDelayMilliseconds: 2_000, collapseDelayMilliseconds: 5_000)
        value.automaticallyCollapses = false
        value.normalize()
        let json = try #require(JSONSerialization.jsonObject(with: JSONEncoder().encode(value)) as? [String: Any])
        #expect(json["schemaVersion"] as? Int == 4)
        #expect(json["automaticallyCollapses"] as? Bool == false)
        #expect(json["revealDelayMilliseconds"] as? Int == 2_000)
        #expect(json["collapseDelayMilliseconds"] as? Int == 5_000)

        let invalid = Data(#"{"schemaVersion":3,"revealDelayMilliseconds":2001,"collapseDelayMilliseconds":5001}"#.utf8)
        let restored = try JSONDecoder().decode(FloatingStripPreferences.self, from: invalid)
        #expect(restored.revealDelayMilliseconds == 150)
        #expect(restored.collapseDelayMilliseconds == 800)
    }

    @Test func disablingAutomaticCollapseClearsDeadlinesAndKeepsTheStripExpanded() {
        var state = FloatingStripFoldState()
        state.update(now: 0, revealDelay: 0.15, collapseDelay: 0.8, hovering: false, lockedOpen: false)
        state.update(now: 1, revealDelay: 0.15, collapseDelay: 0.8, hovering: false, lockedOpen: false)
        #expect(state.isFolded)
        state.update(now: 1.01, revealDelay: 0.15, collapseDelay: 0.8, hovering: false,
                     lockedOpen: false, automaticallyCollapses: false)
        #expect(!state.isFolded)
        state.update(now: 30, revealDelay: 0.15, collapseDelay: 0.8, hovering: false,
                     lockedOpen: false, automaticallyCollapses: false)
        #expect(!state.isFolded)
    }
}
