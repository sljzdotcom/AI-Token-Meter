import AIMeterCore
import AppKit
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("Monitoring refresh interval", .serialized)
@MainActor
struct MonitoringRefreshIntervalTests {
    // A static label or disconnected edit action must fail this real native control test.
    @Test func nativeEditorPersistsAndRestores() async throws {
        let suite = "MonitoringRefreshIntervalTests.\(UUID())"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let model = makeModel(defaults)
        let host = NSHostingView(rootView: MonitoringSettingsView(model: model))
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 560, height: 400), styleMask: .borderless, backing: .buffered, defer: false)
        window.contentView = host
        host.frame = window.contentLayoutRect
        let input = try #require(await rendered(host) { $0.compactMap { $0 as? NSTextField }.first { $0.isEditable } })
        #expect(input.stringValue == "300")
        input.stringValue = "60"
        NotificationCenter.default.post(name: NSControl.textDidChangeNotification, object: input)
        let apply = try #require(await rendered(host) { $0.compactMap { $0 as? NSButton }.first { $0.title == "Apply" } })
        apply.performClick(nil)
        #expect(defaults.integer(forKey: "refreshIntervalSeconds") == 60)
        let restored = NSHostingView(rootView: MonitoringSettingsView(model: makeModel(defaults)))
        window.contentView = restored
        restored.frame = window.contentLayoutRect
        let restoredInput = try #require(await rendered(restored) { $0.compactMap { $0 as? NSTextField }.first { $0.isEditable } })
        #expect(restoredInput.stringValue == "60")
    }

    @Test(arguments: ["Return", "Apply"])
    func focusChangesKeepDraftUntilReturnOrApply(_ submission: String) async throws {
        let suite = "MonitoringRefreshIntervalTests.Focus.\(UUID())"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        defaults.set(60, forKey: "refreshIntervalSeconds")
        let model = makeModel(defaults)
        let host = NSHostingView(rootView: MonitoringSettingsView(model: model))
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 560, height: 400), styleMask: .borderless, backing: .buffered, defer: false)
        window.contentView = host
        host.frame = window.contentLayoutRect
        let input = try #require(await rendered(host) { $0.compactMap { $0 as? NSTextField }.first { $0.isEditable } })

        let apply = try #require(await rendered(host) { $0.compactMap { $0 as? NSButton }.first { $0.title == "Apply" } })
        #expect(window.makeFirstResponder(input))
        let editor = try #require(input.currentEditor() as? NSTextView)
        editor.insertText("1234", replacementRange: NSRange(location: 0, length: editor.string.utf16.count))
        #expect(window.makeFirstResponder(nil))
        #expect(input.stringValue == "1234")
        #expect(model.refreshIntervalSeconds == 60)
        #expect(defaults.integer(forKey: "refreshIntervalSeconds") == 60)

        #expect(window.makeFirstResponder(input))
        let returnEditor = try #require(input.currentEditor() as? NSTextView)
        returnEditor.insertText("90", replacementRange: NSRange(location: 0, length: returnEditor.string.utf16.count))
        let returnKey = try #require(NSEvent.keyEvent(
            with: .keyDown, location: .zero, modifierFlags: [], timestamp: 0,
            windowNumber: window.windowNumber, context: nil, characters: "\r",
            charactersIgnoringModifiers: "\r", isARepeat: false, keyCode: 36
        ))
        if submission == "Return" { returnEditor.keyDown(with: returnKey) }
        else { apply.performClick(nil) }
        #expect(model.refreshIntervalSeconds == 90)
        #expect(defaults.integer(forKey: "refreshIntervalSeconds") == 90)
    }

    @Test(arguments: ["", "abc", "30.5", "29", "86401", "30", "300", "86400"])
    func validatesNativeSubmission(_ value: String) async throws {
        let suite = "MonitoringRefreshIntervalTests.Validation.\(UUID())"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        defaults.set(60, forKey: "refreshIntervalSeconds")
        let host = NSHostingView(rootView: MonitoringSettingsView(model: makeModel(defaults)))
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 560, height: 400), styleMask: .borderless, backing: .buffered, defer: false)
        window.contentView = host
        host.frame = window.contentLayoutRect
        let input = try #require(await rendered(host) { $0.compactMap { $0 as? NSTextField }.first { $0.isEditable } })
        input.stringValue = value
        NotificationCenter.default.post(name: NSControl.textDidChangeNotification, object: input)
        // The field's action is the native Return-key submission path.
        #expect(input.sendAction(input.action, to: input.target))
        let accepted = ["30", "300", "86400"].contains(value)
        #expect(defaults.integer(forKey: "refreshIntervalSeconds") == (accepted ? Int(value)! : 60))
        if !accepted {
            let error = await rendered(host) { $0.compactMap { $0 as? NSTextField }.first { !$0.isEditable && $0.stringValue.contains("Enter a whole number") && !$0.isHidden } }
            #expect(error != nil)
        }
    }
}

@MainActor
private func makeModel(_ defaults: UserDefaults) -> AppModel {
    AppModel(defaults: defaults, secretStore: IntervalSecretStore(), widgetSnapshotPublisher: nil,
             isDemoMode: false, refreshOperation: { [] }, serviceAccountRefreshOperation: { _ in [] })
}
private struct IntervalSecretStore: SecretStore {
    func read() throws -> String? { nil }
    func save(_ secret: String) throws {}
    func delete() throws {}
}
@MainActor
private func descendants(_ view: NSView) -> [NSView] {
    view.subviews.flatMap { [$0] + descendants($0) }
}
@MainActor
private func rendered<T>(_ host: NSView, select: ([NSView]) -> T?) async -> T? {
    let deadline = ContinuousClock.now.advanced(by: .seconds(2))
    while ContinuousClock.now < deadline {
        host.window?.layoutIfNeeded()
        host.layoutSubtreeIfNeeded()
        if let result = select(descendants(host)) { return result }
        try? await Task.sleep(for: .milliseconds(10))
    }
    return nil
}
