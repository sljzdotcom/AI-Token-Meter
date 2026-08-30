import Foundation

public enum DeepSeekCurrency: String, Codable, Sendable {
    case cny = "CNY"
    case usd = "USD"

    var usageUnit: UsageUnit {
        switch self {
        case .cny: .cny
        case .usd: .usd
        }
    }
}

public struct DeepSeekBudget: Equatable, Sendable {
    public let monthlyLimit: Double
    public let trackedSpend: Double
    public let currency: DeepSeekCurrency

    public init(monthlyLimit: Double, trackedSpend: Double, currency: DeepSeekCurrency) {
        self.monthlyLimit = monthlyLimit
        self.trackedSpend = trackedSpend
        self.currency = currency
    }
}

public struct DeepSeekClient: Sendable {
    private let session: URLSession
    private let endpoint: URL
    private let preferredCurrency: DeepSeekCurrency

    public init(
        session: URLSession = .shared,
        endpoint: URL = URL(string: "https://api.deepseek.com/user/balance")!,
        preferredCurrency: DeepSeekCurrency = .cny
    ) {
        self.session = session
        self.endpoint = endpoint
        self.preferredCurrency = preferredCurrency
    }

    public func collect(
        apiKey: String,
        budget: DeepSeekBudget? = nil
    ) async throws -> UsageSnapshot {
        let normalizedAPIKey = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalizedAPIKey.isEmpty else {
            throw UsageCollectionError.authenticationRequired
        }

        var request = URLRequest(url: endpoint)
        request.timeoutInterval = 10
        request.httpMethod = "GET"
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.setValue("Bearer \(normalizedAPIKey)", forHTTPHeaderField: "Authorization")

        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await session.data(for: request)
        } catch let error as URLError where error.code == .timedOut {
            throw UsageCollectionError.timedOut
        } catch {
            throw UsageCollectionError.transportFailure
        }

        guard let httpResponse = response as? HTTPURLResponse else {
            throw UsageCollectionError.invalidResponse
        }
        switch httpResponse.statusCode {
        case 200:
            break
        case 401:
            throw UsageCollectionError.authenticationRequired
        case 429:
            throw UsageCollectionError.rateLimited
        default:
            throw UsageCollectionError.transportFailure
        }

        guard let payload = try? JSONDecoder().decode(DeepSeekBalanceResponse.self, from: data) else {
            throw UsageCollectionError.invalidResponse
        }

        let targetCurrency = budget?.currency ?? preferredCurrency
        guard let balanceInfo = payload.balanceInfos.first(where: { $0.currency == targetCurrency }),
              let totalBalance = Double(balanceInfo.totalBalance) else {
            throw UsageCollectionError.invalidResponse
        }

        let balanceMetric = UsageMetric(
            label: "Available balance",
            current: totalBalance,
            limit: nil,
            unit: targetCurrency.usageUnit,
            kind: .balance
        )
        let budgetMetric = budget.map {
            UsageMetric(
                label: "Local monthly budget",
                current: $0.trackedSpend,
                limit: $0.monthlyLimit,
                unit: $0.currency.usageUnit,
                kind: .localBudget
            )
        }

        return UsageSnapshot(
            provider: .deepSeek,
            primaryMetric: balanceMetric,
            secondaryMetric: budgetMetric,
            availability: payload.isAvailable ? .available : .unavailable,
            sourceVersion: "deepseek-balance-api",
            collectionStatus: .fresh
        )
    }
}

private struct DeepSeekBalanceResponse: Decodable {
    let isAvailable: Bool
    let balanceInfos: [DeepSeekBalanceInfo]

    enum CodingKeys: String, CodingKey {
        case isAvailable = "is_available"
        case balanceInfos = "balance_infos"
    }
}

private struct DeepSeekBalanceInfo: Decodable {
    let currency: DeepSeekCurrency
    let totalBalance: String

    enum CodingKeys: String, CodingKey {
        case currency
        case totalBalance = "total_balance"
    }
}
