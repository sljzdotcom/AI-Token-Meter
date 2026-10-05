import AIMeterCore
import AppKit
import Foundation

enum CLIAuthenticationLaunchError: Error, Equatable {
    case unsupportedProvider
    case notInstalled(UsageProvider)
    case couldNotOpenTerminal
    case externalActionsDisabled
}

@MainActor
final class CLIAuthenticationLauncher {
    private let authenticationDirectoryURL: URL
    private let executableLocator: any ExecutableLocating
    private let scriptBuilder: CLIAuthenticationScriptBuilder
    private let openURL: (URL) -> Bool
    private let systemActionPolicy: SystemActionPolicy
    private let usesSystemOpener: Bool

    init(
        authenticationDirectoryURL: URL? = nil,
        executableLocator: any ExecutableLocating = ExecutableLocator(),
        scriptBuilder: CLIAuthenticationScriptBuilder = CLIAuthenticationScriptBuilder(),
        openURL: ((URL) -> Bool)? = nil,
        systemActionPolicy: SystemActionPolicy = .current
    ) {
        let applicationSupportDirectory = FileManager.default.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        )[0]
        self.authenticationDirectoryURL = authenticationDirectoryURL ?? systemActionPolicy.scriptDirectory(
            name: "Authentication",
            applicationSupportDirectory: applicationSupportDirectory,
            temporaryDirectory: FileManager.default.temporaryDirectory
        )
        self.executableLocator = executableLocator
        self.scriptBuilder = scriptBuilder
        self.systemActionPolicy = systemActionPolicy
        self.usesSystemOpener = openURL == nil
        self.openURL = openURL ?? { NSWorkspace.shared.open($0) }
    }

    @discardableResult
    func open(provider: UsageProvider, completionToken: String? = nil) throws -> URL {
        guard !usesSystemOpener || systemActionPolicy.allowsExternalOpen else {
            throw CLIAuthenticationLaunchError.externalActionsDisabled
        }
        let executableName: String
        let scriptName: String
        switch provider {
        case .claude:
            executableName = "claude"
            scriptName = "Open Claude Login.command"
        case .codex:
            executableName = "codex"
            scriptName = "Open Codex Login.command"
        case .gemini:
            executableName = "agy"
            scriptName = "Open Antigravity Login.command"
        case .deepSeek:
            throw CLIAuthenticationLaunchError.unsupportedProvider
        }

        guard let executableURL = executableLocator.locate(named: executableName) else {
            throw CLIAuthenticationLaunchError.notInstalled(provider)
        }
        let script = try scriptBuilder.build(
            provider: provider,
            executableURL: executableURL,
            completionToken: completionToken
        )
        try FileManager.default.createDirectory(
            at: authenticationDirectoryURL,
            withIntermediateDirectories: true
        )
        let scriptURL = authenticationDirectoryURL.appendingPathComponent(scriptName)
        try Data(script.utf8).write(to: scriptURL, options: .atomic)
        try FileManager.default.setAttributes(
            [.posixPermissions: 0o700],
            ofItemAtPath: scriptURL.path
        )
        guard openURL(scriptURL) else {
            throw CLIAuthenticationLaunchError.couldNotOpenTerminal
        }
        return scriptURL
    }
}
