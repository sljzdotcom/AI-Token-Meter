import AIMeterCore
import AppKit
import Foundation
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("Menu bar provider visibility")
@MainActor
struct MenuBarVisibilityTests {
    @Test("The menu model follows visibility and order without deleting snapshots")
    func visibilityAndOrderUpdateImmediately() async throws {
        let context = makeContext("Immediate")
        defer { context.defaults.removePersistentDomain(forName: context.suiteName) }
        let snapshots = [
            snapshot(.claude, percent: 95),
            snapshot(.codex, percent: 20),
            snapshot(.deepSeek, percent: 30),
            snapshot(.gemini, percent: 40),
        ]
        let model = AppModel(
            defaults: context.defaults,
            secretStore: MenuBarVisibilitySecretStore(),
            widgetSnapshotPublisher: nil,
            isDemoMode: false,
            refreshOperation: { snapshots }
        )
        await model.refresh()

        var preferences = model.stripPreferences
        preferences.orderedProviders = [.gemini, .deepSeek, .codex, .claude]
        preferences.hiddenProviders = [.claude]
        model.setStripPreferences(preferences)

        #expect(model.menuBarSnapshots.map(\.provider) == [.gemini, .deepSeek, .codex])
        #expect(model.menuBarSummary.usageFraction == 0.4)
        #expect(model.snapshots.count == 4)

        preferences.hiddenProviders = []
        model.setStripPreferences(preferences)

        #expect(model.menuBarSnapshots.map(\.provider) == [.gemini, .deepSeek, .codex, .claude])
        #expect(model.menuBarSummary.usageFraction == 0.95)
    }

    @Test("A hosted menu panel and label update after visibility changes")
    func hostedMenuUpdatesImmediately() async throws {
        NSApplication.shared.accessibilitySetValue(
            true,
            forAttribute: NSAccessibility.Attribute(rawValue: "AXEnhancedUserInterface")
        )
        let context = makeContext("Hosted")
        defer { context.defaults.removePersistentDomain(forName: context.suiteName) }
        let snapshots = [
            snapshot(.claude, percent: 95),
            snapshot(.codex, percent: 20),
            snapshot(.deepSeek, percent: 30),
            snapshot(.gemini, percent: 40),
        ]
        let model = AppModel(
            defaults: context.defaults,
            secretStore: MenuBarVisibilitySecretStore(),
            widgetSnapshotPublisher: nil,
            isDemoMode: false,
            refreshOperation: { snapshots }
        )
        await model.refresh()
        let host = NSHostingView(rootView: VStack {
            MenuBarLabel(model: model)
            MenuBarPanel(model: model)
        })
        let window = localizationTestWindow(host, width: 420, height: 820)
        defer { window.close() }

        var preferences = model.stripPreferences
        preferences.orderedProviders = [.gemini, .deepSeek, .codex, .claude]
        preferences.hiddenProviders = [.claude]
        model.setStripPreferences(preferences)

        var output: [String] = []
        for _ in 0..<40 {
            try await settleLocalizationHost(host)
            output = hostedAccessibilityStrings(host)
            if visibleProviderOrder(in: output) == [.gemini, .deepSeek, .codex],
               output.contains("AI Token Meter, highest usage 40 percent") {
                break
            }
        }
        #expect(visibleProviderOrder(in: output) == [.gemini, .deepSeek, .codex])
        #expect(output.contains("AI Token Meter, highest usage 40 percent"))

        preferences.hiddenProviders = []
        model.setStripPreferences(preferences)
        for _ in 0..<40 {
            try await settleLocalizationHost(host)
            output = hostedAccessibilityStrings(host)
            if visibleProviderOrder(in: output) == [.gemini, .deepSeek, .codex, .claude],
               output.contains("AI Token Meter, highest usage 95 percent") {
                break
            }
        }
        #expect(visibleProviderOrder(in: output) == [.gemini, .deepSeek, .codex, .claude])
        #expect(output.contains("AI Token Meter, highest usage 95 percent"))
    }

    @Test("Visibility and order survive model recreation")
    func visibilityAndOrderPersist() async throws {
        let context = makeContext("Persistence")
        defer { context.defaults.removePersistentDomain(forName: context.suiteName) }
        let snapshots = UsageProvider.allCases.map { snapshot($0, percent: 25) }
        let first = AppModel(
            defaults: context.defaults,
            secretStore: MenuBarVisibilitySecretStore(),
            widgetSnapshotPublisher: nil,
            isDemoMode: false,
            refreshOperation: { snapshots }
        )
        var preferences = first.stripPreferences
        preferences.orderedProviders = [.deepSeek, .claude, .gemini, .codex]
        preferences.hiddenProviders = [.gemini]
        first.setStripPreferences(preferences)

        let restored = AppModel(
            defaults: context.defaults,
            secretStore: MenuBarVisibilitySecretStore(),
            widgetSnapshotPublisher: nil,
            isDemoMode: false,
            refreshOperation: { snapshots }
        )
        await restored.refresh()

        #expect(restored.menuBarSnapshots.map(\.provider) == [.deepSeek, .claude, .codex])
        #expect(restored.snapshots.map(\.provider).contains(.gemini))
    }

    private func snapshot(_ provider: UsageProvider, percent: Double) -> UsageSnapshot {
        UsageSnapshot(
            provider: provider,
            primaryMetric: UsageMetric(
                label: "Usage",
                current: percent,
                limit: 100,
                unit: .percent
            )
        )
    }

    private func visibleProviderOrder(in output: [String]) -> [UsageProvider] {
        let names = UsageProvider.allCases.map { ($0, $0.displayName + ", ") }
        return output.compactMap { value in
            names.first(where: { value.hasPrefix($0.1) })?.0
        }
    }

    private func makeContext(_ suffix: String) -> (suiteName: String, defaults: UserDefaults) {
        let suiteName = "MenuBarVisibilityTests.\(suffix).\(UUID().uuidString)"
        return (suiteName, UserDefaults(suiteName: suiteName)!)
    }
}

private struct MenuBarVisibilitySecretStore: SecretStore {
    func read() throws -> String? { nil }
    func save(_ secret: String) throws {}
    func delete() throws {}
}
