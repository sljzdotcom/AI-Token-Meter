@testable import AIMeterCore
import AppKit
import Foundation
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("Settings localization coverage", .serialized,
       .enabled(if: ProcessInfo.processInfo.environment["AI_METER_SCREEN_TESTS"] == "1"))
@MainActor
struct SettingsLocalizationCoverageTests {
    @Test("Services and floating detail replace the one-time action with the same explicit resume in all languages")
    func antigravityExplicitResumeUsesBothExistingSurfaces() async throws {
        for language in AppLanguage.allCases {
            let name = "AntigravityResumeControls.\(UUID())"
            let defaults = try #require(UserDefaults(suiteName: name))
            let directory = FileManager.default.temporaryDirectory.appending(path: name)
            defer {
                defaults.removePersistentDomain(forName: name)
                try? FileManager.default.removeItem(at: directory)
            }
            let metric = UsageMetric(label: "Gemini · Weekly", current: 22, limit: 100, unit: .percent)
            let quota = UsageSnapshot(provider: .gemini, primaryMetric: metric, sourceVersion: "1.1.28", geminiQuotaMetrics: [metric])
            let authorization = GeminiAutomaticRefreshResumeAuthorization(id: UUID())
            let gate = LocalizedResumeGate()
            defer { Task { await gate.release() } }
            var token: String?
            let model = AppModel(
                defaults: defaults, secretStore: LocalizationSecretStore(),
                launchAtLoginStatusOperation: { false }, widgetSnapshotPublisher: nil, isDemoMode: false,
                geminiDiagnosticStore: GeminiDiagnosticStore(fileURL: nil), refreshOperation: { [quota] },
                geminiPauseReasonOperation: { .unknown }, geminiRecoveryBeginOperation: { _ in true },
                geminiRecoveryAuthorizeOperation: { _ in GeminiQuotaRecoveryAuthorization(id: UUID()) },
                geminiQuotaCheckOperation: { _ in quota },
                geminiResumeAuthorizationOperation: { _ in authorization },
                geminiAutomaticRefreshResumeOperation: { _ in await gate.hold(); return quota },
                geminiResumeInvalidationOperation: {}, geminiAuthenticationOpenOperation: { token = $0 },
                geminiAuthenticationCancelOperation: { _ in }, geminiAuthenticationStopOperation: { _ in Task { true } },
                geminiAuthenticationFailureOperation: { _ in false }, geminiAuthenticationExecConfirmedOperation: { _ in true },
                cacheDirectoryURL: directory
            )
            defer { model.stop() }
            model.setAppLanguage(language)
            await model.refresh()
            let services = NSHostingView(rootView: AppLanguageRoot(model: model) {
                ServicesSettingsView(model: model, pendingAPIKey: .constant(""))
            })
            let detail = NSHostingView(rootView: AppLanguageRoot(model: model) {
                FloatingDetailView(model: model, provider: .gemini, onClaudeSetup: {}, onOpenServicesSettings: {}, onInteractionChange: { _ in })
            })
            let servicesWindow = localizationTestWindow(services)
            let detailWindow = localizationTestWindow(detail, width: 440, height: 680)
            defer { servicesWindow.close(); detailWindow.close() }
            let localizer = AppLocalizer(language: language)
            let checkTitle = localizer.text("One-time quota check")
            let resumeTitle = localizer.text("Resume automatic refresh")
            let diagnosticsTitle = localizer.text("Copy diagnostic info")
            for host in [services as NSView, detail as NSView] {
                try await settleLocalizationHost(host)
                #expect(serviceButtons(in: host, title: checkTitle).count == 1)
                #expect(serviceButtons(in: host, title: resumeTitle).isEmpty)
                #expect(serviceButtons(in: host, title: diagnosticsTitle).count == 1)
            }
            let check = try #require(serviceButtons(in: services, title: checkTitle).first)
            #expect(hostedAccessibilityAttribute(check, "accessibilityPerformPress") as? Bool == true)
            let loginDeadline = ContinuousClock.now.advanced(by: .seconds(2))
            while token == nil, ContinuousClock.now < loginDeadline { await Task.yield() }
            await model.completeGeminiInteractiveSignIn(token: try #require(token), result: .success)
            for host in [services as NSView, detail as NSView] {
                try await settleLocalizationHost(host)
                #expect(serviceButtons(in: host, title: checkTitle).isEmpty)
                let resume = try #require(serviceButtons(in: host, title: resumeTitle).first)
                #expect(hostedAccessibilityAttribute(resume, "isAccessibilityEnabled") as? Bool == true)
                #expect(serviceButtons(in: host, title: resumeTitle).count == 1)
                #expect(serviceButtons(in: host, title: diagnosticsTitle).count == 1)
            }
            let resume = try #require(serviceButtons(in: detail, title: resumeTitle).first)
            #expect(hostedAccessibilityAttribute(resume, "accessibilityPerformPress") as? Bool == true)
            let started = await gate.waitUntilStarted(timeout: .seconds(2))
            if !started { await gate.release() }
            try #require(started)
            for host in [services as NSView, detail as NSView] {
                try await settleLocalizationHost(host)
                let busyResume = try #require(serviceButtons(in: host, title: resumeTitle).first)
                #expect(hostedAccessibilityAttribute(busyResume, "isAccessibilityEnabled") as? Bool == false)
                #expect(serviceButtons(in: host, title: checkTitle).isEmpty)
            }
            #expect(model.isGeminiRefreshPaused)
            await gate.release()
            let completionDeadline = ContinuousClock.now.advanced(by: .seconds(2))
            while model.isGeminiAutomaticRefreshResumeInProgress, ContinuousClock.now < completionDeadline { await Task.yield() }
            #expect(!model.isGeminiRefreshPaused)
            #expect(model.settingsNotice == .recoveryResumeSucceeded)
            #expect(await gate.calls == 1)
            for host in [services as NSView, detail as NSView] {
                try await settleLocalizationHost(host)
                #expect(serviceButtons(in: host, title: resumeTitle).isEmpty)
                #expect(serviceButtons(in: host, title: checkTitle).isEmpty)
            }
        }
    }

    @Test("Resume notices translate fixed outcomes without implying a new quota check or repaired cause")
    func antigravityResumeNoticesUseAllThreeLanguages() {
        let notices: [SettingsNotice] = [.recoveryQuotaReadyToResume, .recoveryResumeStarted, .recoveryResumeBusy,
                                        .recoveryResumeValidationRequired, .recoveryResumePersistenceFailed, .recoveryResumeSucceeded]
        for notice in notices {
            let english = notice.text(using: AppLocalizer(language: .english))
            #expect(!english.isEmpty)
            for language in [AppLanguage.simplifiedChinese, .traditionalChinese] {
                let translated = notice.text(using: AppLocalizer(language: language))
                #expect(!translated.isEmpty)
                #expect(translated != english)
            }
        }
        #expect(SettingsNotice.recoveryResumeSucceeded.text(using: AppLocalizer(language: .english))
            == "Antigravity automatic refresh is enabled. Quota will be checked at the next scheduled refresh.")
    }

    // Removing a Section logo, swapping its provider, using white in light mode,
    // or clipping its calibrated silhouette must fail against the real form pixels.
    @Test("Services renders all four matching decorative logos in light and dark appearances")
    func serviceHeaderLogosRenderInBothAppearances() async throws {
        let name = "ServiceHeaderLogos.\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        let model = AppModel(defaults: defaults, secretStore: LocalizationSecretStore(), widgetSnapshotPublisher: nil)
        defer { model.stop() }
        let providers: [(UsageProvider, String)] = [
            (.claude, "Claude Code"), (.codex, "OpenAI Codex"),
            (.deepSeek, "DeepSeek"), (.gemini, "Google Antigravity"),
        ]
        for scheme in [ColorScheme.light, .dark] {
            let host = NSHostingView(rootView: AppLanguageRoot(model: model) {
                ServicesSettingsView(model: model, pendingAPIKey: .constant(""))
            }.environment(\.colorScheme, scheme))
            let window = localizationTestWindow(host)
            window.appearance = NSAppearance(named: scheme == .light ? .aqua : .darkAqua)
            defer { window.close() }
            for language in AppLanguage.allCases {
                model.setAppLanguage(language)
                try await settleLocalizationHost(host)
                if let output = ProcessInfo.processInfo.environment["AI_METER_SERVICES_LOGO_ARTIFACTS"], language == .english {
                    let directory = URL(fileURLWithPath: output)
                    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
                    try headerBitmap(host, rect: host.bounds).representation(using: .png, properties: [:])?
                        .write(to: directory.appending(path: "services-\(scheme).png"))
                }
                for (provider, title) in providers {
                    let titleNodes = hostedAccessibilityObjects(host).filter { object in
                        ["accessibilityLabel", "accessibilityValue"].contains {
                            hostedAccessibilityAttribute(object, $0) as? String == title
                        }
                    }
                    #expect(titleNodes.count == 1, "\(title) must be announced exactly once")
                    let node = try #require(titleNodes.first)
                    let screenFrame = try #require(
                        hostedAccessibilityAttribute(node, "accessibilityFrame") as? NSValue).rectValue
                    let titleFrame = host.convert(window.convertFromScreen(screenFrame), from: nil)
                    // 18pt logo + 6pt gap, centered vertically on the title; the
                    // 32pt crop includes optical overshoot and proves no clipping.
                    let crop = NSRect(x: titleFrame.minX - 31, y: titleFrame.midY - 16, width: 32, height: 32)
                    #expect(host.bounds.contains(crop))
                    let actual = try headerBitmap(host, rect: crop)
                    let referenceHost = NSHostingView(rootView: ProviderLogo(provider: provider, size: 18, tint: .primary)
                        .frame(width: 32, height: 32).environment(\.colorScheme, scheme))
                    let referenceWindow = localizationTestWindow(referenceHost, width: 32, height: 32)
                    defer { referenceWindow.close() }
                    try await settleLocalizationHost(referenceHost)
                    let reference = try headerBitmap(referenceHost, rect: referenceHost.bounds)
                    if let output = ProcessInfo.processInfo.environment["AI_METER_SERVICES_LOGO_ARTIFACTS"], language == .english {
                        let directory = URL(fileURLWithPath: output)
                        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
                        try actual.representation(using: .png, properties: [:])?.write(to: directory.appending(path: "\(provider.rawValue)-\(scheme)-actual.png"))
                        try reference.representation(using: .png, properties: [:])?.write(to: directory.appending(path: "\(provider.rawValue)-\(scheme)-reference.png"))
                    }
                    let expectedMask = try headerInkMask(reference, transparent: true, scheme: scheme)
                    let actualMask = try headerInkMask(actual, transparent: false, scheme: scheme)
                    let expectedCount = expectedMask.filter { $0 }.count
                    let actualCount = actualMask.filter { $0 }.count
                    let intersection = zip(expectedMask, actualMask).filter { $0 && $1 }.count
                    let union = zip(expectedMask, actualMask).filter { $0 || $1 }.count
                    #expect(expectedCount > 20)
                    #expect(actualCount > 20, "Missing visible \(title) logo in \(scheme)")
                    #expect(Double(intersection) / Double(expectedCount) > 0.88,
                        "\(title) logo must retain its full silhouette in \(scheme): \(intersection)/\(expectedCount)")
                    #expect(Double(intersection) / Double(max(union, 1)) > 0.78,
                        "\(title) must use its matching 18pt logo with a 6pt gap in \(scheme)")
                }
                #expect(window.contentView === host)
            }
        }
    }

    private func headerBitmap(_ host: NSView, rect: NSRect) throws -> NSBitmapImageRep {
        let bitmap = try #require(NSBitmapImageRep(bitmapDataPlanes: nil,
            pixelsWide: Int(rect.width * 2), pixelsHigh: Int(rect.height * 2), bitsPerSample: 8, samplesPerPixel: 4,
            hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0))
        bitmap.size = rect.size
        host.cacheDisplay(in: rect, to: bitmap)
        return bitmap
    }

    private func headerInkMask(_ bitmap: NSBitmapImageRep, transparent: Bool, scheme: ColorScheme) throws -> [Bool] {
        let background = try #require(bitmap.colorAt(x: 0, y: 0)?.usingColorSpace(.deviceRGB))
        return try (0..<64).flatMap { y in
            try (0..<64).map { x in
                let color = try #require(bitmap.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB))
                if transparent { return color.alphaComponent > 0.5 }
                return abs(color.redComponent - background.redComponent) > abs((scheme == .light ? 0 : 1) - background.redComponent) * 0.5
            }
        }
    }

    // Bypassing localization for a coordinator status or interpolated version
    // must fail on the exact value consumed by the existing hosted controls.
    @Test("Update events and installed version translate in the same existing host")
    func updateStatesTranslateInPlace() async throws {
        let name = "UpdateLanguage.\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        let model = AppModel(defaults: defaults, secretStore: LocalizationSecretStore(), widgetSnapshotPublisher: nil)
        let engine = CoverageUpdateEngine()
        let coordinator = SoftwareUpdateCoordinator(engine: engine, currentVersion: "1.2.3", currentBuild: "7")
        defer { coordinator.stop() }
        let host = NSHostingView(rootView: AppLanguageRoot(model: model) {
            Form { SoftwareUpdateSettingsView(coordinator: coordinator) }.formStyle(.grouped)
        })
        let window = localizationTestWindow(host, height: 450)
        defer { window.close() }
        let release = SoftwareUpdateRelease(version: "2.0.0", build: "8", publishedAt: nil, summary: "External release summary")
        let steps: [(() -> Void, [String])] = [
            ({ engine.eventHandler?(.cancelled(nil)) }, ["Not checked yet", "尚未检查", "尚未檢查"]),
            ({ coordinator.checkForUpdates() }, ["Checking…", "正在检查…", "正在檢查…"]),
            ({ engine.eventHandler?(.noUpdate) }, ["You’re up to date", "已是最新版本", "已是最新版本"]),
            ({ engine.eventHandler?(.found(release)) }, ["Version 2.0.0 is available", "版本 2.0.0 可用", "版本 2.0.0 可用"]),
            ({ coordinator.installAvailableUpdate() }, ["Preparing version 2.0.0…", "正在准备版本 2.0.0…", "正在準備版本 2.0.0…"]),
            ({ engine.eventHandler?(.failed(.offline)) }, ["You appear to be offline.", "当前似乎处于离线状态。", "目前似乎處於離線狀態。"]),
            ({ engine.eventHandler?(.failed(.timedOut)) }, ["The update check timed out.", "检查更新超时。", "檢查更新逾時。"]),
            ({ engine.eventHandler?(.failed(.invalidFeed)) }, ["The update information is unavailable.", "无法获取更新信息。", "無法取得更新資訊。"]),
            ({ engine.eventHandler?(.failed(.invalidSignature)) }, ["The update could not be verified.", "无法验证此更新。", "無法驗證此更新。"]),
            ({ engine.eventHandler?(.failed(.permissionDenied)) }, ["The update could not be installed in Applications.", "无法将更新安装到应用程序文件夹。", "無法將更新安裝到應用程式檔案夾。"]),
            ({ engine.eventHandler?(.failed(.configuration)) }, ["Software updates are not configured correctly.", "软件更新配置不正确。", "軟體更新設定不正確。"]),
            ({ engine.eventHandler?(.failed(.other)) }, ["The update check failed. Try again later.", "检查更新失败。请稍后重试。", "檢查更新失敗。請稍後再試。"]),
        ]
        for (advance, expected) in steps {
            advance()
            for (index, language) in AppLanguage.allCases.enumerated() {
                model.setAppLanguage(language)
                try await settleLocalizationHost(host)
                let text = hostedAccessibilityStrings(host).joined(separator: "\n")
                #expect(text.contains(expected[index]), "Missing \(expected[index]) in \(text)")
                #expect(text.contains(index == 0 ? "Version 1.2.3 (7)" : "版本 1.2.3（7）"))
                if coordinator.state.availableRelease != nil { #expect(text.contains("External release summary")) }
                #expect(window.contentView === host)
                try assertHostedRendering(host)
            }
        }
    }

    @Test("The update page translates missing installed-version metadata")
    func missingInstalledVersionTranslatesInPlace() async throws {
        let name = "MissingUpdateLanguage.\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        let model = AppModel(defaults: defaults, secretStore: LocalizationSecretStore(), widgetSnapshotPublisher: nil)
        for (version, build) in [("", ""), ("Unavailable", "Unavailable")] {
            let coordinator = SoftwareUpdateCoordinator(engine: CoverageUpdateEngine(), currentVersion: version, currentBuild: build)
            defer { coordinator.stop() }
            let host = NSHostingView(rootView: AppLanguageRoot(model: model) {
                Form { SoftwareUpdateSettingsView(coordinator: coordinator) }.formStyle(.grouped)
            })
            let window = localizationTestWindow(host, height: 350)
            defer { window.close() }
            for (language, expected) in [(AppLanguage.english, "Version unavailable"), (.simplifiedChinese, "版本不可用"), (.traditionalChinese, "無法取得版本")] {
                model.setAppLanguage(language)
                try await settleLocalizationHost(host)
                #expect(hostedAccessibilityStrings(host).contains(expected))
                #expect(window.contentView === host)
            }
        }
    }

    @Test("About translates present and unavailable bundle versions without replacing its host")
    func aboutVersionsTranslateInPlace() async throws {
        let fixtures: [([String: Any], [String])] = [
            ([:], ["Version unavailable", "版本不可用", "無法取得版本"]),
            (["CFBundleShortVersionString": "", "CFBundleVersion": "7"], ["Version unavailable", "版本不可用", "無法取得版本"]),
            (["CFBundleShortVersionString": "3.2.1", "CFBundleVersion": "9"], ["Version 3.2.1 (9)", "版本 3.2.1（9）", "版本 3.2.1（9）"]),
        ]
        for (info, expected) in fixtures {
            let name = "AboutLanguage.\(UUID())"
            let defaults = try #require(UserDefaults(suiteName: name))
            defer { defaults.removePersistentDomain(forName: name) }
            let model = AppModel(defaults: defaults, secretStore: LocalizationSecretStore(), widgetSnapshotPublisher: nil)
            let coordinator = SoftwareUpdateCoordinator(engine: CoverageUpdateEngine(), currentVersion: "1.2.3", currentBuild: "7")
            defer { coordinator.stop() }
            let host = NSHostingView(rootView: AppLanguageRoot(model: model) { AboutSettingsView(updateCoordinator: coordinator, versionInfo: info) })
            let window = localizationTestWindow(host, height: 700)
            defer { window.close() }
            for (index, language) in AppLanguage.allCases.enumerated() {
                model.setAppLanguage(language)
                try await settleLocalizationHost(host)
                #expect(hostedAccessibilityStrings(host).contains(expected[index]))
                #expect(window.contentView === host)
            }
        }
    }

    @Test("Actual Claude and Codex primary buttons translate every account and busy state")
    func serviceActionsTranslateInPlace() async throws {
        for (state, expected) in [
            (ServiceAccountConnectionState.notInstalled, ["Install CLI", "安装 CLI", "安裝 CLI"]),
            (.connected, ["Sign in again", "重新登录", "重新登入"]),
            (.unavailable, ["Check Status", "检查状态", "檢查狀態"]),
            (.signInRequired, ["Sign in", "登录", "登入"]),
            (.checking, ["Checking account…", "正在检查账户…", "正在檢查帳戶…"]),
        ] {
            let name = "ServiceLanguage.\(UUID())"
            let defaults = try #require(UserDefaults(suiteName: name))
            defer { defaults.removePersistentDomain(forName: name) }
            var authenticated: [UsageProvider] = []
            let model = AppModel(defaults: defaults, secretStore: LocalizationSecretStore(), widgetSnapshotPublisher: nil,
                serviceAccountRefreshOperation: { _ in [.init(provider: .claude, connectionState: state), .init(provider: .codex, connectionState: state)] },
                authenticationOpenOperation: { authenticated.append($0) }, signInSleep: { _ in try await Task.sleep(for: .seconds(30)) })
            defer { model.stop() }
            if state != .checking { await model.refreshServiceAccounts() }
            #expect(model.serviceAccounts[.claude]?.connectionState == state)
            #expect(model.serviceAccounts[.codex]?.connectionState == state)
            #expect(model.serviceAction(for: .claude).title == expected[0])
            #expect(model.serviceAction(for: .codex).title == expected[0])
            let host = NSHostingView(rootView: AppLanguageRoot(model: model) { ServicesSettingsView(model: model, pendingAPIKey: .constant("")) })
            let window = localizationTestWindow(host)
            defer { window.close() }
            for (index, language) in AppLanguage.allCases.enumerated() {
                model.setAppLanguage(language)
                try await settleLocalizationHost(host)
                let buttons = serviceButtons(in: host, title: expected[index])
                let actualButtons = hostedAccessibilityObjects(host).filter {
                    hostedAccessibilityAttribute($0, "accessibilityRole") as? String == "AXButton"
                }.map { button in
                    ["accessibilityLabel", "accessibilityTitle", "accessibilityValue", "isAccessibilityEnabled"]
                        .map { "\($0)=\(String(describing: hostedAccessibilityAttribute(button, $0) ?? "<nil>"))" }
                        .joined(separator: ", ")
                }
                #expect(buttons.count == 2,
                    "Both CLI controls must expose \(expected[index]); state=\(state), language=\(language.rawValue), AX buttons=\(actualButtons)")
                #expect(buttons.allSatisfy { (hostedAccessibilityAttribute($0, "isAccessibilityEnabled") as? Bool) == (state != .checking) })
                #expect(model.serviceAction(for: .claude).isEnabled == (state != .checking))
                #expect(model.serviceAction(for: .codex).isEnabled == (state != .checking))
                #expect(window.contentView === host)
            }
            if state == .connected {
                for button in serviceButtons(in: host, title: "重新登入") {
                    // SwiftUI AccessibilityNode implements the AX selectors without
                    // declaring NSAccessibilityProtocol conformance; KVC boxes BOOL.
                    let pressed = try #require(hostedAccessibilityAttribute(button, "accessibilityPerformPress") as? Bool)
                    #expect(pressed)
                }
                try await settleLocalizationHost(host)
                #expect(authenticated == [.claude, .codex])
                for (index, language) in AppLanguage.allCases.enumerated() {
                    model.setAppLanguage(language)
                    try await settleLocalizationHost(host)
                    let expected = ["Waiting for Terminal…", "正在等待 Terminal…", "正在等待 Terminal…"][index]
                    let buttons = serviceButtons(in: host, title: expected)
                    #expect(buttons.count == 2)
                    #expect(buttons.allSatisfy { (hostedAccessibilityAttribute($0, "isAccessibilityEnabled") as? Bool) == false })
                    #expect(!model.serviceAction(for: .claude).isEnabled)
                    #expect(!model.serviceAction(for: .codex).isEnabled)
                }
            }
        }
    }

    @Test("Selected display keeps its system name and identity while translating all suffix combinations")
    func displayChoicesTranslateInPlace() async throws {
        let name = "DisplayLanguage.\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        let model = AppModel(defaults: defaults, secretStore: LocalizationSecretStore(), widgetSnapshotPublisher: nil)
        let choices = [
            FloatingStripDisplayChoice(id: "built-main", name: "Fixture Display", isPrimary: true, isBuiltIn: true),
            .init(id: "built-second", name: "Fixture Display", isPrimary: false, isBuiltIn: true),
            .init(id: "external-main", name: "Fixture Display", isPrimary: true, isBuiltIn: false),
            .init(id: "external-second", name: "Fixture Display", isPrimary: false, isBuiltIn: false),
        ]
        model.updateAvailableStripDisplays(choices)
        let host = NSHostingView(rootView: AppLanguageRoot(model: model) { Form { FloatingStripDisplaySettings(model: model) }.formStyle(.grouped) })
        let window = localizationTestWindow(host, height: 400)
        defer { window.close() }
        let expected = [
            ["Fixture Display · Built-in · Primary", "Fixture Display · 内置 · 主显示器", "Fixture Display · 內建 · 主顯示器"],
            ["Fixture Display · Built-in", "Fixture Display · 内置", "Fixture Display · 內建"],
            ["Fixture Display · External · Primary", "Fixture Display · 外接 · 主显示器", "Fixture Display · 外接 · 主顯示器"],
            ["Fixture Display · External", "Fixture Display · 外接", "Fixture Display · 外接"],
        ]
        for (choiceIndex, choice) in choices.enumerated() {
            model.selectFloatingStripDisplay(choice.id)
            for (index, language) in AppLanguage.allCases.enumerated() {
                model.setAppLanguage(language)
                try await settleLocalizationHost(host)
                let labels = hostedAccessibilityStrings(host)
                #expect(labels.contains(expected[choiceIndex][index]), "Missing display \(expected[choiceIndex][index]) in \(labels)")
                #expect(model.floatingStripDisplays.selectedIdentifier == choice.id)
                #expect(model.availableStripDisplays == choices)
                #expect(window.contentView === host)
            }
        }
    }

    private func serviceButtons(in host: NSView, title: String) -> [NSObject] {
        hostedAccessibilityObjects(host).filter { object in
            hostedAccessibilityAttribute(object, "accessibilityRole") as? String == "AXButton"
                && hostedAccessibilityAttribute(object, "accessibilityLabel") as? String == title
        }
    }
}

private actor LocalizedResumeGate {
    private(set) var calls = 0
    private var isReleased = false
    private var continuations: [CheckedContinuation<Void, Never>] = []
    func hold() async {
        calls += 1
        if !isReleased { await withCheckedContinuation { continuations.append($0) } }
    }
    func waitUntilStarted(timeout: Duration) async -> Bool {
        let deadline = ContinuousClock.now.advanced(by: timeout)
        while calls == 0 {
            guard !Task.isCancelled, ContinuousClock.now < deadline else { return false }
            await Task.yield()
        }
        return true
    }
    func release() {
        isReleased = true
        let waiting = continuations
        continuations.removeAll()
        waiting.forEach { $0.resume() }
    }
}

private struct LocalizationSecretStore: SecretStore {
    func read() throws -> String? { nil }
    func save(_ secret: String) throws {}
    func delete() throws {}
}

@MainActor
private final class CoverageUpdateEngine: SoftwareUpdateEngine {
    var eventHandler: ((SoftwareUpdateEvent) -> Void)?
    var canCheckForUpdates = true
    func start() throws {}
    func checkForUpdateInformation() {}
    func presentAvailableUpdate() {}
    func stop() {}
}
