import AIMeterCore
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
            "%lldm limit": ["70m limit", "70 分钟限额", "70 分鐘限額"],
            "Cached · %lld min ago": ["Cached · 70 min ago", "缓存 · 70 分钟前", "快取 · 70 分鐘前"],
            "%lld%% remaining": ["70% remaining", "剩余 70%", "剩餘 70%"],
            "Resets %@": ["Resets Claude Code", "重置时间：Claude Code", "重設時間：Claude Code"],
            "Resets in %@": ["Resets in Claude Code", "Claude Code 后重置", "Claude Code 後重設"],
            "Source: Antigravity CLI %@ · /usage": ["Source: Antigravity CLI Claude Code · /usage", "来源：Antigravity CLI Claude Code · /usage", "來源：Antigravity CLI Claude Code · /usage"],
            "Official quota, %@, %lld percent used, %@": ["Official quota, Claude Code, 70 percent used, Weekly", "官方额度，Claude Code，已用百分之 70，Weekly", "官方額度，Claude Code，已用百分之 70，Weekly"],
            "Local estimate, %@, %@": ["Local estimate, Claude Code, Weekly", "本地估算，Claude Code，Weekly", "本機估算，Claude Code，Weekly"],
            "Local estimate, %@": ["Local estimate, Claude Code", "本地估算，Claude Code", "本機估算，Claude Code"],
            "%@ total tokens": ["Claude Code total tokens", "共 Claude Code 个 Token", "共 Claude Code 個 Token"],
            "%lld day": ["70 day", "70 天", "70 天"],
            "%lld days": ["70 days", "70 天", "70 天"],
            "%lldd %lldh": ["70d 70h", "70 天 70 小时", "70 天 70 小時"],
            "%lldd": ["70d", "70 天", "70 天"],
            "%lldh %lldm": ["70h 70m", "70 小时 70 分钟", "70 小時 70 分鐘"],
            "%lldh": ["70h", "70 小时", "70 小時"],
            "%lldm": ["70m", "70 分钟", "70 分鐘"],
            "%lld day remaining": ["70 day remaining", "剩余 70 天", "剩餘 70 天"],
            "%lld days remaining": ["70 days remaining", "剩余 70 天", "剩餘 70 天"],
            "%lld available": ["70 available", "70 张可用", "70 張可用"],
            "Sign in on the official DeepSeek page once. %@ keeps the web session on this Mac and stores only daily totals.": ["Sign in on the official DeepSeek page once. Claude Code keeps the web session on this Mac and stores only daily totals.", "请在 DeepSeek 官方页面登录一次。Claude Code 会在此 Mac 上保留网页会话，并仅保存每日汇总。", "請在 DeepSeek 官方頁面登入一次。Claude Code 會在這部 Mac 上保留網頁工作階段，並僅儲存每日彙總。"],
            "Approve the private %@ workspace in Terminal, then refresh.": ["Approve the private Claude Code workspace in Terminal, then refresh.", "请在 Terminal 中授权私有 Claude Code 工作区，然后刷新。", "請在 Terminal 中授權私人 Claude Code 工作區，然後重新整理。"],
            "%@ sign-in could not be opened.": ["Claude Code sign-in could not be opened.", "无法打开 Claude Code 登录。", "無法開啟 Claude Code 登入。"],
            "Complete %@ sign-in in Terminal. Status will update automatically.": ["Complete Claude Code sign-in in Terminal. Status will update automatically.", "请在 Terminal 中完成 Claude Code 登录。状态将自动更新。", "請在 Terminal 中完成 Claude Code 登入。狀態將自動更新。"],
            "%@ account connected.": ["Claude Code account connected.", "Claude Code 账户已连接。", "Claude Code 帳戶已連線。"],
            "%@ tokens": ["Claude Code tokens", "Claude Code 个 Token", "Claude Code 個 Token"],
            "%@ requests": ["Claude Code requests", "Claude Code 次请求", "Claude Code 次請求"],
            "Updated %@": ["Updated Claude Code", "更新于 Claude Code", "更新於 Claude Code"],
            "Quit %@": ["Quit Claude Code", "退出 Claude Code", "結束 Claude Code"],
            "%@, vertical position %lld percent": ["Claude Code, vertical position 70 percent", "Claude Code，垂直位置百分之 70", "Claude Code，垂直位置百分之 70"],
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
                case ["1:@", "2:@"]: rendered = localizer.text(key, "Claude Code", "Weekly")
                case ["1:lld", "2:lld"]: rendered = localizer.text(key, Int64(70), Int64(70))
                case ["1:@", "2:lld", "3:@"]: rendered = localizer.text(key, "Claude Code", Int64(70), "Weekly")
                default:
                    Issue.record("Add a typed invocation for \(key): \(signature)")
                    continue
                }
                #expect(rendered == expected, "\(language.rawValue): \(key)")
            }
        }
    }

    @Test("Settings and meter controls translate every app-owned label and accessibility instruction")
    func settingsAndMeterTranslations() throws {
        // Removing a resource or falling back to English in either Chinese language is a regression.
        let keys = [
            "Display", "Content and Size", "Screen and Position", "Behavior", "Privacy", "Software Update",
            "Refresh now", "Hide for 1 hour", "Settings…", "Quit AI Token Meter", "Show Floating Strip Now",
            "Checking usage", "Waiting for first refresh", "Not installed", "Expand floating meter",
            "Move floating meter", "Use up or down to move. Left and right set the edge preference",
            "Set edge preference to Left", "Set edge preference to Right", "Left edge", "Right edge",
            "Private AI usage monitor", "Apply", "Refresh interval in seconds", "Check Status",
            "Authorize Usage Workspace", "Save API Key", "Replace API Key", "Check for Updates", "Update Now",
        ]
        for language in AppLanguage.allCases {
            let localizer = AppLocalizer(language: language)
            let values = try table(language)
            for key in keys {
                let value = try #require(values[key], "Missing resource: \(language.rawValue), \(key)")
                #expect(localizer.text(key) == value)
                if language != .english { #expect(value != key, "Untranslated: \(key)") }
            }
        }
        #expect(AppLocalizer(language: .simplifiedChinese).text("Expand floating meter") == "展开悬浮用量表")
        #expect(AppLocalizer(language: .traditionalChinese).text("Expand floating meter") == "展開懸浮用量表")
    }

    @Test("Settings and meter source literals have registered resources")
    func settingsSourceResourceInventory() throws {
        // Supplemental inventory gate: runtime tests below verify that registered keys are actually resolved.
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent()
        let files = ["AppearanceSettingsView", "FloatingStripSettingsView", "FloatingStripDisplaySettings",
                     "MonitoringSettingsView", "ServicesSettingsView", "AboutSettingsView",
                     "SoftwareUpdateSettingsView", "MenuBarPanel", "FloatingStripView", "RefreshIntervalEditor"]
        let english = try table(.english)
        let pattern = try NSRegularExpression(pattern: #""([^"\n]*)""#)
        // Product names and currency identifiers retain their spelling; the other entries are SF Symbols/URLs.
        let preserved = Set(["", "CNY", "DeepSeek", "Google Antigravity", "arrow.up", "arrow.down",
                             "checkmark.shield", "gauge.with.dots.needle.50percent", "arrow.clockwise", "gearshape", "power",
                             "https://code.claude.com/docs/en/setup", "https://learn.chatgpt.com/docs/codex/cli"])
        for file in files {
            let source = try String(contentsOf: root.appending(path: "Sources/AIMeterApp/Views/\(file).swift"), encoding: .utf8)
            let text = source as NSString
            for match in pattern.matches(in: source, range: NSRange(location: 0, length: text.length)) {
                let key = text.substring(with: match.range(at: 1))
                #expect(preserved.contains(key) || english[key] != nil, "Unregistered user-facing text: \(file): \(key)")
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


extension AppLocalizationTests {
    @Test("Provider detail vocabulary translates four quota windows and local history without changing brands")
    func providerDetailVocabulary() {
        let cases: [(String, String, String)] = [
            ("Gemini · Five hour", "Gemini · 五小时", "Gemini · 五小時"),
            ("Gemini · Weekly", "Gemini · 每周", "Gemini · 每週"),
            ("Claude/GPT · Five hour", "Claude/GPT · 五小时", "Claude/GPT · 五小時"),
            ("Claude/GPT · Weekly", "Claude/GPT · 每周", "Claude/GPT · 每週"),
            ("Fresh", "最新", "最新"), ("Unavailable", "不可用", "無法使用"),
            ("Official quota", "官方额度", "官方額度"),
            ("Last 30 days · This Mac", "最近 30 天 · 此 Mac", "最近 30 天 · 這部 Mac"),
            ("Local estimate", "本地估算", "本機估算"),
            ("Daily token activity", "每日 Token 活动", "每日 Token 活動"),
            ("Current streak", "连续活跃天数", "連續活躍天數"),
            ("Longest session", "最长会话", "最長工作階段"),
            ("Daily cost (CNY)", "每日费用（CNY）", "每日費用（CNY）"),
            ("Checking account…", "正在检查账户…", "正在檢查帳戶…"),
            ("Reset time unavailable", "重置时间不可用", "無法取得重設時間"),
            ("System Default", "系统默认", "系統預設"),
        ]
        for (key, simplified, traditional) in cases {
            #expect(AppLocalizer(language: .english).text(key) == key)
            #expect(AppLocalizer(language: .simplifiedChinese).text(key) == simplified)
            #expect(AppLocalizer(language: .traditionalChinese).text(key) == traditional)
        }
    }
}


extension AppLocalizationTests {
    @Test("All providers localize fresh cached unavailable and recovery states while preserving diagnostics")
    func providerDynamicStates() {
        let now = Date(timeIntervalSince1970: 1_700_000_000)
        for provider in UsageProvider.allCases {
            for (language, fresh, cached, unavailable, noData) in [
                (AppLanguage.english, "Fresh", "Cached · 2 min ago", "Unavailable", "No current data"),
                (.simplifiedChinese, "最新", "缓存 · 2 分钟前", "不可用", "暂无当前数据"),
                (.traditionalChinese, "最新", "快取 · 2 分鐘前", "無法使用", "暫無目前資料"),
            ] {
                let localizer = AppLocalizer(language: language)
                let current = UsageSnapshot(provider: provider, fetchedAt: now)
                #expect(ProviderDetailText.freshness(current, now: now, localizer: localizer) == fresh)
                let retained = UsageSnapshot(provider: provider,
                    primaryMetric: .init(label: "Current session", current: 23, limit: 100, unit: .percent),
                    fetchedAt: now.addingTimeInterval(-125), collectionStatus: .cached,
                    statusMessage: "External CLI detail: Something went wrong")
                #expect(ProviderDetailText.freshness(retained, now: now, localizer: localizer) == cached)
                let display = AppProviderPresentation(snapshot: retained, localizer: localizer)
                #expect(display.valueText == "23%")
                #expect(display.semantic == .stale)
                #expect(display.statusText == "External CLI detail: Something went wrong")
                let missing = UsageSnapshot(provider: provider, availability: .unavailable, fetchedAt: now, collectionStatus: .unavailable)
                let empty = AppProviderPresentation(snapshot: missing, localizer: localizer)
                #expect(empty.valueText == unavailable)
                #expect(empty.detailText == noData)
            }
        }
    }

    @Test("Local statistics and reset credits translate units without changing rounding or day decisions")
    func localStatisticsAndCredits() {
        let simplified = AppLocalizer(language: .simplifiedChinese)
        let traditional = AppLocalizer(language: .traditionalChinese)
        let summary = CodexLocalActivitySummary(tokenCount: 1_200_000, currentStreakDays: 7, longestSessionDuration: 3_660)
        #expect(ProviderDetailText.localStreak(summary, localizer: simplified) == "7 天")
        #expect(ProviderDetailText.localDuration(summary, localizer: traditional) == "1 小時 1 分鐘")
        #expect(ProviderDetailText.creditStatus(.remaining(days: 2), localizer: simplified) == "剩余 2 天")
        #expect(ProviderDetailText.creditStatus(.today, localizer: traditional) == "今日到期")
        #expect(ProviderDetailText.creditStatus(.expired, localizer: simplified) == "已过期")
        #expect(ProviderDetailText.creditStatus(.unavailable, localizer: traditional) == "無法取得到期時間")
    }
}


extension AppLocalizationTests {
    @Test("Quota labels, percentages, reset dates and accessibility rerender in the selected language")
    func quotaResetAndAccessibility() {
        let date = Date(timeIntervalSince1970: 1_704_067_200)
        let metric = UsageMetric(label: "Gemini · Five hour", current: 20, limit: 100, unit: .percent, resetAt: date)
        for (language, label, remaining) in [
            (AppLanguage.simplifiedChinese, "Gemini · 五小时", "剩余 80%"),
            (.traditionalChinese, "Gemini · 五小時", "剩餘 80%"),
        ] {
            let localizer = AppLocalizer(language: language)
            let presentation = AppProviderPresentation(snapshot: UsageSnapshot(provider: .gemini, primaryMetric: metric), localizer: localizer)
            #expect(presentation.detailText == label)
            #expect(presentation.primaryResetText == localizer.text("Resets %@", localizer.date(date)))
            #expect(GeminiDetailPresentation.remainingText(for: metric, localizer: localizer) == remaining)
            #expect(metric.usedFraction == 0.2)
        }
        let chinese = AppLocalizer(language: .simplifiedChinese)
        #expect(ClaudeDetailPresentation.officialQuotaAccessibilityLabel(metric, resetText: nil, localizer: chinese)
            == "官方额度，Gemini · 五小时，已用百分之 20，重置时间不可用")
        #expect(ClaudeDetailPresentation.localStatAccessibilityLabel(title: "Sessions", value: "7", localizer: chinese)
            == "本地估算，会话数，7")
        #expect(ClaudeDetailPresentation.localActivityAccessibilityLabel(title: "Status", detail: "Local activity unavailable", localizer: chinese)
            == "本地估算，状态，本地活动不可用")
    }
}


extension AppLocalizationTests {
    @Test("System font option localizes but actual font family names are preserved")
    @MainActor
    func fontOptionNames() {
        for (language, expected) in [(AppLanguage.english, "System Default"), (.simplifiedChinese, "系统默认"), (.traditionalChinese, "系統預設")] {
            let localizer = AppLocalizer(language: language)
            #expect(AppearanceSettingsView.fontOptionTitle(.system, localizer: localizer) == expected)
            for font in DisplayFontChoice.allCases where font != .system {
                #expect(AppearanceSettingsView.fontOptionTitle(font, localizer: localizer) == font.displayName)
            }
        }
    }

    @Test("Account status and DeepSeek history translate app-owned states and keep external identity and diagnostics")
    @MainActor
    func accountAndHistoryStates() {
        let simplified = AppLocalizer(language: .simplifiedChinese)
        let traditional = AppLocalizer(language: .traditionalChinese)
        for provider in UsageProvider.allCases {
            for (state, expected) in [(ServiceAccountConnectionState.connected, "已连接"),
                                      (.notInstalled, "未安装 CLI"), (.checking, "正在检查账户…"),
                                      (.unavailable, "账户状态不可用")] {
                #expect(ProviderDetailText.accountText(.init(provider: provider, connectionState: state), localizer: simplified) == expected)
            }
            #expect(ProviderDetailText.accountText(.init(provider: provider, connectionState: .signInRequired), localizer: simplified)
                == (provider == .deepSeek ? "未保存 API Key" : "需要登录"))
            #expect(ProviderDetailText.accountText(.init(provider: provider, connectionState: .connected, accountLabel: "person@example.com"), localizer: simplified) == "person@example.com")
        }
        for (state, expected) in [(DeepSeekWebSession.SyncState.signedOut, "需要登入官網"),
                                  (.loading, "正在同步官方用量…"), (.ready, "官方用量已同步"),
                                  (.stale("Using cached usage"), "使用快取用量"),
                                  (.stale("External website diagnosis: unknown"), "External website diagnosis: unknown")] {
            #expect(ProviderDetailText.deepSeekSync(state, isDemo: false, localizer: traditional) == expected)
        }
        #expect(ProviderDetailText.deepSeekSync(.signedOut, isDemo: true, localizer: simplified) == "预览数据")
        #expect(GeminiInstallationGuide.instructions(for: .notInstalled, localizer: traditional) == [
            "curl -fsSL https://antigravity.google/cli/install.sh | bash", "執行 agy 並完成 Google 登入。", "返回 AI Token Meter 並選擇「重試」。",
        ])
        #expect(GeminiInstallationGuide.instructions(for: .connected, localizer: simplified).isEmpty)
    }

    @Test("Uncommon quota windows and collector-owned failures have localized display text")
    func uncommonProviderStates() {
        let localizer = AppLocalizer(language: .simplifiedChinese)
        #expect(ProviderDetailText.metricLabel("120m limit", localizer: localizer) == "120 分钟限额")
        #expect(ProviderDetailText.metricLabel("Usage limit", localizer: localizer) == "用量限额")
        for (message, translated) in [
            ("Antigravity CLI is not executable", "Antigravity CLI 无法执行"),
            ("Antigravity CLI environment uses an unsupported override", "Antigravity CLI 环境使用了不支持的覆盖设置"),
            ("Antigravity CLI version is not supported (requires 1.1.28 or later in major version 1)", "不支持此 Antigravity CLI 版本（需要主版本 1 中的 1.1.28 或更高版本）"),
        ] {
            let presentation = AppProviderPresentation(snapshot: .init(provider: .gemini, availability: .unavailable, collectionStatus: .unavailable, statusMessage: message), localizer: localizer)
            #expect(presentation.statusText == translated)
        }
    }
}


extension AppLocalizationTests {
    @Test("Raw diagnostics and account identity never collide with unrelated interface translation keys")
    @MainActor
    func diagnosticsDoNotUseGeneralUIKeys() {
        let localizer = AppLocalizer(language: .simplifiedChinese)
        let snapshot = UsageSnapshot(provider: .claude, availability: .unavailable, collectionStatus: .unavailable, statusMessage: "Appearance")
        #expect(AppProviderPresentation(snapshot: snapshot, localizer: localizer).statusText == "Appearance")
        #expect(ProviderDetailText.accountText(.init(provider: .claude, connectionState: .connected, accountLabel: "Appearance"), localizer: localizer) == "Appearance")
        #expect(ProviderDetailText.deepSeekSync(.stale("Appearance"), isDemo: false, localizer: localizer) == "Appearance")
        let metric = UsageMetric(label: "Appearance", current: 20, limit: 100, unit: .percent, resetDescription: "Appearance")
        #expect(ProviderDetailText.metricLabel(metric.label, localizer: localizer) == "Appearance")
        #expect(ProviderDetailText.reset(metric, localizer: localizer) == "Appearance")
    }
}


extension AppLocalizationTests {
    @Test("Reset descriptions localize standard units and retain each provider's timestamp precedence")
    func resetDescriptionsAndTimestampPrecedence() {
        let localizer = AppLocalizer(language: .traditionalChinese)
        for (description, expected) in [("Resets in 51 min", "51 分鐘 後重設"), ("Resets in 3 hours", "3 小時 後重設"), ("Resets in 3h 12m", "3 小時 12 分鐘 後重設")] {
            let metric = UsageMetric(label: "Current session", current: 20, limit: 100, unit: .percent, resetDescription: description)
            #expect(ProviderDetailText.reset(metric, localizer: localizer) == expected)
            #expect(ProviderDetailText.reset(metric, localizer: AppLocalizer(language: .english)) == description)
        }
        let date = Date(timeIntervalSince1970: 1_704_067_200)
        let both = UsageMetric(label: "Current session", current: 20, limit: 100, unit: .percent, resetAt: date, resetDescription: "Resets Friday")
        #expect(ProviderDetailText.reset(both, localizer: localizer) == "週五重設")
        #expect(ProviderDetailText.reset(both, localizer: localizer, prefersTimestamp: true) == localizer.text("Resets %@", localizer.date(date)))
    }
}
