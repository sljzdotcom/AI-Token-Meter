import Foundation
import Testing
@testable import AIMeterCore

@Suite("Code0 optional fifth provider compatibility")
struct Code0ProviderCompatibilityTests {
    @Test("Code0 is catalogued but hidden on fresh defaults while widgets keep their three services")
    func newCode0ServiceDefaultsHiddenWithoutChangingWidgetDefaults() {
        let value = FloatingStripPreferences()
        #expect(value.orderedProviders.map(\.rawValue) == ["claude", "codex", "deepSeek", "gemini", "code0"])
        #expect(value.hiddenProviders.map(\.rawValue) == ["code0"])
        #expect(value.visibleProviders.map(\.rawValue) == ["claude", "codex", "deepSeek", "gemini"])
        #expect(WidgetProvider.displayedProviders.map(\.rawValue) == ["claude", "codex", "deepSeek"])
    }

    @Test("Upgrade appends hidden Code0 and preserves the four existing provider choices")
    func legacyFourProviderChoicesSurviveCode0Migration() throws {
        let data = Data(#"{"schemaVersion":5,"density":"comfortable","appearance":"liquidGlass","automaticallyCollapses":false,"revealDelayMilliseconds":200,"collapseDelayMilliseconds":950,"orderedProviders":["gemini","codex","claude","deepSeek"],"hiddenProviders":["claude"],"hiddenUntil":123}"#.utf8)
        let value = try JSONDecoder().decode(FloatingStripPreferences.self, from: data)
        #expect(value.orderedProviders.map(\.rawValue) == ["gemini", "codex", "claude", "deepSeek", "code0"])
        #expect(Set(value.hiddenProviders.map(\.rawValue)) == ["claude", "code0"])
        #expect(value.visibleProviders.map(\.rawValue) == ["gemini", "codex", "deepSeek"])
        #expect(value.density == .comfortable)
        #expect(value.appearance == .liquidGlass)
        #expect(!value.automaticallyCollapses)
        #expect(value.revealDelayMilliseconds == 200)
        #expect(value.collapseDelayMilliseconds == 950)
        #expect(value.hiddenUntil == 123)
        let json = try #require(JSONSerialization.jsonObject(with: JSONEncoder().encode(value)) as? [String: Any])
        #expect(json["schemaVersion"] as? Int == 6)
        #expect(try JSONDecoder().decode(FloatingStripPreferences.self, from: JSONEncoder().encode(value)) == value)
    }

    @Test("A new-schema explicit Code0 visibility and order survive normalization and reload")
    func explicitCode0ChoiceSurvivesReload() throws {
        let data = Data(#"{"schemaVersion":6,"orderedProviders":["code0","gemini","codex","claude","deepSeek"],"hiddenProviders":["codex"]}"#.utf8)
        let value = try JSONDecoder().decode(FloatingStripPreferences.self, from: data)
        #expect(value.orderedProviders.map(\.rawValue) == ["code0", "gemini", "codex", "claude", "deepSeek"])
        #expect(value.visibleProviders.map(\.rawValue) == ["code0", "gemini", "claude", "deepSeek"])
        #expect(try JSONDecoder().decode(FloatingStripPreferences.self, from: JSONEncoder().encode(value)) == value)
    }

    @Test("Five visible rows have full height while all original row dimensions stay unchanged")
    func fifthProviderHasFullHeightWithoutChangingOriginalFourHeights() {
        for (density, original) in [(FloatingStripDensity.compact, [170.0, 228, 286, 344]), (.comfortable, [212.0, 284, 356, 428])] {
            for (index, height) in original.enumerated() {
                #expect(density.height(providerCount: index + 1) == height)
            }
        }
        #expect(FloatingStripDensity.compact.height(providerCount: 5) == 402)
        #expect(FloatingStripDensity.comfortable.height(providerCount: 5) == 500)
    }
}
