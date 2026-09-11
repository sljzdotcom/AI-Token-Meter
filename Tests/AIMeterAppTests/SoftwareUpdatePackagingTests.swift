import Foundation
import Testing

@Suite("Software update packaging")
struct SoftwareUpdatePackagingTests {
    @Test("Release metadata opts into only manual signed updates")
    func updateMetadata() throws {
        let plist = try loadInfoPlist()

        #expect(plist["CFBundleShortVersionString"] as? String == "0.8.1")
        #expect(plist["CFBundleVersion"] as? String == "24")
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

        #expect(plist["CFBundleShortVersionString"] as? String == "0.8.1")
        #expect(plist["CFBundleVersion"] as? String == "24")
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

    @Test("Cross-platform drafts publish the checked-in versioned release notes")
    func crossPlatformReleaseNotesContract() throws {
        let source = try String(
            contentsOf: Self.projectRoot.appending(path: "scripts/package-cross-platform-release.sh"),
            encoding: .utf8
        )

        #expect(source.contains("RELEASE_NOTES=\"$PROJECT_DIR/docs/releases/v$VERSION.md\""))
        #expect(source.contains("--notes-file \"$RELEASE_NOTES\""))
        #expect(!source.contains("--generate-notes"))
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

    @Test("Stable appcast advertises internally consistent signed releases")
    func stableAppcastContract() throws {
        let data = try Data(contentsOf: Self.projectRoot.appending(path: "appcast.xml"))

        #expect(try validateStableAppcast(data).isEmpty)
    }

    @Test("Stable appcast validation rejects corrupt release metadata", arguments: [
        ("version URL mismatch", "0.5.1", "13", "v0.5.0/AI-Token-Meter-0.5.0-macOS-arm64.zip", Self.validFixtureSignature, "42", "14.0"),
        ("invalid build", "0.5.0", "zero", "v0.5.0/AI-Token-Meter-0.5.0-macOS-arm64.zip", Self.validFixtureSignature, "42", "14.0"),
        ("invalid signature", "0.5.0", "13", "v0.5.0/AI-Token-Meter-0.5.0-macOS-arm64.zip", "not base64!", "42", "14.0"),
        ("short decoded signature", "0.5.0", "13", "v0.5.0/AI-Token-Meter-0.5.0-macOS-arm64.zip", "c2hvcnQ=", "42", "14.0"),
        ("invalid length", "0.5.0", "13", "v0.5.0/AI-Token-Meter-0.5.0-macOS-arm64.zip", Self.validFixtureSignature, "0", "14.0"),
        ("invalid minimum OS", "0.5.0", "13", "v0.5.0/AI-Token-Meter-0.5.0-macOS-arm64.zip", Self.validFixtureSignature, "42", "13.0"),
    ])
    func corruptStableAppcastIsRejected(
        _ label: String,
        _ version: String,
        _ build: String,
        _ archivePath: String,
        _ signature: String,
        _ length: String,
        _ minimumSystemVersion: String
    ) throws {
        let fixture = """
        <?xml version="1.0"?>
        <rss xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle" version="2.0">
          <channel><item>
            <sparkle:version>\(build)</sparkle:version>
            <sparkle:shortVersionString>\(version)</sparkle:shortVersionString>
            <sparkle:minimumSystemVersion>\(minimumSystemVersion)</sparkle:minimumSystemVersion>
            <enclosure url="https://github.com/sljzdotcom/AI-Token-Meter/releases/download/\(archivePath)"
                       length="\(length)" sparkle:edSignature="\(signature)"/>
          </item></channel>
        </rss>
        """

        #expect(try !validateStableAppcast(Data(fixture.utf8)).isEmpty, Comment(rawValue: label))
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

    private static let validFixtureSignature =
        "N3uMwUOzoejXn+oRZ2gSm0mmxvrbxGgWcgzrFHt2YBxj5G7Wccz+EX+tW0gpQSB5H33QeKbApiwKxZyjxih/Dg=="
}

private func validateStableAppcast(_ data: Data) throws -> [String] {
    let delegate = AppcastParserDelegate()
    let parser = XMLParser(data: data)
    parser.delegate = delegate
    guard parser.parse() else {
        throw parser.parserError ?? AppcastValidationError.invalidXML
    }

    guard !delegate.items.isEmpty else {
        return ["appcast contains no release items"]
    }

    return delegate.items.enumerated().flatMap { index, item in
        let prefix = "item \(index + 1)"
        var issues: [String] = []
        let versionParts = item.shortVersion.split(separator: ".", omittingEmptySubsequences: false)
        if versionParts.count < 2 || versionParts.contains(where: { Int($0) == nil }) {
            issues.append("\(prefix) has an invalid version")
        }
        if Int(item.build).map({ $0 > 0 }) != true {
            issues.append("\(prefix) has a non-positive build")
        }

        let expectedURL = "https://github.com/sljzdotcom/AI-Token-Meter/releases/download/v\(item.shortVersion)/AI-Token-Meter-\(item.shortVersion)-macOS-arm64.zip"
        if item.url != expectedURL {
            issues.append("\(prefix) URL does not match its version")
        }
        if Data(base64Encoded: item.signature)?.count != 64 {
            issues.append("\(prefix) has an invalid EdDSA signature encoding")
        }
        if Int64(item.length).map({ $0 > 0 }) != true {
            issues.append("\(prefix) has a non-positive archive length")
        }
        if item.minimumSystemVersion != "14.0" {
            issues.append("\(prefix) does not require macOS 14.0")
        }
        return issues
    }
}

private struct AppcastItem {
    var build = ""
    var shortVersion = ""
    var minimumSystemVersion = ""
    var url = ""
    var signature = ""
    var length = ""
}

private final class AppcastParserDelegate: NSObject, XMLParserDelegate {
    private(set) var items: [AppcastItem] = []
    private var item: AppcastItem?
    private var element = ""
    private var text = ""

    func parser(
        _ parser: XMLParser,
        didStartElement elementName: String,
        namespaceURI: String?,
        qualifiedName qName: String?,
        attributes attributeDict: [String: String] = [:]
    ) {
        element = qName ?? elementName
        text = ""
        if element == "item" {
            item = AppcastItem()
        } else if element == "enclosure", item != nil {
            item?.url = attributeDict["url"] ?? ""
            item?.signature = attributeDict["sparkle:edSignature"] ?? ""
            item?.length = attributeDict["length"] ?? ""
        }
    }

    func parser(_ parser: XMLParser, foundCharacters string: String) {
        text += string
    }

    func parser(
        _ parser: XMLParser,
        didEndElement elementName: String,
        namespaceURI: String?,
        qualifiedName qName: String?
    ) {
        let name = qName ?? elementName
        let value = text.trimmingCharacters(in: .whitespacesAndNewlines)
        switch name {
        case "sparkle:version": item?.build = value
        case "sparkle:shortVersionString": item?.shortVersion = value
        case "sparkle:minimumSystemVersion": item?.minimumSystemVersion = value
        case "item":
            if let item { items.append(item) }
            item = nil
        default: break
        }
        text = ""
    }
}

private enum AppcastValidationError: Error {
    case invalidXML
}
