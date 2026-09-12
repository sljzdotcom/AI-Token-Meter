import Foundation

enum AppLanguage: String, CaseIterable, Identifiable {
    case english = "en"
    case simplifiedChinese = "zh-Hans"
    case traditionalChinese = "zh-Hant"

    var id: Self { self }

    var locale: Locale { Locale(identifier: rawValue) }

    var displayName: String {
        switch self {
        case .english: "English"
        case .simplifiedChinese: "简体中文"
        case .traditionalChinese: "繁體中文"
        }
    }
}

struct AppLanguagePreferenceStore {
    static let key = "appLanguage"

    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    func load() -> AppLanguage {
        guard let rawValue = defaults.string(forKey: Self.key),
              let language = AppLanguage(rawValue: rawValue) else {
            return .english
        }
        return language
    }

    func save(_ language: AppLanguage) {
        defaults.set(language.rawValue, forKey: Self.key)
    }
}
