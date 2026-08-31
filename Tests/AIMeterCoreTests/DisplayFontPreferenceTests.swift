import Foundation
import Testing
@testable import AIMeterCore

@Suite("Display font preference")
struct DisplayFontPreferenceTests {
    @Test("Defines stable identifiers and labels for exactly three choices")
    func choices() {
        #expect(DisplayFontChoice.allCases == [.system, .antonio, .dinCondensed])
        #expect(DisplayFontChoice.system.rawValue == "system")
        #expect(DisplayFontChoice.antonio.rawValue == "antonio")
        #expect(DisplayFontChoice.dinCondensed.rawValue == "din-condensed")
        #expect(DisplayFontChoice.system.displayName == "System Default")
        #expect(DisplayFontChoice.antonio.displayName == "Antonio")
        #expect(DisplayFontChoice.dinCondensed.displayName == "DIN Condensed")
    }

    @Test("Defaults to system and round-trips a supported selection")
    func roundTrip() {
        let suite = "DisplayFontPreferenceTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let store = DisplayFontPreferenceStore(defaults: defaults)

        #expect(store.load() == .system)
        store.save(.antonio)
        #expect(store.load() == .antonio)
        store.save(.system)
        #expect(store.load() == .system)
    }

    @Test("Corrupt persisted values recover to system")
    func corruptValue() {
        let suite = "DisplayFontPreferenceTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        defaults.set("unknown-font", forKey: DisplayFontPreferenceStore.key)

        #expect(DisplayFontPreferenceStore(defaults: defaults).load() == .system)
    }
}
