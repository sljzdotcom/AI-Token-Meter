public enum DetailAutoHideInterval: Int, CaseIterable, Identifiable, Sendable {
    case threeSeconds = 3
    case fiveSeconds = 5
    case eightSeconds = 8
    case fifteenSeconds = 15
    case thirtySeconds = 30

    public static let `default` = DetailAutoHideInterval.eightSeconds

    public var id: Int { rawValue }

    public init(storedSeconds: Int?) {
        self = storedSeconds.flatMap(Self.init(rawValue:)) ?? .default
    }
}
