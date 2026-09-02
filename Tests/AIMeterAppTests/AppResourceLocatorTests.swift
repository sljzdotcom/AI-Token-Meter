import Foundation
import Testing
@testable import AIMeterApp

@Suite("Portable app resource lookup")
struct AppResourceLocatorTests {
    @Test("A packaged app resolves a resource from its own Resources directory")
    func packagedResourceLookup() throws {
        let fixture = try makeAppBundle(resourceName: "portable", resourceContents: "fixture")
        defer { try? FileManager.default.removeItem(at: fixture.root) }
        var fallbackCalls = 0

        let url = AppResourceLocator.url(
            forResource: "portable",
            withExtension: "txt",
            subdirectory: "Logos",
            primaryBundle: fixture.bundle,
            packageBundle: {
                fallbackCalls += 1
                return nil
            }
        )

        #expect(url?.lastPathComponent == "portable.txt")
        #expect(fallbackCalls == 0)
    }

    @Test("A packaged app never falls back to SwiftPM's build-machine bundle")
    func packagedMissingResourceDoesNotUseBuildFallback() throws {
        let fixture = try makeAppBundle(resourceName: nil, resourceContents: nil)
        defer { try? FileManager.default.removeItem(at: fixture.root) }
        var fallbackCalls = 0

        let url = AppResourceLocator.url(
            forResource: "missing",
            withExtension: "txt",
            subdirectory: "Logos",
            primaryBundle: fixture.bundle,
            packageBundle: {
                fallbackCalls += 1
                return nil
            }
        )

        #expect(url == nil)
        #expect(fallbackCalls == 0)
    }

    private func makeAppBundle(
        resourceName: String?,
        resourceContents: String?
    ) throws -> (root: URL, bundle: Bundle) {
        let root = FileManager.default.temporaryDirectory
            .appending(path: "AIMeter-resource-fixture-\(UUID().uuidString)")
        let appURL = root.appending(path: "Fixture.app")
        let contentsURL = appURL.appending(path: "Contents")
        let resourcesURL = contentsURL.appending(path: "Resources/Logos")
        try FileManager.default.createDirectory(at: resourcesURL, withIntermediateDirectories: true)

        let plist: [String: Any] = [
            "CFBundleIdentifier": "com.millerpan.AIMeter.ResourceFixture",
            "CFBundleName": "Fixture",
            "CFBundlePackageType": "APPL",
            "CFBundleVersion": "1",
        ]
        let plistData = try PropertyListSerialization.data(
            fromPropertyList: plist,
            format: .xml,
            options: 0
        )
        try plistData.write(to: contentsURL.appending(path: "Info.plist"))

        if let resourceName, let resourceContents {
            try Data(resourceContents.utf8).write(
                to: resourcesURL.appending(path: "\(resourceName).txt")
            )
        }

        return (root, try #require(Bundle(url: appURL)))
    }
}
