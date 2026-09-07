import Foundation
import Testing
@testable import AIMeterCore
@testable import AIMeterApp

@Suite("CLI installation launcher", .serialized)
@MainActor
struct CLIInstallationLauncherTests {
    @Test("Nonexecutable and broken-link CLI candidates cannot authorize installation", arguments: [false, true])
    func invalidExistingCLI(brokenLink: Bool) async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        for name in ["claude", "codex"] {
            let url = root.appendingPathComponent(name)
            if brokenLink { try FileManager.default.createSymbolicLink(at: url, withDestinationURL: root.appendingPathComponent("missing-target")) }
            else {
                try Data("#!/bin/sh\n".utf8).write(to: url)
                try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: url.path)
            }
        }
        let locator = ExecutableLocator(searchPaths: [root.path], bundledExecutablePaths: [:])
        let output = root.appendingPathComponent("installer")
        let launcher = CLIInstallationLauncher(directory: output, locator: locator, openURL: { _ in Issue.record("Must not open installer over invalid CLI"); return true })
        for provider in [UsageProvider.claude, .codex] {
            var blocked = false
            do { _ = try launcher.open(provider: provider) } catch { blocked = true }
            #expect(blocked)
        }
        #expect(!FileManager.default.fileExists(atPath: output.path))
        #expect(await ClaudeAccountReader(locator: locator).read().connectionState == .unavailable)
        #expect(await CodexAccountReader(locator: locator).read().connectionState == .unavailable)
    }
    @Test("Discovery protects an existing CLI without creating or opening installer files")
    func existingCLI() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let launcher = CLIInstallationLauncher(directory: root, locator: InstallLocator(found: true), openURL: { _ in Issue.record("Must not open an installer"); return true })
        #expect(try !launcher.open(provider: .codex))
        #expect(!FileManager.default.fileExists(atPath: root.path))
    }

    @Test("Failed terminal opening is reported and written script is private")
    func terminalFailure() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: root) }
        var script: URL?
        let launcher = CLIInstallationLauncher(directory: root, locator: InstallLocator(found: false), openURL: { script = $0; return false })
        #expect(throws: CLIAuthenticationLaunchError.couldNotOpenTerminal) { try launcher.open(provider: .claude) }
        let path = try #require(script).path
        let permissions = try FileManager.default.attributesOfItem(atPath: path)[.posixPermissions] as? NSNumber
        #expect(permissions?.intValue == 0o700)
    }
}

private struct InstallLocator: ExecutableLocating {
    let found: Bool
    func locate(named name: String) -> URL? { found ? URL(fileURLWithPath: "/fixture/\(name)") : nil }
}
