import AIMeterCore
import AppKit
import Foundation

@MainActor
final class ClaudeWorkspaceSetupLauncher {
    private let workspaceResolver: any ClaudeUsageWorkspaceResolving
    private let executableLocator: any ExecutableLocating
    private let scriptBuilder: ClaudeSetupScriptBuilder
    private let openURL: ((URL) -> Bool)?
    private let systemActionPolicy: SystemActionPolicy

    init(
        workspaceResolver: any ClaudeUsageWorkspaceResolving = ClaudeUsageWorkspaceResolver(),
        executableLocator: any ExecutableLocating = ExecutableLocator(),
        scriptBuilder: ClaudeSetupScriptBuilder = ClaudeSetupScriptBuilder(),
        openURL: ((URL) -> Bool)? = nil,
        systemActionPolicy: SystemActionPolicy = .current
    ) {
        self.workspaceResolver = workspaceResolver
        self.executableLocator = executableLocator
        self.scriptBuilder = scriptBuilder
        self.openURL = openURL
        self.systemActionPolicy = systemActionPolicy
    }

    func open() throws {
        guard openURL != nil || systemActionPolicy.allowsExternalOpen else {
            throw ClaudeWorkspaceSetupError.externalActionsDisabled
        }
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
        let didOpen = if let openURL {
            openURL(scriptURL)
        } else {
            systemActionPolicy.open(scriptURL) { NSWorkspace.shared.open($0) }
        }
        guard didOpen else {
            throw ClaudeWorkspaceSetupError.couldNotOpenTerminal
        }
    }
}

private enum ClaudeWorkspaceSetupError: Error {
    case claudeNotInstalled
    case couldNotOpenTerminal
    case externalActionsDisabled
}
