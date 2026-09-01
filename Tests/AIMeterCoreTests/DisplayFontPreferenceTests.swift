import Foundation
import Testing
@testable import AIMeterCore

@Suite("Display font preference")
struct DisplayFontPreferenceTests {
    @Test("Defines stable identifiers and labels for eight ordered choices")
    func choices() {
        #expect(DisplayFontChoice.allCases == [
            .system,
            .antonio,
            .dinCondensed,
            .alimamaFangYuanTiVF,
            .firaCode,
            .leigo,
            .menlo,
            .alimamaDaoLiTi,
        ])
        #expect(DisplayFontChoice.system.rawValue == "system")
        #expect(DisplayFontChoice.antonio.rawValue == "antonio")
        #expect(DisplayFontChoice.dinCondensed.rawValue == "din-condensed")
        #expect(DisplayFontChoice.alimamaFangYuanTiVF.rawValue == "alimama-fangyuanti-vf")
        #expect(DisplayFontChoice.firaCode.rawValue == "fira-code")
        #expect(DisplayFontChoice.leigo.rawValue == "leigo")
        #expect(DisplayFontChoice.menlo.rawValue == "menlo")
        #expect(DisplayFontChoice.alimamaDaoLiTi.rawValue == "alimama-daoliti")
        #expect(DisplayFontChoice.system.displayName == "System Default")
        #expect(DisplayFontChoice.antonio.displayName == "Antonio")
        #expect(DisplayFontChoice.dinCondensed.displayName == "DIN Condensed")
        #expect(DisplayFontChoice.alimamaFangYuanTiVF.displayName == "Alimama FangYuanTi VF")
        #expect(DisplayFontChoice.firaCode.displayName == "Fira Code")
        #expect(DisplayFontChoice.leigo.displayName == "Leigo")
        #expect(DisplayFontChoice.menlo.displayName == "Menlo")
        #expect(DisplayFontChoice.alimamaDaoLiTi.displayName == "Alimama DaoLiTi")
    }

    @Test("Defaults to system and round-trips a supported selection")
    func roundTrip() {
        let suite = "DisplayFontPreferenceTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let store = DisplayFontPreferenceStore(defaults: defaults)

        #expect(store.load() == .system)
        for choice in DisplayFontChoice.allCases {
            store.save(choice)
            #expect(store.load() == choice)
        }
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
