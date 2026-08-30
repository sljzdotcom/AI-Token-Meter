import AIMeterCore
import AppKit
import Foundation

@MainActor
final class ClaudeWorkspaceSetupLauncher {
    private let workspaceResolver: any ClaudeUsageWorkspaceResolving
    private let executableLocator: any ExecutableLocating
    private let scriptBuilder: ClaudeSetupScriptBuilder

    init(
        workspaceResolver: any ClaudeUsageWorkspaceResolving = ClaudeUsageWorkspaceResolver(),
        executableLocator: any ExecutableLocating = ExecutableLocator(),
        scriptBuilder: ClaudeSetupScriptBuilder = ClaudeSetupScriptBuilder()
    ) {
        self.workspaceResolver = workspaceResolver
        self.executableLocator = executableLocator
        self.scriptBuilder = scriptBuilder
    }

    func open() throws {
        guard let executableURL = executableLocator.locate(named: "claude") else {
            throw ClaudeWorkspaceSetupError.claudeNotInstalled
        }
        let workspaceURL = try workspaceResolver.resolve()
        let scriptURL = workspaceURL
            .deletingLastPathComponent()
            .appendingPathComponent("Open Claude Usage Setup.command")
        let script = scriptBuilder.build(
            workspaceURL: workspaceURL,
            executableURL: executableURL
        )
        try Data(script.utf8).write(to: scriptURL, options: .atomic)
        try FileManager.default.setAttributes(
            [.posixPermissions: 0o700],
            ofItemAtPath: scriptURL.path
        )
        guard NSWorkspace.shared.open(scriptURL) else {
            throw ClaudeWorkspaceSetupError.couldNotOpenTerminal
        }
    }
}

private enum ClaudeWorkspaceSetupError: Error {
    case claudeNotInstalled
    case couldNotOpenTerminal
}
