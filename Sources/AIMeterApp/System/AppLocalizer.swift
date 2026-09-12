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
        let resourceRoot: URL?
        if let resourceBundle {
            resourceRoot = resourceBundle.resourceURL
        } else {
            // English is mandatory. Resolve its root through the portable app locator
            // so distributed apps never evaluate SwiftPM's build-machine fallback.
            resourceRoot = AppResourceLocator.url(
                forResource: "Localizable", withExtension: "strings", subdirectory: "en.lproj"
            )?.deletingLastPathComponent().deletingLastPathComponent()
        }
        guard let resourceRoot,
              let directories = try? FileManager.default.contentsOfDirectory(
                at: resourceRoot, includingPropertiesForKeys: nil
              ),
              let directory = directories.first(where: {
                $0.lastPathComponent.caseInsensitiveCompare("\(language.rawValue).lproj") == .orderedSame
              }),
              FileManager.default.fileExists(atPath: directory.appending(path: "Localizable.strings").path)
        else { return nil }
        // Keep the actual directory spelling: SwiftPM lowercases language identifiers.
        return Bundle(url: directory)
    }
}
