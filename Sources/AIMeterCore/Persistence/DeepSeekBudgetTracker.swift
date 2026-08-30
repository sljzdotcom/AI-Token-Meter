import Foundation

public final class DeepSeekBudgetTracker: @unchecked Sendable {
    private let defaults: UserDefaults
    private let keyPrefix: String
    private let lock = NSLock()

    public init(
        defaults: UserDefaults = .standard,
        keyPrefix: String = "deepseek.budget"
    ) {
        self.defaults = defaults
        self.keyPrefix = keyPrefix
    }

    @discardableResult
    public func record(
        balance: Double,
        currency: DeepSeekCurrency,
        at date: Date = Date(),
        isFresh: Bool = true
    ) -> Double {
        lock.withLock {
            let currentMonth = monthIdentifier(for: date)
            let monthKey = "\(keyPrefix).month"
            if defaults.string(forKey: monthKey) != currentMonth {
                if !isFresh { return 0 }
                defaults.set(currentMonth, forKey: monthKey)
                for knownCurrency in [DeepSeekCurrency.cny, .usd] {
                    defaults.removeObject(forKey: lastBalanceKey(for: knownCurrency))
                    defaults.removeObject(forKey: spentKey(for: knownCurrency))
                }
            }

            let spentKey = spentKey(for: currency)
            let existingSpend = defaults.double(forKey: spentKey)
            guard isFresh else { return existingSpend }

            let lastKey = lastBalanceKey(for: currency)
            if defaults.object(forKey: lastKey) == nil {
                defaults.set(balance, forKey: lastKey)
                defaults.set(existingSpend, forKey: spentKey)
                return existingSpend
            }

            let previousBalance = defaults.double(forKey: lastKey)
            let observedSpend = max(previousBalance - balance, 0)
            let totalSpend = existingSpend + observedSpend
            defaults.set(balance, forKey: lastKey)
            defaults.set(totalSpend, forKey: spentKey)
            return totalSpend
        }
    }

    private func monthIdentifier(for date: Date) -> String {
        let components = Calendar.current.dateComponents([.year, .month], from: date)
        return String(format: "%04d-%02d", components.year ?? 0, components.month ?? 0)
    }

    private func lastBalanceKey(for currency: DeepSeekCurrency) -> String {
        "\(keyPrefix).\(currency.rawValue).lastBalance"
    }

    private func spentKey(for currency: DeepSeekCurrency) -> String {
        "\(keyPrefix).\(currency.rawValue).spent"
    }
}
