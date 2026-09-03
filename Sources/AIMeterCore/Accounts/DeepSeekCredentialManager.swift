import Foundation

public enum DeepSeekCredentialReplacementError: Error, Equatable, Sendable {
    case emptyCandidate
    case invalidKey
    case verificationUnavailable
    case keychainFailure
}

public struct DeepSeekCredentialManager: ServiceAccountReading, Sendable {
    public let provider = UsageProvider.deepSeek
    private let secretStore: any SecretStore
    private let secretReader: DeepSeekCredentialReader
    private let validate: @Sendable (String) async throws -> Void

    public init(
        client: DeepSeekClient = DeepSeekClient(),
        secretStore: any SecretStore = KeychainStore(),
        secretReadTimeout: Duration = .seconds(2)
    ) {
        self.init(
            secretStore: secretStore,
            secretReadTimeout: secretReadTimeout,
            validate: { apiKey in
                _ = try await client.collect(apiKey: apiKey)
            }
        )
    }

    public init(
        secretStore: any SecretStore,
        secretReadTimeout: Duration = .seconds(2),
        validate: @escaping @Sendable (String) async throws -> Void
    ) {
        self.secretStore = secretStore
        self.secretReader = DeepSeekCredentialReader(
            secretStore: secretStore,
            timeout: secretReadTimeout
        )
        self.validate = validate
    }

    public func readStatus() async -> ServiceAccountStatus {
        do {
            let secret = try await secretReader.read()
            return Self.status(for: secret)
        } catch {
            return ServiceAccountStatus(provider: .deepSeek, connectionState: .unavailable)
        }
    }

    public func read() async -> ServiceAccountStatus {
        await readStatus()
    }

    public func replace(with candidate: String) async throws -> ServiceAccountStatus {
        let normalized = candidate.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else {
            throw DeepSeekCredentialReplacementError.emptyCandidate
        }

        do {
            try await validate(normalized)
        } catch let error as UsageCollectionError {
            if error == .authenticationRequired {
                throw DeepSeekCredentialReplacementError.invalidKey
            }
            throw DeepSeekCredentialReplacementError.verificationUnavailable
        } catch {
            throw DeepSeekCredentialReplacementError.verificationUnavailable
        }

        let previous: String?
        do {
            previous = try await secretReader.read()
        } catch {
            throw DeepSeekCredentialReplacementError.keychainFailure
        }

        do {
            try await write(normalized)
        } catch {
            await restore(previous)
            throw DeepSeekCredentialReplacementError.keychainFailure
        }
        return Self.status(for: normalized)
    }

    private func write(_ secret: String) async throws {
        let secretStore = self.secretStore
        try await Task.detached(priority: .utility) {
            try secretStore.save(secret)
        }.value
    }

    private func restore(_ secret: String?) async {
        let secretStore = self.secretStore
        _ = try? await Task.detached(priority: .utility) {
            if let secret {
                try secretStore.save(secret)
            } else {
                try secretStore.delete()
            }
        }.value
    }

    private static func status(for secret: String?) -> ServiceAccountStatus {
        guard let normalized = secret?.trimmingCharacters(in: .whitespacesAndNewlines),
              !normalized.isEmpty else {
            return ServiceAccountStatus(provider: .deepSeek, connectionState: .signInRequired)
        }
        return ServiceAccountStatus(
            provider: .deepSeek,
            connectionState: .connected,
            accountLabel: "API Key ••••\(normalized.suffix(4))"
        )
    }
}

private enum DeepSeekCredentialReadOutcome: Sendable {
    case value(String?)
    case failed
    case timedOut
}

private actor DeepSeekCredentialReader {
    private let secretStore: any SecretStore
    private let timeout: Duration
    private var inFlightToken: UUID?

    init(secretStore: any SecretStore, timeout: Duration) {
        self.secretStore = secretStore
        self.timeout = timeout
    }

    func read() async throws -> String? {
        guard inFlightToken == nil else {
            throw UsageCollectionError.timedOut
        }

        let token = UUID()
        let secretStore = self.secretStore
        let task = Task.detached(priority: .userInitiated) {
            do {
                return DeepSeekCredentialReadOutcome.value(try secretStore.read())
            } catch {
                return DeepSeekCredentialReadOutcome.failed
            }
        }
        inFlightToken = token

        Task { [weak self] in
            _ = await task.value
            await self?.finish(token: token)
        }

        switch await Self.firstResult(from: task, timeout: timeout) {
        case .value(let secret): return secret
        case .failed: throw UsageCollectionError.transportFailure
        case .timedOut: throw UsageCollectionError.timedOut
        }
    }

    private func finish(token: UUID) {
        guard inFlightToken == token else { return }
        inFlightToken = nil
    }

    nonisolated private static func firstResult(
        from task: Task<DeepSeekCredentialReadOutcome, Never>,
        timeout: Duration
    ) async -> DeepSeekCredentialReadOutcome {
        await withCheckedContinuation { continuation in
            let gate = CredentialOneShotContinuation(continuation)
            Task.detached { gate.resume(returning: await task.value) }
            Task.detached {
                try? await Task.sleep(for: timeout)
                gate.resume(returning: .timedOut)
            }
        }
    }
}

private final class CredentialOneShotContinuation<Value: Sendable>: @unchecked Sendable {
    private let lock = NSLock()
    private var continuation: CheckedContinuation<Value, Never>?

    init(_ continuation: CheckedContinuation<Value, Never>) {
        self.continuation = continuation
    }

    func resume(returning value: Value) {
        let continuation = lock.withLock {
            defer { self.continuation = nil }
            return self.continuation
        }
        continuation?.resume(returning: value)
    }
}
