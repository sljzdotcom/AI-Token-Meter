import Foundation

public struct DeepSeekCollector: UsageCollector {
    public let provider = UsageProvider.deepSeek

    private let client: DeepSeekClient
    private let secretStore: any SecretStore
    private let budget: DeepSeekBudget?

    public init(
        client: DeepSeekClient = DeepSeekClient(),
        secretStore: any SecretStore = KeychainStore(),
        budget: DeepSeekBudget? = nil
    ) {
        self.client = client
        self.secretStore = secretStore
        self.budget = budget
    }

    public func collect() async throws -> UsageSnapshot {
        let storedSecret: String?
        do {
            storedSecret = try secretStore.read()
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
