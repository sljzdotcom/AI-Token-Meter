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
    }

    private static let infoPlistURL = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .appending(path: "Sources/AIMeterApp/Resources/Info.plist")
}
