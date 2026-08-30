import Foundation
import Testing
@testable import AIMeterCore

@Suite("Detail auto-hide interval")
struct DetailAutoHideIntervalTests {
    @Test("Offers the approved fixed choices with eight seconds as default")
    func choicesAndDefault() {
        #expect(DetailAutoHideInterval.allCases.map(\.rawValue) == [3, 5, 8, 15, 30])
        #expect(DetailAutoHideInterval.default.rawValue == 8)
    }

    @Test("Restores supported values and rejects unsupported persisted values")
    func restoresPersistedValue() {
        #expect(DetailAutoHideInterval(storedSeconds: 15).rawValue == 15)
        #expect(DetailAutoHideInterval(storedSeconds: 0) == .default)
        #expect(DetailAutoHideInterval(storedSeconds: 999) == .default)
        #expect(DetailAutoHideInterval(storedSeconds: nil) == .default)
    }

    @Test("Persists a supported choice and falls back from corrupt stored values")
    func persistsChoice() throws {
        let suiteName = "com.millerpan.AIMeter.interval-tests.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let store = DetailAutoHidePreferenceStore(defaults: defaults)

        #expect(store.load() == .default)
        store.save(.fifteenSeconds)
        #expect(DetailAutoHidePreferenceStore(defaults: defaults).load() == .fifteenSeconds)
        defaults.set(999, forKey: "detailAutoHideSeconds")
        #expect(store.load() == .default)
    }
}
