import Foundation
import Testing

@Suite("AI Token Meter bundle metadata")
struct AppBundleMetadataTests {
    @Test("Declares the visible brand without changing compatibility identity")
    func bundleMetadata() throws {
        let data = try Data(contentsOf: Self.infoPlistURL)
        let plist = try #require(
            PropertyListSerialization.propertyList(from: data, format: nil)
                as? [String: Any]
        )

        #expect(plist["CFBundleDisplayName"] as? String == "AI Token Meter")
        #expect(plist["CFBundleName"] as? String == "AI Token Meter")
        #expect(plist["CFBundleIdentifier"] as? String == "com.millerpan.AIMeter")
        #expect(plist["CFBundleExecutable"] as? String == "AIMeterApp")
        #expect(plist["CFBundleIconFile"] as? String == "AppIcon")
        #expect(plist["CFBundleShortVersionString"] as? String == "0.2.1")
        #expect(plist["CFBundleVersion"] as? String == "5")

        let URLTypes = try #require(plist["CFBundleURLTypes"] as? [[String: Any]])
        let URLType = try #require(URLTypes.first)
        #expect(URLType["CFBundleURLName"] as? String == "com.millerpan.AIMeter")
        #expect(URLType["CFBundleURLSchemes"] as? [String] == ["aitokenmeter"])
    }

    private static let infoPlistURL = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .appending(path: "Sources/AIMeterApp/Resources/Info.plist")
}
