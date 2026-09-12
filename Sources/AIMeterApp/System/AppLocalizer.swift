import Foundation

/// Resolves app-owned text without depending on the process or system language.
struct AppLocalizer {
    let language: AppLanguage
    private let localizedBundle: Bundle?
    private let englishBundle: Bundle?

    init(language: AppLanguage, resourceBundle: Bundle? = nil) {
        self.language = language
        localizedBundle = Self.bundle(for: language, resourceBundle: resourceBundle)
        englishBundle = Self.bundle(for: .english, resourceBundle: resourceBundle)
    }

    func text(_ key: String, _ arguments: CVarArg...) -> String {
        let missing = "\u{0}missing-localization\u{0}"
        let translated = localizedBundle?.localizedString(forKey: key, value: missing, table: "Localizable")
        let format: String
        if let translated, translated != missing {
            format = translated
        } else {
            let fallback = englishBundle?.localizedString(forKey: key, value: missing, table: "Localizable")
            format = fallback.flatMap { $0 == missing ? nil : $0 } ?? key
        }
        // Ordinary labels can contain literal percent signs and are not format strings.
        guard !arguments.isEmpty else { return format }
        return String(format: format, locale: language.locale, arguments: arguments)
    }

    func number(_ value: Int64, style: NumberFormatter.Style = .decimal) -> String {
        let formatter = NumberFormatter()
        formatter.locale = language.locale
        formatter.numberStyle = style
        return formatter.string(from: NSNumber(value: value)) ?? String(value)
    }

    func date(
        _ value: Date,
        dateStyle: DateFormatter.Style = .medium,
        timeStyle: DateFormatter.Style = .short,
        timeZone: TimeZone = .current
    ) -> String {
        let formatter = DateFormatter()
        formatter.locale = language.locale
        formatter.timeZone = timeZone
        formatter.dateStyle = dateStyle
        formatter.timeStyle = timeStyle
        return formatter.string(from: value)
    }

    private static func bundle(for language: AppLanguage, resourceBundle: Bundle?) -> Bundle? {
        let tableURL: URL?
        if let resourceBundle {
            // Exact directories avoid Foundation silently selecting another preferred language.
            tableURL = resourceBundle.resourceURL?
                .appending(path: "\(language.rawValue).lproj/Localizable.strings")
        } else {
            // Packaged apps use their own Resources; SwiftPM runs fall back to Bundle.module.
            tableURL = AppResourceLocator.url(
                forResource: "Localizable", withExtension: "strings",
                subdirectory: "\(language.rawValue).lproj"
            )
        }
        guard let tableURL, FileManager.default.fileExists(atPath: tableURL.path) else { return nil }
        return Bundle(url: tableURL.deletingLastPathComponent())
    }
}
