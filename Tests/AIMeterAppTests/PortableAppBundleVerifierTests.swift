import Foundation
import Testing

@Suite("Portable app bundle verifier", .serialized)
struct PortableAppBundleVerifierTests {
    @Test("Accepts resources directly embedded in the main app bundle")
    func acceptsPortableResourceLayout() throws {
        let fixture = try makeFixture(layout: .portable)
        defer { try? FileManager.default.removeItem(at: fixture) }

        let result = try runVerifier(on: fixture)

        #expect(result.status == 0)
        #expect(result.output.contains("Portable app resources verified"))
    }

    @Test("Rejects the old nested SwiftPM bundle layout")
    func rejectsNestedSwiftPMLayout() throws {
        let fixture = try makeFixture(layout: .nestedSwiftPMBundle)
        defer { try? FileManager.default.removeItem(at: fixture) }

        let result = try runVerifier(on: fixture)

        #expect(result.status != 0)
        #expect(result.output.contains("missing portable app resource"))
    }

    private enum Layout {
        case portable
        case nestedSwiftPMBundle
    }

    private func makeFixture(layout: Layout) throws -> URL {
        let root = FileManager.default.temporaryDirectory
            .appending(path: "AIMeter-portable-app-fixture-\(UUID().uuidString)")
        let app = root.appending(path: "AI Token Meter.app")
        let contents = app.appending(path: "Contents")
        let resources = contents.appending(path: "Resources")
        try FileManager.default.createDirectory(at: resources, withIntermediateDirectories: true)

        let plist: [String: Any] = [
            "CFBundleIdentifier": "com.millerpan.AIMeter",
            "CFBundleName": "AI Token Meter",
            "CFBundlePackageType": "APPL",
            "CFBundleVersion": "1",
        ]
        let plistData = try PropertyListSerialization.data(
            fromPropertyList: plist,
            format: .xml,
            options: 0
        )
        try plistData.write(to: contents.appending(path: "Info.plist"))

        let resourceRoot: URL
        switch layout {
        case .portable:
            resourceRoot = resources
        case .nestedSwiftPMBundle:
            resourceRoot = resources.appending(path: "AI-Meter_AIMeterApp.bundle")
        }

        for path in [
            "Logos/claude.png",
            "Logos/codex.svg",
            "Logos/deepseek.svg",
            "Backgrounds/floating-strip-deep-sea.png",
        ] {
            let url = resourceRoot.appending(path: path)
            try FileManager.default.createDirectory(
                at: url.deletingLastPathComponent(),
                withIntermediateDirectories: true
            )
            try Data("fixture".utf8).write(to: url)
        }

        return app
    }

    private func runVerifier(on app: URL) throws -> (status: Int32, output: String) {
        let process = Process()
        let output = Pipe()
        process.executableURL = repositoryRoot.appending(path: "scripts/verify-app-resources.sh")
        process.arguments = [app.path]
        process.standardOutput = output
        process.standardError = output
        try process.run()
        process.waitUntilExit()
        let data = output.fileHandleForReading.readDataToEndOfFile()
        return (process.terminationStatus, String(decoding: data, as: UTF8.self))
    }

    private var repositoryRoot: URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
    }
}
