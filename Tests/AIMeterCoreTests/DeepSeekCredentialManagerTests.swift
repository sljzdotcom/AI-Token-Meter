import Foundation
import Testing
@testable import AIMeterCore

@Suite("DeepSeek credential manager", .serialized)
struct DeepSeekCredentialManagerTests {
    @Test("Status exposes only the last four API Key characters")
    func maskedStatus() async {
        let store = RecordingCredentialStore(initial: "sk-private-ABCD")
        let status = await manager(store: store).readStatus()

        #expect(status.connectionState == .connected)
        #expect(status.accountLabel == "API Key ••••ABCD")
        #expect(status.accountDetail == nil)
        #expect(!String(describing: status).contains("sk-private"))
    }

    @Test("Missing Key requires configuration")
    func missingKey() async {
        let status = await manager(store: RecordingCredentialStore(initial: nil)).readStatus()

        #expect(status.connectionState == .signInRequired)
        #expect(status.accountLabel == nil)
    }

    @Test("An invalid candidate never replaces a working Key")
    func invalidCandidateKeepsOldKey() async {
        let store = RecordingCredentialStore(initial: "old-working-key")
        let manager = DeepSeekCredentialManager(
            secretStore: store,
            validate: { _ in throw UsageCollectionError.authenticationRequired }
        )

        await #expect(throws: DeepSeekCredentialReplacementError.invalidKey) {
            try await manager.replace(with: "new-invalid-key")
        }
        #expect(store.secret == "old-working-key")
        #expect(store.savedValues.isEmpty)
    }

    @Test("Network and server failures preserve the old Key")
    func verificationFailureKeepsOldKey() async {
        for error in [UsageCollectionError.timedOut, .transportFailure, .invalidResponse] {
            let store = RecordingCredentialStore(initial: "old-working-key")
            let manager = DeepSeekCredentialManager(
                secretStore: store,
                validate: { _ in throw error }
            )

            await #expect(throws: DeepSeekCredentialReplacementError.verificationUnavailable) {
                try await manager.replace(with: "candidate-key")
            }
            #expect(store.secret == "old-working-key")
            #expect(store.savedValues.isEmpty)
        }
    }

    @Test("A verified candidate replaces the Key and returns its masked status")
    func successfulReplacement() async throws {
        let store = RecordingCredentialStore(initial: "old-working-key")
        let manager = manager(store: store)

        let status = try await manager.replace(with: "  new-working-WXYZ  ")

        #expect(store.secret == "new-working-WXYZ")
        #expect(store.savedValues == ["new-working-WXYZ"])
        #expect(status.accountLabel == "API Key ••••WXYZ")
    }

    @Test("A failed Keychain update restores the previous Key")
    func failedWriteRestoresOldKey() async {
        let store = RecordingCredentialStore(initial: "old-working-key", failAfterMutatingFirstSave: true)
        let manager = manager(store: store)

        await #expect(throws: DeepSeekCredentialReplacementError.keychainFailure) {
            try await manager.replace(with: "new-working-key")
        }

        #expect(store.secret == "old-working-key")
        #expect(store.savedValues == ["new-working-key", "old-working-key"])
    }

    @Test("A failed first save removes a partially written candidate")
    func failedFirstSaveRemovesCandidate() async {
        let store = RecordingCredentialStore(initial: nil, failAfterMutatingFirstSave: true)
        let manager = manager(store: store)

        await #expect(throws: DeepSeekCredentialReplacementError.keychainFailure) {
            try await manager.replace(with: "first-key")
        }

        #expect(store.secret == nil)
        #expect(store.deleteCount == 1)
    }

    @Test("An empty candidate is rejected before verification or Keychain access")
    func emptyCandidate() async {
        let store = RecordingCredentialStore(initial: "old")
        let validationCount = LockedCounter()
        let manager = DeepSeekCredentialManager(
            secretStore: store,
            validate: { _ in validationCount.increment() }
        )

        await #expect(throws: DeepSeekCredentialReplacementError.emptyCandidate) {
            try await manager.replace(with: " \n ")
        }
        #expect(validationCount.value == 0)
        #expect(store.readCount == 0)
    }

    @Test("A slow Keychain read degrades to unavailable")
    func slowRead() async {
        let store = RecordingCredentialStore(initial: "late-key", readDelay: 0.2)
        let manager = DeepSeekCredentialManager(
            secretStore: store,
            secretReadTimeout: .milliseconds(20),
            validate: { _ in }
        )

        let status = await manager.readStatus()

        #expect(status.connectionState == .unavailable)
        #expect(status.accountLabel == nil)
    }

    private func manager(store: RecordingCredentialStore) -> DeepSeekCredentialManager {
        DeepSeekCredentialManager(secretStore: store, validate: { _ in })
    }
}

private final class RecordingCredentialStore: SecretStore, @unchecked Sendable {
    private let lock = NSLock()
    private var storedSecret: String?
    private var saves: [String] = []
    private var reads = 0
    private var deletes = 0
    private var shouldFailFirstSave: Bool
    private let readDelay: TimeInterval

    init(
        initial: String?,
        failAfterMutatingFirstSave: Bool = false,
        readDelay: TimeInterval = 0
    ) {
        storedSecret = initial
        shouldFailFirstSave = failAfterMutatingFirstSave
        self.readDelay = readDelay
    }

    func read() throws -> String? {
        if readDelay > 0 { Thread.sleep(forTimeInterval: readDelay) }
        return lock.withLock {
            reads += 1
            return storedSecret
        }
    }

    func save(_ secret: String) throws {
        let shouldFail = lock.withLock {
            storedSecret = secret
            saves.append(secret)
            defer { shouldFailFirstSave = false }
            return shouldFailFirstSave
        }
        if shouldFail { throw SecretStoreError.invalidData }
    }

    func delete() throws {
        lock.withLock {
            storedSecret = nil
            deletes += 1
        }
    }

    var secret: String? { lock.withLock { storedSecret } }
    var savedValues: [String] { lock.withLock { saves } }
    var readCount: Int { lock.withLock { reads } }
    var deleteCount: Int { lock.withLock { deletes } }
}

private final class LockedCounter: @unchecked Sendable {
    private let lock = NSLock()
    private var count = 0
    func increment() { lock.withLock { count += 1 } }
    var value: Int { lock.withLock { count } }
}
