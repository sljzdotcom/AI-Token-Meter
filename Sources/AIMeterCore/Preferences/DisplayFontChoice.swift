import Foundation

public enum DisplayFontChoice: String, CaseIterable, Codable, Identifiable, Sendable {
    case system
    case antonio
    case dinCondensed = "din-condensed"
    case alimamaFangYuanTiVF = "alimama-fangyuanti-vf"
    case firaCode = "fira-code"
    case leigo
    case menlo
    case alimamaDaoLiTi = "alimama-daoliti"

    public var id: String { rawValue }

    public var displayName: String {
        switch self {
        case .system: "System Default"
        case .antonio: "Antonio"
        case .dinCondensed: "DIN Condensed"
        case .alimamaFangYuanTiVF: "Alimama FangYuanTi VF"
        case .firaCode: "Fira Code"
        case .leigo: "Leigo"
        case .menlo: "Menlo"
        case .alimamaDaoLiTi: "Alimama DaoLiTi"
        }
    }
}

public struct DisplayFontPreferenceStore {
    public static let key = "appearance.displayFont"
    private let defaults: UserDefaults

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    public func load() -> DisplayFontChoice {
        defaults.string(forKey: Self.key)
            .flatMap(DisplayFontChoice.init(rawValue:)) ?? .system
    }

    public func save(_ choice: DisplayFontChoice) {
        defaults.set(choice.rawValue, forKey: Self.key)
    }
}
