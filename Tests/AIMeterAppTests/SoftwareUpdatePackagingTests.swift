import Foundation
import Testing

@Suite("Software update packaging")
struct SoftwareUpdatePackagingTests {
    @Test("Release metadata opts into only manual signed updates")
    func updateMetadata() throws {
        let plist = try loadInfoPlist()

        #expect(plist["CFBundleShortVersionString"] as? String == "0.2.0")
        #expect(plist["CFBundleVersion"] as? String == "4")
        #expect(
            plist["SUFeedURL"] as? String
                == "https://raw.githubusercontent.com/sljzdotcom/AI-Token-Meter/main/appcast.xml"
        )
        let publicKey = try #require(plist["SUPublicEDKey"] as? String)
        #expect(!publicKey.isEmpty)
        #expect(!publicKey.localizedCaseInsensitiveContains("placeholder"))
        #expect(plist["SUEnableAutomaticChecks"] as? Bool == false)
        #expect(plist["SUAutomaticallyUpdate"] as? Bool == false)
    }

    @Test("Widget metadata stays aligned with the host release")
    func widgetVersionMetadata() throws {
        let data = try Data(contentsOf: Self.widgetInfoPlistURL)
        let plist = try #require(
            PropertyListSerialization.propertyList(from: data, format: nil)
                as? [String: Any]
        )

        #expect(plist["CFBundleShortVersionString"] as? String == "0.2.0")
        #expect(plist["CFBundleVersion"] as? String == "4")
    }

    @Test("Build embeds and explicitly signs Sparkle before the host app")
    func buildContract() throws {
        let source = try String(
            contentsOf: Self.projectRoot.appending(path: "scripts/build-app.sh"),
            encoding: .utf8
        )

        #expect(source.contains("FRAMEWORKS_DIR=\"$CONTENTS_DIR/Frameworks\""))
        #expect(source.contains("Sparkle.framework"))
        #expect(source.contains("copy_sparkle_framework"))
        #expect(source.contains("codesign_sparkle_framework"))
        #expect(source.contains("verify-update-bundle.sh"))

        let sparkleSigning = try #require(source.range(of: "codesign_sparkle_framework"))
        let mainSigning = try #require(source.range(of: "codesign_main_app"))
        #expect(sparkleSigning.lowerBound < mainSigning.lowerBound)
    }

    @Test("Update bundle verifier checks helpers linkage metadata and strict signatures")
    func verifierContract() throws {
        let source = try String(
            contentsOf: Self.projectRoot.appending(path: "scripts/verify-update-bundle.sh"),
            encoding: .utf8
        )

        #expect(source.contains("SPARKLE_FRAMEWORK=\"$CONTENTS_DIR/Frameworks/Sparkle.framework\""))
        #expect(source.contains("Updater.app/Contents/MacOS/Updater"))
        #expect(source.contains("XPCServices/Downloader.xpc"))
        #expect(source.contains("XPCServices/Installer.xpc"))
        #expect(source.contains("SUFeedURL"))
        #expect(source.contains("SUPublicEDKey"))
        #expect(source.contains("otool -L"))
        #expect(source.contains("@rpath/Sparkle.framework"))
        #expect(source.contains("codesign --verify --deep --strict"))
        #expect(source.contains(".build"))
    }

    @Test("Release pipeline creates a signed GitHub update archive and appcast")
    func releasePipelineContract() throws {
        let source = try String(
            contentsOf: Self.projectRoot.appending(path: "scripts/package-update-release.sh"),
            encoding: .utf8
        )

        #expect(source.contains("SPARKLE_TOOLS_DIR"))
        #expect(source.contains("AI-Token-Meter-${VERSION}-macOS-arm64.zip"))
        #expect(source.contains("scripts/test.sh"))
        #expect(source.contains("check-public-release.sh"))
        #expect(source.contains("AI_METER_INCLUDE_WIDGET=0"))
        #expect(source.contains("verify-update-bundle.sh"))
        #expect(source.contains("verify-update-archive.sh"))
        #expect(source.contains("ditto -c -k --keepParent"))
        #expect(source.contains("shasum -a 256"))
        #expect(source.contains("generate_appcast"))
        #expect(source.contains("KEY_ACCOUNT=\"com.millerpan.AIMeter\""))
        #expect(source.contains("--account \"$KEY_ACCOUNT\""))
        #expect(source.contains("releases/download/v${VERSION}/"))
    }

    @Test("Archive verifier accepts the signed release and rejects a tampered copy")
    func archiveVerifierContract() throws {
        let source = try String(
            contentsOf: Self.projectRoot.appending(path: "scripts/verify-update-archive.sh"),
            encoding: .utf8
        )

        #expect(source.contains("xmllint --xpath"))
        #expect(source.contains("sign_update"))
        #expect(source.contains("--verify"))
        #expect(source.contains("tampered"))
        #expect(source.contains("PlistBuddy"))
        #expect(source.contains("CFBundleShortVersionString"))
        #expect(source.contains("CFBundleVersion"))
    }

    @Test("Stable appcast advertises the signed 0.2.0 release")
    func stableAppcastContract() throws {
        let source = try String(
            contentsOf: Self.projectRoot.appending(path: "appcast.xml"),
            encoding: .utf8
        )

        #expect(source.contains("<sparkle:version>4</sparkle:version>"))
        #expect(source.contains("<sparkle:shortVersionString>0.2.0</sparkle:shortVersionString>"))
        #expect(source.contains("sparkle:edSignature="))
        #expect(source.contains("length="))
        #expect(source.contains("releases/download/v0.2.0/AI-Token-Meter-0.2.0-macOS-arm64.zip"))
        #expect(source.contains("<sparkle:minimumSystemVersion>14.0</sparkle:minimumSystemVersion>"))
    }

    private func loadInfoPlist() throws -> [String: Any] {
        let data = try Data(contentsOf: Self.infoPlistURL)
        return try #require(
            PropertyListSerialization.propertyList(from: data, format: nil)
                as? [String: Any]
        )
    }

    private static let projectRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()

    private static let infoPlistURL = projectRoot
        .appending(path: "Sources/AIMeterApp/Resources/Info.plist")

    private static let widgetInfoPlistURL = projectRoot
        .appending(path: "Sources/AIMeterWidgetExtension/Resources/Info.plist")
}
