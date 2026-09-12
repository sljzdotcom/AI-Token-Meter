import AIMeterCore
import AppKit
import Foundation
import SwiftUI
import Testing
import Vision
@testable import AIMeterApp

@Suite("Parser-derived provider localization", .serialized)
struct ProviderParserLocalizationTests {
    @Test("Actual Claude weekly labels map the parser's trimmed parentheses to translated titles")
    func realClaudeWeeklyLabels() throws {
        let input = try fixture("claude-usage-promo-en")
        let snapshot = try ClaudeUsageParser().parse(input)
        let weekly = try #require(snapshot.secondaryMetric)
        #expect(weekly.label == "Current week (all models")
        let sonnet = try #require(ClaudeUsageParser().parse(input.replacingOccurrences(of: "(all models)", with: "(Sonnet only)")).secondaryMetric)
        #expect(sonnet.label == "Current week (Sonnet only")
        for (language, all, one) in [(AppLanguage.simplifiedChinese, "本周（所有模型）", "本周（仅 Sonnet）"),
                                    (.traditionalChinese, "本週（所有模型）", "本週（僅 Sonnet）")] {
            let localizer = AppLocalizer(language: language)
            #expect(ProviderDetailText.metricLabel(weekly.label, localizer: localizer) == all)
            #expect(ProviderDetailText.metricLabel(sonnet.label, localizer: localizer) == one)
        }
        #expect(weekly.usedFraction == 0)
        #expect(weekly.label == "Current week (all models")
        #expect(ProviderDetailText.metricLabel(weekly.label, localizer: .init(language: .english)) == weekly.label)
    }

    @Test("Real Claude reset dates format their wall time and stated timezone in each app language")
    func realClaudeResetDates() throws {
        let snapshot = try ClaudeUsageParser().parse(fixture("claude-usage-promo-en"))
        let ordinary = try ClaudeUsageParser().parse(fixture("claude-usage-en"))
        for (language, primary, weekly, weekday) in [
            (AppLanguage.simplifiedChinese, "重置时间：20:10 (Asia/Singapore)", "重置时间：9月5日 15:59 (Asia/Singapore)", "重置时间：周四 00:00"),
            (.traditionalChinese, "重設時間：下午8:10 (Asia/Singapore)", "重設時間：9月5日 下午3:59 (Asia/Singapore)", "重設時間：週四上午12:00"),
        ] {
            let localizer = AppLocalizer(language: language)
            let presentation = AppProviderPresentation(snapshot: snapshot, localizer: localizer)
            #expect(presentation.primaryResetText == primary)
            #expect(presentation.secondaryResetText == weekly)
            #expect(AppProviderPresentation(snapshot: ordinary, localizer: localizer).secondaryResetText == weekday)
        }
        let english = AppProviderPresentation(snapshot: snapshot, localizer: .init(language: .english))
        #expect(english.primaryResetText == "Resets 8:10pm (Asia/Singapore)")
        #expect(english.secondaryResetText == "Resets Sep 5 at 3:59pm (Asia/Singapore)")
        #expect(snapshot.primaryMetric?.resetAt == nil)
        #expect(snapshot.primaryMetric?.resetDescription == "Resets 8:10pm (Asia/Singapore)")
        #expect(snapshot.secondaryMetric?.resetDescription == "Resets Sep 5 at 3:59pm (Asia/Singapore)")
    }

    @Test("Codex parser reset times localize without inferring a reset date")
    func realCodexResetTimes() throws {
        let snapshot = try CodexUsageParser().parse(fixture("codex-status-en"))
        let simplified = AppProviderPresentation(snapshot: snapshot, localizer: .init(language: .simplifiedChinese))
        let traditional = AppProviderPresentation(snapshot: snapshot, localizer: .init(language: .traditionalChinese))
        #expect(simplified.primaryResetText == "重置时间：22:30")
        #expect(simplified.secondaryResetText == "重置时间：周五 01:00")
        #expect(traditional.primaryResetText == "重設時間：下午10:30")
        #expect(traditional.secondaryResetText == "重設時間：週五上午1:00")
        #expect(snapshot.primaryMetric?.resetDescription == "Resets 10:30 PM")
        #expect(snapshot.secondaryMetric?.resetAt == nil)
    }

    @Test("Traditional weekday reset spacing is stable across macOS formatter versions")
    func traditionalWeekdaySpacingIsStable() {
        #expect(ProviderDetailText.normalizeResetDateSpacing("週四 上午12:00", language: .traditionalChinese) == "週四上午12:00")
        #expect(ProviderDetailText.normalizeResetDateSpacing("週五 下午1:00", language: .traditionalChinese) == "週五下午1:00")
        #expect(ProviderDetailText.normalizeResetDateSpacing("9月5日 下午3:59", language: .traditionalChinese) == "9月5日 下午3:59")
        #expect(ProviderDetailText.normalizeResetDateSpacing("周四 00:00", language: .simplifiedChinese) == "周四 00:00")
    }

    @Test("Invalid and unrecognized textual reset dates stay verbatim")
    func unrecognizedResetDatesArePreserved() {
        for raw in ["Resets vendor-defined time", "Resets Feb 30 at 3:59pm (Asia/Singapore)",
                    "Resets 29:10", "Resets Thu 12:75 AM", "Resets 0:10pm", "Resets 8:10pm (unknown-zone)"] {
            let metric = UsageMetric(label: "Current session", current: 0, limit: 100, unit: .percent, resetDescription: raw)
            for language in AppLanguage.allCases {
                #expect(ProviderDetailText.reset(metric, localizer: .init(language: language)) == raw)
            }
        }
    }

    @Test("Real no-email Claude account results translate known auth methods and preserve external identities")
    func realClaudeAccountFallbacks() throws {
        for (method, english, simplified, traditional) in [
            ("oauth", "OAuth account", "OAuth 账户", "OAuth 帳戶"),
            ("claude.ai", "Claude Code account", "Claude Code 账户", "Claude Code 帳戶"),
            ("api_key", "API Key account", "API Key 账户", "API Key 帳戶"),
        ] {
            let status = try ClaudeAccountStatusParser().parse("{\"loggedIn\":true,\"authMethod\":\"\(method)\"}")
            #expect(status.accountLabel == english)
            for (language, expected) in [(AppLanguage.english, english), (.simplifiedChinese, simplified), (.traditionalChinese, traditional)] {
                #expect(ProviderDetailText.accountText(status, localizer: .init(language: language)) == expected)
            }
        }
        for input in [#"{"loggedIn":true,"authMethod":"External provider"}"#,
                      #"{"loggedIn":true,"authMethod":"oauth","email":"Appearance"}"#] {
            let status = try ClaudeAccountStatusParser().parse(input)
            #expect(ProviderDetailText.accountText(status, localizer: .init(language: .simplifiedChinese)) == status.accountLabel)
        }
    }

    @Test("Actual Claude and account views render parser-derived Chinese titles dates and identities",
          .enabled(if: ProcessInfo.processInfo.environment["AI_METER_SCREEN_TESTS"] == "1"))
    @MainActor
    func parserDerivedViewsRenderChinese() async throws {
        let snapshot = try ClaudeUsageParser().parse(fixture("claude-usage-promo-en"))
        for (language, week, account) in [(AppLanguage.simplifiedChinese, "本周（所有模型）", "OAuth账户"),
                                        (.traditionalChinese, "本週（所有模型）", "OAuth帳戶")] {
            let labels = try await renderedText(
                ClaudeDetailView(snapshot: snapshot, onSetup: {}, onOpenServicesSettings: {}),
                language: language, name: "claude-\(language.rawValue)", width: 650, height: 550
            )
            #expect(labels.contains(week), "Missing weekly title in \(labels)")
            #expect(labels.contains("9月5日"), "Missing localized reset date in \(labels)")
            #expect(!labels.contains("Currentweek"))
            #expect(!labels.contains("Sep5"))
            #expect(!labels.lowercased().contains("3:59pm"))
            #expect(!labels.lowercased().contains("8:10pm"))
            let status = try ClaudeAccountStatusParser().parse(#"{"loggedIn":true,"authMethod":"oauth"}"#)
            let accountLabels = try await renderedText(ServiceAccountStatusView(status: status), language: language,
                                                       name: "account-\(language.rawValue)", width: 400, height: 90)
            #expect(accountLabels.contains(account), "Missing account label in \(accountLabels)")
        }
    }

    private func fixture(_ name: String) throws -> String {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent()
        return try String(contentsOf: root.appending(path: "Tests/AIMeterCoreTests/Fixtures/\(name).txt"), encoding: .utf8)
    }

    @MainActor
    private func renderedText<V: View>(_ view: V, language: AppLanguage, name: String,
                                       width: CGFloat, height: CGFloat) async throws -> String {
        let host = NSHostingView(rootView: view.environment(\.locale, language.locale).environment(\.colorScheme, .dark))
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: width, height: height),
                              styleMask: [.borderless], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = host
        defer { window.close() }
        host.layoutSubtreeIfNeeded()
        try await Task.sleep(for: .milliseconds(100))
        let bitmap = try #require(NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(width * 2),
            pixelsHigh: Int(height * 2), bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
            colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0))
        bitmap.size = NSSize(width: width, height: height)
        host.cacheDisplay(in: host.bounds, to: bitmap)
        if let directory = ProcessInfo.processInfo.environment["AI_METER_DOC_SCREENSHOT_DIR"] {
            let folder = URL(fileURLWithPath: directory)
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            try #require(bitmap.representation(using: .png, properties: [:])).write(to: folder.appending(path: "task-5-fix-\(name).png"))
        }
        let request = VNRecognizeTextRequest()
        request.recognitionLevel = .accurate
        request.recognitionLanguages = [language.rawValue, "en-US"]
        request.usesLanguageCorrection = false
        try VNImageRequestHandler(cgImage: #require(bitmap.cgImage)).perform([request])
        return (request.results ?? []).compactMap { $0.topCandidates(1).first?.string }
            .joined(separator: " ").filter { !$0.isWhitespace }
    }
}
