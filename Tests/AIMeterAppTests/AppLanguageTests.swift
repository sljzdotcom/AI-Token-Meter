import AppKit
import Foundation
import Observation
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("App language")
struct AppLanguageTests {
    @Test("Language preference defaults to English and accepts only supported persisted values")
    func languagePreferenceDefaultsAndPersistsSupportedValues() throws {
        let suiteName = "AppLanguageTests.Preference.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let store = AppLanguagePreferenceStore(defaults: defaults)

        #expect(store.load() == .english)
        defaults.set("unknown", forKey: AppLanguagePreferenceStore.key)
        #expect(store.load() == .english)

        for language in [.english, .simplifiedChinese, .traditionalChinese] as [AppLanguage] {
            store.save(language)
            #expect(store.load() == language)
        }
    }

    @Test("Changing the app language updates the model immediately and survives reconstruction")
    @MainActor
    func appModelLanguagePreference() throws {
        let suiteName = "AppLanguageTests.Model.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let model = AppModel(defaults: defaults, widgetSnapshotPublisher: nil)
        #expect(model.appLanguage == .english)

        model.setAppLanguage(.english)
        #expect(defaults.object(forKey: AppLanguagePreferenceStore.key) == nil)

        model.setAppLanguage(.simplifiedChinese)
        #expect(model.appLanguage == .simplifiedChinese)

        let reconstructedModel = AppModel(defaults: defaults, widgetSnapshotPublisher: nil)
        #expect(reconstructedModel.appLanguage == .simplifiedChinese)
    }

    @Test("Language observers are notified once per changed value and never for repeated choices")
    @MainActor
    func languageObservationDeduplicates() throws {
        let suiteName = "AppLanguageTests.Observation.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let model = AppModel(defaults: defaults, widgetSnapshotPublisher: nil)
        let recorder = LanguageChangeRecorder()

        withObservationTracking { _ = model.appLanguage } onChange: { recorder.record() }
        model.setAppLanguage(.english)
        #expect(recorder.count == 0)
        model.setAppLanguage(.simplifiedChinese)
        #expect(recorder.count == 1)

        withObservationTracking { _ = model.appLanguage } onChange: { recorder.record() }
        model.setAppLanguage(.simplifiedChinese)
        #expect(recorder.count == 1)
        model.setAppLanguage(.traditionalChinese)
        #expect(recorder.count == 2)
    }


    @Test("An existing hosted language root delivers the selected Locale without recreation",
          .enabled(if: ProcessInfo.processInfo.environment["AI_METER_SCREEN_TESTS"] == "1"))
    @MainActor
    func existingHostUpdatesLocale() async throws {
        let suiteName = "AppLanguageTests.Host.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let model = AppModel(defaults: defaults, widgetSnapshotPublisher: nil)
        let recorder = LocaleRecorder()
        let host = NSHostingView(rootView: AppLanguageRoot(model: model) {
            LocaleProbe(recorder: recorder)
        })
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 160, height: 80),
                              styleMask: [.borderless], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = host
        window.orderFrontRegardless()
        defer { window.close() }
        for (language, identifier) in [(AppLanguage.english, "en"), (.simplifiedChinese, "zh-Hans"),
                                       (.traditionalChinese, "zh-Hant"), (.english, "en")] {
            model.setAppLanguage(language)
            for _ in 0..<50 where recorder.identifier != identifier {
                host.layoutSubtreeIfNeeded()
                try await Task.sleep(for: .milliseconds(20))
            }
            #expect(recorder.identifier == identifier)
            #expect(window.contentView === host)
        }
    }

    @Test("Existing floating strip and detail roots keep the shared language model",
          .enabled(if: ProcessInfo.processInfo.environment["AI_METER_SCREEN_TESTS"] == "1"))
    @MainActor
    func existingControllerRootsUseLatestLocale() throws {
        let suiteName = "AppLanguageTests.Controller.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let model = AppModel(defaults: defaults, widgetSnapshotPublisher: nil, isDemoMode: true)
        let controller = FloatingPanelController(model: model)
        defer { controller.close() }
        let panels = Mirror(reflecting: controller).children
        let strip = try #require(panels.first { $0.label == "stripPanel" }?.value as? NSPanel)
        let detail = try #require(panels.first { $0.label == "detailPanel" }?.value as? NSPanel)
        controller.showDetail(for: .claude)
        let stripHost = try #require(strip.contentView as? NSHostingView<AppLanguageRoot<FloatingStripView>>)
        let detailHost = try #require(detail.contentView as? NSHostingView<AppLanguageRoot<FloatingDetailView>>)
        for (language, identifier) in [(AppLanguage.simplifiedChinese, "zh-Hans"), (.traditionalChinese, "zh-Hant")] {
            model.setAppLanguage(language)
            #expect(stripHost.rootView.locale.identifier == identifier)
            #expect(detailHost.rootView.locale.identifier == identifier)
            #expect(strip.contentView === stripHost)
            #expect(detail.contentView === detailHost)
        }
    }

}

private final class LanguageChangeRecorder: @unchecked Sendable {
    private let lock = NSLock()
    private var value = 0
    func record() { lock.withLock { value += 1 } }
    var count: Int { lock.withLock { value } }
}


@MainActor
private final class LocaleRecorder {
    var identifier: String?
}

private struct LocaleProbe: NSViewRepresentable {
    @Environment(\.locale) private var locale
    let recorder: LocaleRecorder
    func makeNSView(context: Context) -> NSView { NSView() }
    func updateNSView(_ nsView: NSView, context: Context) {
        recorder.identifier = locale.identifier
    }
}
