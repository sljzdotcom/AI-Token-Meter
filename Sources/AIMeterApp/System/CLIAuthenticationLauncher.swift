import AIMeterCore
import AppKit
import Foundation

enum CLIAuthenticationLaunchError: Error, Equatable {
    case unsupportedProvider
    case notInstalled(UsageProvider)
    case couldNotOpenTerminal
}

@MainActor
final class CLIAuthenticationLauncher {
    private let authenticationDirectoryURL: URL
    private let executableLocator: any ExecutableLocating
    private let scriptBuilder: CLIAuthenticationScriptBuilder
    private let openURL: (URL) -> Bool

    init(
        authenticationDirectoryURL: URL? = nil,
        executableLocator: any ExecutableLocating = ExecutableLocator(),
        scriptBuilder: CLIAuthenticationScriptBuilder = CLIAuthenticationScriptBuilder(),
        openURL: @escaping (URL) -> Bool = { NSWorkspace.shared.open($0) }
    ) {
        self.authenticationDirectoryURL = authenticationDirectoryURL
            ?? FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
                .appendingPathComponent("AI Meter/Authentication", isDirectory: true)
        self.executableLocator = executableLocator
        self.scriptBuilder = scriptBuilder
        self.openURL = openURL
    }

    @discardableResult
    func open(provider: UsageProvider) throws -> URL {
        let executableName: String
        let scriptName: String
        switch provider {
        case .claude:
            executableName = "claude"
            scriptName = "Open Claude Login.command"
        case .codex:
            executableName = "codex"
            scriptName = "Open Codex Login.command"
        case .deepSeek:
            throw CLIAuthenticationLaunchError.unsupportedProvider
        }

        guard let executableURL = executableLocator.locate(named: executableName) else {
            throw CLIAuthenticationLaunchError.notInstalled(provider)
        }
        let script = try scriptBuilder.build(provider: provider, executableURL: executableURL)
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
