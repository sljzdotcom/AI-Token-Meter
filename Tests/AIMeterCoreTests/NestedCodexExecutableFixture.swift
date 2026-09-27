import Foundation
@testable import AIMeterCore

struct NestedCodexExecutableFixture {
    let root: URL
    let homeDirectory: URL
    let systemApplications: URL
    let executableURL: URL
    let locator: ExecutableLocator

    init(copying source: URL) throws {
        root = FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-nested-codex-\(UUID().uuidString)", isDirectory: true)
        homeDirectory = root.appendingPathComponent("home", isDirectory: true)
        systemApplications = root.appendingPathComponent("Applications", isDirectory: true)
        executableURL = homeDirectory.appendingPathComponent(
            "Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex"
        )
        try FileManager.default.createDirectory(
            at: executableURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try FileManager.default.copyItem(at: source, to: executableURL)
        try FileManager.default.setAttributes(
            [.posixPermissions: 0o755],
            ofItemAtPath: executableURL.path
        )
        locator = ExecutableLocator(
            searchPaths: [],
            bundledExecutablePaths: ExecutableLocator.bundledExecutablePaths(
                homeDirectory: homeDirectory,
                systemApplications: systemApplications
            )
        )
    }

    func remove() {
        try? FileManager.default.removeItem(at: root)
    }
}
