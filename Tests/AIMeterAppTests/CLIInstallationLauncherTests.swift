import Foundation
import Testing
import AIMeterCore
@testable import AIMeterApp

@Suite("CLI installation launcher", .serialized)
@MainActor
struct CLIInstallationLauncherTests {
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
