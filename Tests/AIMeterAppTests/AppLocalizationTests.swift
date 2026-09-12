import Foundation
import Testing
@testable import AIMeterApp

@Suite("App localization")
struct AppLocalizationTests {
    // Missing, empty or duplicate shipped entries must fail before any UI consumes them.
    @Test("Shipped language tables are complete, nonempty and unique")
    func shippedTablesAreComplete() throws {
        var keySets: [Set<String>] = []
        for language in AppLanguage.allCases {
            let url = try #require(AppResourceLocator.url(
                forResource: "Localizable", withExtension: "strings",
                subdirectory: "\(language.rawValue).lproj"
            ))
            let data = try Data(contentsOf: url)
            let table = try #require(PropertyListSerialization.propertyList(from: data, format: nil) as? [String: String])
            #expect(!table.isEmpty)
            #expect(table.values.allSatisfy { !$0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty })
            let source = try String(contentsOf: url, encoding: .utf8)
            let entryPattern = try NSRegularExpression(pattern: #"(?m)^\s*\"(?:[^\"\\]|\\.)*\"\s*="#)
            #expect(entryPattern.numberOfMatches(in: source, range: NSRange(source.startIndex..., in: source)) == table.count)
            keySets.append(Set(table.keys))
        }
        #expect(keySets.count == 3)
        #expect(keySets.dropFirst().allSatisfy { $0 == keySets.first })
    }

    @Test("Explicit language determines tab labels and preserves product brands")
    func translatedLabelsAndBrands() {
        let cases: [(AppLanguage, String, String)] = [
            (.english, "Appearance", "Floating Strip"),
            (.simplifiedChinese, "外观", "悬浮条"),
            (.traditionalChinese, "外觀", "懸浮條"),
        ]
        for (language, appearance, strip) in cases {
            let localizer = AppLocalizer(language: language)
            #expect(localizer.text("Appearance") == appearance)
            #expect(localizer.text("Floating Strip") == strip)
            for brand in ["AI Token Meter", "Claude Code", "OpenAI Codex", "DeepSeek", "Google Antigravity"] {
                #expect(localizer.text(brand) == brand)
            }
            #expect(localizer.text("Unknown fallback key") == "Unknown fallback key")
        }
    }

    @Test("Complete notification templates format variables in the translated sentence")
    func notificationTemplates() {
        #expect(AppLocalizer(language: .english).text("%@ usage reached %lld%%.", "Claude Code", Int64(70)) == "Claude Code usage reached 70%.")
        #expect(AppLocalizer(language: .simplifiedChinese).text("%@ usage reached %lld%%.", "Claude Code", Int64(70)) == "Claude Code 的用量已达到 70%。")
        #expect(AppLocalizer(language: .traditionalChinese).text("%@ usage reached %lld%%.", "Claude Code", Int64(90)) == "Claude Code 的用量已達到 90%。")
        #expect(AppLocalizer(language: .english).text("Usage alerts at 70% and 90%") == "Usage alerts at 70% and 90%")
    }

    @Test("Missing translation falls back to English before returning the original key")
    func fallbackUsesEnglish() throws {
        let root = FileManager.default.temporaryDirectory.appending(path: "Localizer-\(UUID().uuidString).bundle")
        defer { try? FileManager.default.removeItem(at: root) }
        for code in ["en", "zh-Hans"] {
            let directory = root.appending(path: "\(code).lproj")
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            let content = code == "en" ? "\"Missing translation\" = \"English fallback\";" : "\"Other\" = \"其他\";"
            try content.write(to: directory.appending(path: "Localizable.strings"), atomically: true, encoding: .utf8)
        }
        let bundle = try #require(Bundle(url: root))
        let localizer = AppLocalizer(language: .simplifiedChinese, resourceBundle: bundle)
        #expect(localizer.text("Other") == "其他")
        #expect(localizer.text("Missing translation") == "English fallback")
        #expect(localizer.text("Absent everywhere") == "Absent everywhere")
        #expect(AppLocalizer(language: .traditionalChinese, resourceBundle: bundle).text("Missing translation") == "English fallback")
    }

    @Test("Application formatters use selected locale for fixed dates and numbers")
    func explicitLocaleFormatting() {
        let date = Date(timeIntervalSince1970: 1_704_067_200) // 2024-01-01 00:00 UTC
        let utc = TimeZone(secondsFromGMT: 0)!
        let english = AppLocalizer(language: .english)
        let simplified = AppLocalizer(language: .simplifiedChinese)
        let traditional = AppLocalizer(language: .traditionalChinese)
        #expect(english.date(date, dateStyle: .long, timeStyle: .none, timeZone: utc) == "January 1, 2024")
        #expect(simplified.date(date, dateStyle: .long, timeStyle: .none, timeZone: utc) == "2024年1月1日")
        #expect(traditional.date(date, dateStyle: .long, timeStyle: .none, timeZone: utc) == "2024年1月1日")
        #expect(english.number(12_345) == "12,345")
        #expect(simplified.number(12_345) == "12,345")
        #expect(traditional.number(12_345) == "12,345")
        // Decimal grouping is identical in these locales; spell-out independently proves locale routing.
        #expect(english.number(12_345, style: .spellOut) == "twelve thousand three hundred forty-five")
        #expect(simplified.number(12_345, style: .spellOut) == "一万二千三百四十五")
        #expect(traditional.number(12_345, style: .spellOut) == "一萬二千三百四十五")
    }
}
