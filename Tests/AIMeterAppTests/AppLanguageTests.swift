import Foundation
import Testing
@testable import AIMeterApp

@Suite("App language")
struct AppLanguageTests {
    @Test("Language preference defaults to English and accepts only supported persisted values")
    func languagePreferenceDefaultsAndPersistsSupportedValues() throws {
        let suiteName = "AppLanguageTests.Preference.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let store = AppLanguagePreferenceStore(defaults: defaults)

        #expect(store.load() == .english)
        defaults.set("unknown", forKey: AppLanguagePreferenceStore.key)
        #expect(store.load() == .english)

        for language in [.english, .simplifiedChinese, .traditionalChinese] as [AppLanguage] {
            store.save(language)
            #expect(store.load() == language)
        }
    }

    @Test("Changing the app language updates the model immediately and survives reconstruction")
    @MainActor
    func appModelLanguagePreference() throws {
        let suiteName = "AppLanguageTests.Model.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let model = AppModel(defaults: defaults, widgetSnapshotPublisher: nil)
        #expect(model.appLanguage == .english)

        model.setAppLanguage(.english)
        #expect(defaults.object(forKey: AppLanguagePreferenceStore.key) == nil)

        model.setAppLanguage(.simplifiedChinese)
        #expect(model.appLanguage == .simplifiedChinese)

        let reconstructedModel = AppModel(defaults: defaults, widgetSnapshotPublisher: nil)
        #expect(reconstructedModel.appLanguage == .simplifiedChinese)
    }
}
