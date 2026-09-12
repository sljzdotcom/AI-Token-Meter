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
            let url = try tableURL(language)
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
            var content = code == "en" ? "\"Missing translation\" = \"English fallback\";" : "\"Other\" = \"其他\";"
            content += "\n" + (code == "en"
                ? #""Order %@ %lld%%" = "%@: %lld%%";"#
                : #""Order %@ %lld%%" = "%2$lld%%：%1$@";"#)
            try content.write(to: directory.appending(path: "Localizable.strings"), atomically: true, encoding: .utf8)
        }
        let bundle = try #require(Bundle(url: root))
        let localizer = AppLocalizer(language: .simplifiedChinese, resourceBundle: bundle)
        #expect(localizer.text("Other") == "其他")
        #expect(localizer.text("Order %@ %lld%%", "Claude Code", Int64(70)) == "70%：Claude Code")
        #expect(localizer.text("Missing translation") == "English fallback")
        #expect(localizer.text("Absent everywhere") == "Absent everywhere")
        #expect(AppLocalizer(language: .traditionalChinese, resourceBundle: bundle).text("Missing translation") == "English fallback")
    }

    @Test("All translated format strings preserve argument positions and types")
    func allFormatSignaturesMatch() throws {
        let english = try table(.english)
        for language in AppLanguage.allCases {
            let translated = try table(language)
            for (key, source) in english {
                let value = try #require(translated[key])
                #expect(try formatSignature(source) == formatSignature(value), "\(language.rawValue): \(key)")
            }
        }
        // Position-based signatures permit reordering but reject missing, extra and retyped arguments.
        #expect(try formatSignature("%@ %lld%%") == formatSignature("%2$lld%%：%1$@"))
        #expect(try formatSignature("%@ %lld%%") != formatSignature("%lld%%"))
        #expect(try formatSignature("%@ %lld%%") != formatSignature("%@ %@%%"))
        #expect(try formatSignature("%@ %lld%%") != formatSignature("%@ %lld %@%%"))
        #expect(try formatSignature("%lld%%") != formatSignature("%lld"))
        #expect(try formatSignature("%%") == ["literal:%"])
        #expect(try formatSignature("%2$lld %1$@ %2$lld") == ["1:@", "2:lld", "2:lld"])
    }

    @Test("Every shipped dynamic template renders its arguments and literal percent signs")
    func allDynamicTemplatesRender() throws {
        let examples: [String: [String]] = [
            "Move %@ up": ["Move Claude Code up", "上移 Claude Code", "將 Claude Code 上移"],
            "Move %@ down": ["Move Claude Code down", "下移 Claude Code", "將 Claude Code 下移"],
            "Show delay: %lld ms": ["Show delay: 70 ms", "显示延迟：70 毫秒", "顯示延遲：70 毫秒"],
            "Hide delay: %lld ms": ["Hide delay: 70 ms", "隐藏延迟：70 毫秒", "隱藏延遲：70 毫秒"],
            "%lld seconds": ["70 seconds", "70 秒", "70 秒"],
            "Open %@ at login": ["Open Claude Code at login", "登录时打开 Claude Code", "登入時開啟 Claude Code"],
            "%@ usage reached %lld%%.": ["Claude Code usage reached 70%.", "Claude Code 的用量已达到 70%。", "Claude Code 的用量已達到 70%。"],
            "Usage reached %lld%%": ["Usage reached 70%", "用量已达到 70%", "用量已達到 70%"],
            "%@ · %@ is at %lld%%.": ["Claude Code · Weekly is at 70%.", "Claude Code · Weekly 的用量已达到 70%。", "Claude Code · Weekly 的用量已達到 70%。"],
        ]
        let english = try table(.english)
        let dynamicKeys = try Set(english.filter { try formatSignature($0.value).contains { $0 != "literal:%" } }.keys)
        #expect(dynamicKeys == Set(examples.keys))
        for (index, language) in AppLanguage.allCases.enumerated() {
            let localizer = AppLocalizer(language: language)
            let translated = try table(language)
            for key in dynamicKeys {
                let expected = try #require(examples[key]?[index])
                let sourceSignature = try formatSignature(try #require(english[key]))
                let translatedSignature = try formatSignature(try #require(translated[key]))
                guard sourceSignature == translatedSignature else {
                    Issue.record("Unsafe translated format in \(language.rawValue): \(key)")
                    continue
                }
                let signature = sourceSignature.filter { $0 != "literal:%" }
                let rendered: String
                switch signature {
                case ["1:@"]: rendered = localizer.text(key, "Claude Code")
                case ["1:lld"]: rendered = localizer.text(key, Int64(70))
                case ["1:@", "2:lld"]: rendered = localizer.text(key, "Claude Code", Int64(70))
                case ["1:@", "2:@", "3:lld"]: rendered = localizer.text(key, "Claude Code", "Weekly", Int64(70))
                default:
                    Issue.record("Add a typed invocation for \(key): \(signature)")
                    continue
                }
                #expect(rendered == expected, "\(language.rawValue): \(key)")
            }
        }
    }

    private func table(_ language: AppLanguage) throws -> [String: String] {
        let data = try Data(contentsOf: tableURL(language))
        return try #require(PropertyListSerialization.propertyList(from: data, format: nil) as? [String: String])
    }

    private func formatSignature(_ text: String) throws -> [String] {
        // App resources use printf object/numeric conversions, numeric widths/precision,
        // optional positions and %% escapes. A plain percent in a non-format label is literal.
        let pattern = try NSRegularExpression(pattern: #"%%|%(?:([1-9][0-9]*)\$)?[-+#0]*[0-9]*(?:\.[0-9]+)?(hh|ll|[hlqztjL])?([@diuoxXfFeEgGaAcCsSp])"#)
        let string = text as NSString
        var nextPosition = 1
        var signature: [String] = []
        for match in pattern.matches(in: text, range: NSRange(location: 0, length: string.length)) {
            if string.substring(with: match.range) == "%%" {
                signature.append("literal:%")
                continue
            }
            let position: Int
            if match.range(at: 1).location != NSNotFound {
                position = Int(string.substring(with: match.range(at: 1)))!
            } else {
                position = nextPosition
                nextPosition += 1
            }
            let length = match.range(at: 2).location == NSNotFound ? "" : string.substring(with: match.range(at: 2))
            let conversion = string.substring(with: match.range(at: 3))
            signature.append("\(position):\(length)\(conversion == "i" ? "d" : conversion)")
        }
        return signature.sorted()
    }

    @Test("Chinese resources resolve on a real case-sensitive volume")
    func caseSensitiveResourceLookup() throws {
        let root = FileManager.default.temporaryDirectory.appending(path: "LocalizerCase-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let image = root.appending(path: "case.dmg")
        let mount = root.appending(path: "volume")
        try FileManager.default.createDirectory(at: mount, withIntermediateDirectories: true)
        try diskImage(["create", "-size", "16m", "-fs", "HFSX", "-volname", "LocalizerCase", image.path])
        try diskImage(["attach", "-nobrowse", "-mountpoint", mount.path, image.path])
        defer { try? diskImage(["detach", mount.path]) }
        let bundleURL = mount.appending(path: "Fixture.bundle")
        for language in AppLanguage.allCases {
            let directory = bundleURL.appending(path: "\(language.rawValue.lowercased()).lproj")
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            try FileManager.default.copyItem(at: tableURL(language), to: directory.appending(path: "Localizable.strings"))
        }
        // Prove the fixture actually distinguishes casing, even on a case-insensitive host.
        #expect(!FileManager.default.fileExists(atPath: bundleURL.appending(path: "zh-Hans.lproj").path))
        let bundle = try #require(Bundle(url: bundleURL))
        #expect(AppLocalizer(language: .simplifiedChinese, resourceBundle: bundle).text("Appearance") == "外观")
        #expect(AppLocalizer(language: .traditionalChinese, resourceBundle: bundle).text("Appearance") == "外觀")
    }

    private func tableURL(_ language: AppLanguage) throws -> URL {
        let root = try #require(Bundle.module.resourceURL)
        let directory = try #require(FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: nil)
            .first { $0.lastPathComponent.caseInsensitiveCompare("\(language.rawValue).lproj") == .orderedSame })
        return directory.appending(path: "Localizable.strings")
    }

    private func diskImage(_ arguments: [String]) throws {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/hdiutil")
        process.arguments = arguments
        let output = Pipe()
        process.standardOutput = output
        process.standardError = output
        try process.run()
        let data = output.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else {
            throw NSError(domain: "AppLocalizationTests.hdiutil", code: Int(process.terminationStatus),
                          userInfo: [NSLocalizedDescriptionKey: String(decoding: data, as: UTF8.self)])
        }
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
