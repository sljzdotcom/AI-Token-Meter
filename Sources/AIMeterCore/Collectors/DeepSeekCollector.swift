import Foundation

public struct DeepSeekCollector: UsageCollector {
    public let provider = UsageProvider.deepSeek

    private let client: DeepSeekClient
    private let secretReader: TimedSecretReader
    private let budget: DeepSeekBudget?

    public init(
        client: DeepSeekClient = DeepSeekClient(),
        secretStore: any SecretStore = KeychainStore(),
        budget: DeepSeekBudget? = nil,
        secretReadTimeout: Duration = .seconds(2)
    ) {
        self.client = client
        self.secretReader = TimedSecretReader(
            secretStore: secretStore,
            timeout: secretReadTimeout
        )
        self.budget = budget
    }

    public func collect() async throws -> UsageSnapshot {
        let storedSecret: String?
        do {
            storedSecret = try await secretReader.read()
        } catch UsageCollectionError.timedOut {
            throw UsageCollectionError.timedOut
        } catch {
            throw UsageCollectionError.transportFailure
        }

        guard let apiKey = storedSecret?.trimmingCharacters(in: .whitespacesAndNewlines),
              !apiKey.isEmpty else {
            throw UsageCollectionError.authenticationRequired
        }
        return try await client.collect(apiKey: apiKey, budget: budget)
    }
}

private enum SecretReadOutcome: Sendable {
    case value(String?)
    case failed
    case timedOut
}

private actor TimedSecretReader {
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
                return SecretReadOutcome.value(try secretStore.read())
            } catch {
                return SecretReadOutcome.failed
            }
        }
        inFlightToken = token

        Task { [weak self] in
            _ = await task.value
            await self?.finish(token: token)
        }

        switch await Self.firstResult(from: task, timeout: timeout) {
        case let .value(secret):
            return secret
        case .failed:
            throw UsageCollectionError.transportFailure
        case .timedOut:
            throw UsageCollectionError.timedOut
        }
    }

    private func finish(token: UUID) {
        guard inFlightToken == token else { return }
        inFlightToken = nil
    }

    nonisolated private static func firstResult(
        from task: Task<SecretReadOutcome, Never>,
        timeout: Duration
    ) async -> SecretReadOutcome {
        await NonStarvingDeadline.firstResult(
            from: task,
            timeout: timeout,
            timedOutValue: .timedOut
        )
    }
}
