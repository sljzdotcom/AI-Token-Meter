import Foundation
import Testing
@testable import AIMeterCore

private let keychainTestsEnabled = ProcessInfo.processInfo.environment["AI_METER_RUN_KEYCHAIN_TESTS"] == "1"

@Suite("Keychain secret store", .serialized)
struct KeychainStoreTests {
    @Test(
        "Creates, replaces, reads, and deletes an isolated secret",
        .enabled(if: keychainTestsEnabled)
    )
    func lifecycle() throws {
        let service = "com.millerpan.AIMeter.tests.\(UUID().uuidString)"
        let store = KeychainStore(service: service, account: "deepseek-api-key")
        defer { try? store.delete() }

        #expect(try store.read() == nil)
        try store.save("first-test-secret")
        #expect(try store.read() == "first-test-secret")
        try store.save("replacement-test-secret")
        #expect(try store.read() == "replacement-test-secret")
        try store.delete()
        #expect(try store.read() == nil)
    }
}
