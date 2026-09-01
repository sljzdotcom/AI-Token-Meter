import Foundation
import Testing
@testable import AIMeterApp
@testable import AIMeterCore

@Suite("CLI authentication launcher", .serialized)
@MainActor
struct CLIAuthenticationLauncherTests {
    @Test("Claude and Codex login scripts are private and opened")
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

        #expect(claudeURL.lastPathComponent == "Open Claude Login.command")
        #expect(codexURL.lastPathComponent == "Open Codex Login.command")
        #expect(opened == [claudeURL, codexURL])
        #expect(permissions(of: claudeURL) == 0o700)
        #expect(permissions(of: codexURL) == 0o700)
        #expect(try String(contentsOf: claudeURL, encoding: .utf8).contains("auth login"))
        #expect(try String(contentsOf: codexURL, encoding: .utf8).contains("codex' login"))
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
