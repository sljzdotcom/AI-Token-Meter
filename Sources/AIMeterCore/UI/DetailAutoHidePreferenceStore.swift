import Foundation

public struct DetailAutoHidePreferenceStore {
    private let defaults: UserDefaults
    private let key: String

    public init(
        defaults: UserDefaults = .standard,
        key: String = "detailAutoHideSeconds"
    ) {
        self.defaults = defaults
        self.key = key
    }

    public func load() -> DetailAutoHideInterval {
        DetailAutoHideInterval(storedSeconds: defaults.object(forKey: key) as? Int)
    }

    public func save(_ interval: DetailAutoHideInterval) {
        defaults.set(interval.rawValue, forKey: key)
    }
}
