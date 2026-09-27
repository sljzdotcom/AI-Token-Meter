import Foundation
import Testing
@testable import AIMeterCore

@Suite("Executable locator")
struct ExecutableLocatorTests {
    @Test("An explicitly installed CLI wins over an app-bundled executable")
    func explicitCLIHasPriority() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let explicit = try makeExecutable(at: root.appendingPathComponent("bin/codex"))
        let bundled = try makeExecutable(at: root.appendingPathComponent("ChatGPT.app/Contents/Resources/codex"))
        let locator = ExecutableLocator(
            searchPaths: [explicit.deletingLastPathComponent().path],
            bundledExecutablePaths: ["codex": [bundled.path]]
        )

        #expect(locator.locate(named: "codex") == explicit)
    }

    @Test("An app-bundled Codex is found when a GUI launch has no useful PATH")
    func appBundledCodexFallback() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let bundled = try makeExecutable(at: root.appendingPathComponent("ChatGPT.app/Contents/Resources/codex"))
        let locator = ExecutableLocator(
            searchPaths: [],
            bundledExecutablePaths: ["codex": [bundled.path]]
        )

        #expect(locator.locate(named: "codex") == bundled)
    }

    @Test("ChatGPT's nested Codex CLI candidates follow all legacy bundle candidates")
    func nestedChatGPTCandidatesPreserveLegacyOrder() {
        let home = URL(fileURLWithPath: "/Users/tester", isDirectory: true)
        let systemApplications = URL(fileURLWithPath: "/System/Applications", isDirectory: true)
        let candidates = ExecutableLocator.bundledExecutablePaths(
            homeDirectory: home,
            systemApplications: systemApplications
        )["codex"] ?? []

        #expect(candidates == [
            "/Users/tester/Applications/ChatGPT.app/Contents/Resources/codex",
            "/Users/tester/Applications/Codex.app/Contents/Resources/codex",
            "/System/Applications/ChatGPT.app/Contents/Resources/codex",
            "/System/Applications/Codex.app/Contents/Resources/codex",
            "/Users/tester/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
            "/Users/tester/Applications/Codex.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
            "/System/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
            "/System/Applications/Codex.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
        ])
    }

    @Test("A nested ChatGPT CLI is discovered after invalid legacy candidates")
    func nestedChatGPTCLIIsFallbackAfterInvalidCandidates() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let home = root.appendingPathComponent("home", isDirectory: true)
        let systemApplications = root.appendingPathComponent("Applications", isDirectory: true)
        let oldCandidate = home.appendingPathComponent(
            "Applications/ChatGPT.app/Contents/Resources/codex"
        )
        try FileManager.default.createDirectory(
            at: oldCandidate.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try Data("not executable".utf8).write(to: oldCandidate)

        let nestedCLI = try makeExecutable(at: home.appendingPathComponent(
            "Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex"
        ))
        let candidates = ExecutableLocator.bundledExecutablePaths(
            homeDirectory: home,
            systemApplications: systemApplications
        )
        let locator = ExecutableLocator(searchPaths: [], bundledExecutablePaths: candidates)

        #expect(locator.locate(named: "codex") == nestedCLI)
    }

    @Test("A system Applications ChatGPT bundle is used when the user bundle is absent")
    func nestedSystemChatGPTCLIIsDiscovered() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let home = root.appendingPathComponent("home", isDirectory: true)
        let systemApplications = root.appendingPathComponent("Applications", isDirectory: true)
        let nestedCLI = try makeExecutable(at: systemApplications.appendingPathComponent(
            "ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex"
        ))
        let candidates = ExecutableLocator.bundledExecutablePaths(
            homeDirectory: home,
            systemApplications: systemApplications
        )
        let locator = ExecutableLocator(searchPaths: [], bundledExecutablePaths: candidates)

        #expect(locator.locate(named: "codex") == nestedCLI)
    }

    @Test("A broken legacy app symlink falls through to the nested CLI")
    func brokenLegacySymlinkFallsThroughToNestedCLI() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let home = root.appendingPathComponent("home", isDirectory: true)
        let systemApplications = root.appendingPathComponent("Applications", isDirectory: true)
        let legacyCLI = home.appendingPathComponent(
            "Applications/ChatGPT.app/Contents/Resources/codex"
        )
        try FileManager.default.createDirectory(
            at: legacyCLI.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try FileManager.default.createSymbolicLink(
            at: legacyCLI,
            withDestinationURL: root.appendingPathComponent("missing-codex")
        )
        let nestedCLI = try makeExecutable(at: home.appendingPathComponent(
            "Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex"
        ))
        let candidates = ExecutableLocator.bundledExecutablePaths(
            homeDirectory: home,
            systemApplications: systemApplications
        )
        let locator = ExecutableLocator(searchPaths: [], bundledExecutablePaths: candidates)

        #expect(locator.locate(named: "codex") == nestedCLI)
    }

    @Test("A non-executable app resource is not mistaken for Codex CLI")
    func rejectsNonExecutableBundleResource() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let bundled = root.appendingPathComponent("ChatGPT.app/Contents/Resources/codex")
        try FileManager.default.createDirectory(
            at: bundled.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try Data("placeholder".utf8).write(to: bundled)
        let locator = ExecutableLocator(
            searchPaths: [],
            bundledExecutablePaths: ["codex": [bundled.path]]
        )

        #expect(locator.locate(named: "codex") == nil)
    }

    @Test("nvm Node version bins are discovered newest first")
    func discoversNVMInstallations() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        _ = try makeExecutable(
            at: root.appendingPathComponent(".nvm/versions/node/v9.8.0/bin/codex")
        )
        _ = try makeExecutable(
            at: root.appendingPathComponent(".nvm/versions/node/v25.2.0/bin/codex")
        )

        let paths = ExecutableLocator.nvmSearchPaths(homeDirectory: root)

        #expect(paths.first?.hasSuffix("/.nvm/versions/node/v25.2.0/bin") == true)
        #expect(paths.contains { $0.hasSuffix("/.nvm/versions/node/v9.8.0/bin") })
    }

    private func temporaryDirectory() -> URL {
        FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-executable-locator-\(UUID().uuidString)", isDirectory: true)
    }

    private func makeExecutable(at url: URL) throws -> URL {
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try Data("#!/bin/sh\nexit 0\n".utf8).write(to: url)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: url.path)
        return url
    }
}
