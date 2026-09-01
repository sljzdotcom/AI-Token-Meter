import Foundation
import Testing

@Suite("Widget build packaging contract")
struct WidgetBuildScriptTests {
    @Test("Build script embeds only a properly signed Widget extension")
    func buildScriptContract() throws {
        let source = try String(contentsOf: Self.projectRoot.appending(path: "scripts/build-app.sh"))

        #expect(source.contains("AI_METER_INCLUDE_WIDGET"))
        #expect(source.contains("AI_METER_CODESIGN_IDENTITY"))
        #expect(source.contains("AI_METER_WIDGET_TEAM_ID"))
        #expect(source.contains("AIMeterWidgetExtension"))
        #expect(source.contains("Contents/PlugIns/AITokenMeterWidget.appex"))
        #expect(source.contains("Widget skipped"))
        #expect(source.contains("Widget requested but Apple Development signing is unavailable"))
        #expect(source.contains(".com.millerpan.AIMeter"))

        let extensionSigning = try #require(source.range(of: "codesign_widget_extension"))
        let appSigning = try #require(source.range(of: "codesign_main_app"))
        #expect(extensionSigning.lowerBound < appSigning.lowerBound)
    }

    @Test("Verifier checks the executable, shared group, sandbox, and nested signatures")
    func verifierContract() throws {
        let source = try String(contentsOf: Self.projectRoot.appending(path: "scripts/verify-widget-bundle.sh"))

        #expect(source.contains("AIMeterWidgetExtension"))
        #expect(source.contains("com.apple.security.application-groups"))
        #expect(source.contains("com.apple.security.app-sandbox"))
        #expect(source.contains("codesign --verify --deep --strict"))
        #expect(source.contains("AIWidgetAppGroupIdentifier"))
    }

    @Test("Both source entitlement templates declare the same App Group placeholder")
    func entitlementTemplates() throws {
        let app = try String(contentsOf: Self.projectRoot.appending(
            path: "Sources/AIMeterApp/Resources/AIMeterApp.entitlements"
        ))
        let widget = try String(contentsOf: Self.projectRoot.appending(
            path: "Sources/AIMeterWidgetExtension/Resources/AITokenMeterWidget.entitlements"
        ))

        #expect(app.contains("$(AI_WIDGET_APP_GROUP_IDENTIFIER)"))
        #expect(widget.contains("$(AI_WIDGET_APP_GROUP_IDENTIFIER)"))
        #expect(widget.contains("com.apple.security.app-sandbox"))
    }

    private static let projectRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
}
