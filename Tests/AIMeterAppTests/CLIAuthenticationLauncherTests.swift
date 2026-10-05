import Foundation
import Testing
@testable import AIMeterApp
@testable import AIMeterCore

@Suite("CLI authentication launcher", .serialized)
@MainActor
struct CLIAuthenticationLauncherTests {
    @Test("Claude, Codex, and Antigravity login scripts are private and opened")
    func writesPrivateScripts() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        var opened: [URL] = []
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root,
            executableLocator: AuthenticationFixedLocator(),
            openURL: { opened.append($0); return true }
        )

        let claudeURL = try launcher.open(provider: .claude)
        let codexURL = try launcher.open(provider: .codex)
        let antigravityURL = try launcher.open(
            provider: .gemini,
            completionToken: "12345678-1234-1234-1234-123456789abc"
        )

        #expect(claudeURL.lastPathComponent == "Open Claude Login.command")
        #expect(codexURL.lastPathComponent == "Open Codex Login.command")
        #expect(antigravityURL.lastPathComponent == "Open Antigravity Login.command")
        #expect(opened == [claudeURL, codexURL, antigravityURL])
        #expect(permissions(of: claudeURL) == 0o700)
        #expect(permissions(of: codexURL) == 0o700)
        #expect(permissions(of: antigravityURL) == 0o700)
        #expect(try String(contentsOf: claudeURL, encoding: .utf8).contains("auth login"))
        #expect(try String(contentsOf: codexURL, encoding: .utf8).contains("codex' login"))
        #expect(try String(contentsOf: antigravityURL, encoding: .utf8).contains("agy"))
        #expect(try String(contentsOf: antigravityURL, encoding: .utf8).contains("aitokenmeter://antigravity-login-complete?token=12345678-1234-1234-1234-123456789abc"))
    }

    @Test("Codex login script uses the nested ChatGPT app-bundled CLI")
    func nestedBundledCodexLoginScript() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let applications = root.appendingPathComponent("Applications", isDirectory: true)
        let home = root.appendingPathComponent("home", isDirectory: true)
        let executable = home.appendingPathComponent(
            "Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex"
        )
        try FileManager.default.createDirectory(
            at: executable.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try Data("#!/bin/sh\nexit 0\n".utf8).write(to: executable)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: executable.path)
        let locator = ExecutableLocator(
            searchPaths: [],
            bundledExecutablePaths: ExecutableLocator.bundledExecutablePaths(
                homeDirectory: home,
                systemApplications: applications
            )
        )
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root.appendingPathComponent("authentication"),
            executableLocator: locator,
            openURL: { _ in true }
        )

        let scriptURL = try launcher.open(provider: .codex)
        let script = try String(contentsOf: scriptURL, encoding: .utf8)

        #expect(script.contains("exec '\(executable.path)' login"))
    }

    @Test("A default test-process launch is refused before writing a login script")
    func defaultTestProcessLaunchIsFailClosed() {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let authenticationDirectory = root.appendingPathComponent("Authentication", isDirectory: true)
        let policy = SystemActionPolicy(
            environment: ["XCTestConfigurationFilePath": "/tmp/fake-tests.xctest"],
            isXCTestBundle: false
        )
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: authenticationDirectory,
            executableLocator: AuthenticationFixedLocator(),
            systemActionPolicy: policy
        )

        #expect(throws: CLIAuthenticationLaunchError.externalActionsDisabled) {
            try launcher.open(
                provider: .gemini,
                completionToken: "12345678-1234-1234-1234-123456789abc"
            )
        }
        #expect(!FileManager.default.fileExists(atPath: authenticationDirectory.path))
    }

    @Test("A missing executable produces a provider-specific failure")
    func missingCLI() {
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: temporaryDirectory(),
            executableLocator: AuthenticationMissingLocator(),
            openURL: { _ in true }
        )

        #expect(throws: CLIAuthenticationLaunchError.notInstalled(.claude)) {
            try launcher.open(provider: .claude)
        }
    }

    @Test("Terminal open failure remains visible to the caller")
    func openFailure() {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root,
            executableLocator: AuthenticationFixedLocator(),
            openURL: { _ in false }
        )

        #expect(throws: CLIAuthenticationLaunchError.couldNotOpenTerminal) {
            try launcher.open(provider: .codex)
        }
    }

    @Test("DeepSeek is rejected before any file is written")
    func rejectsDeepSeek() {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root,
            executableLocator: AuthenticationFixedLocator(),
            openURL: { _ in true }
        )

        #expect(throws: CLIAuthenticationLaunchError.unsupportedProvider) {
            try launcher.open(provider: .deepSeek)
        }
        #expect(!FileManager.default.fileExists(atPath: root.path))
    }

    private func temporaryDirectory() -> URL {
        FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-auth-\(UUID().uuidString)", isDirectory: true)
    }

    private func permissions(of url: URL) -> Int {
        let attributes = try? FileManager.default.attributesOfItem(atPath: url.path)
        return (attributes?[.posixPermissions] as? NSNumber)?.intValue ?? 0
    }
}

private struct AuthenticationFixedLocator: ExecutableLocating {
    func locate(named name: String) -> URL? {
        URL(fileURLWithPath: "/tmp/\(name.uppercased()) CLI/\(name)")
    }
}

private struct AuthenticationMissingLocator: ExecutableLocating {
    func locate(named name: String) -> URL? { nil }
}
