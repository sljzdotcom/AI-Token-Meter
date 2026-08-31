import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("App model startup")
struct AppModelStartupTests {
    @Test("Initialization never blocks on a Keychain read")
    @MainActor
    func initializationDoesNotReadSecret() {
        let secretStore = ReadCountingSecretStore()
        let suiteName = "AppModelStartupTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let model = AppModel(defaults: defaults, secretStore: secretStore)

        #expect(secretStore.readCount == 0)
        #expect(!model.apiKeyConfigured)
    }

    @Test("Display font defaults, updates, and persists without restarting")
    @MainActor
    func displayFontPreference() {
        let suiteName = "AppModelStartupTests.Font.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let secretStore = ReadCountingSecretStore()

        let model = AppModel(defaults: defaults, secretStore: secretStore)
        #expect(model.displayFontChoice == .system)

        model.setDisplayFontChoice(.antonio)
        #expect(model.displayFontChoice == .antonio)
        #expect(DisplayFontPreferenceStore(defaults: defaults).load() == .antonio)

        model.restoreDefaultDisplayFont()
        #expect(model.displayFontChoice == .system)
    }
}

private final class ReadCountingSecretStore: SecretStore, @unchecked Sendable {
    private let lock = NSLock()
    private var storedReadCount = 0

    var readCount: Int {
        lock.withLock { storedReadCount }
    }

    func read() throws -> String? {
        lock.withLock { storedReadCount += 1 }
        return "configured"
    }

    func save(_ secret: String) throws {}
    func delete() throws {}
}
