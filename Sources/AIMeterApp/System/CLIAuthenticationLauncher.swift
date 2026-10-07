import AIMeterCore
import AppKit
import Darwin
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
    private var pendingGeminiTokenWrites: [String: (pipeURL: URL, task: Task<Void, Never>)] = [:]

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
        let completionTokenPipeURL: URL?
        if provider == .gemini {
            guard let completionToken, UUID(uuidString: completionToken) != nil else {
                throw CLIAuthenticationLaunchError.unsupportedProvider
            }
            try FileManager.default.createDirectory(
                at: authenticationDirectoryURL,
                withIntermediateDirectories: true,
                attributes: [.posixPermissions: 0o700]
            )
            try FileManager.default.setAttributes(
                [.posixPermissions: 0o700],
                ofItemAtPath: authenticationDirectoryURL.path
            )
            let tokenPipe = authenticationDirectoryURL
                .appendingPathComponent("antigravity-\(UUID().uuidString).token")
            let created = tokenPipe.path.withCString {
                Darwin.mkfifo($0, mode_t(S_IRUSR | S_IWUSR))
            }
            guard created == 0 else { throw CLIAuthenticationLaunchError.couldNotOpenTerminal }
            completionTokenPipeURL = tokenPipe
        } else {
            completionTokenPipeURL = nil
        }
        let script: String
        do {
            script = try scriptBuilder.build(
                provider: provider,
                executableURL: executableURL,
                completionTokenPipeURL: completionTokenPipeURL
            )
        } catch {
            if let completionTokenPipeURL {
                try? FileManager.default.removeItem(at: completionTokenPipeURL)
            }
            throw error
        }
        let scriptURL = authenticationDirectoryURL.appendingPathComponent(scriptName)
        do {
            try FileManager.default.createDirectory(
                at: authenticationDirectoryURL,
                withIntermediateDirectories: true,
                attributes: [.posixPermissions: 0o700]
            )
            try Data(script.utf8).write(to: scriptURL, options: .atomic)
            try FileManager.default.setAttributes(
                [.posixPermissions: 0o700],
                ofItemAtPath: scriptURL.path
            )
            guard openURL(scriptURL) else {
                throw CLIAuthenticationLaunchError.couldNotOpenTerminal
            }
        } catch {
            try? FileManager.default.removeItem(at: scriptURL)
            if let completionTokenPipeURL {
                try? FileManager.default.removeItem(at: completionTokenPipeURL)
            }
            throw error
        }
        if let completionToken, let completionTokenPipeURL {
            startGeminiTokenWrite(completionToken, to: completionTokenPipeURL)
        }
        return scriptURL
    }

    func cancelPendingGeminiLogin(token: String) {
        guard let pending = pendingGeminiTokenWrites.removeValue(forKey: token) else { return }
        pending.task.cancel()
        try? FileManager.default.removeItem(at: pending.pipeURL)
    }

    private func startGeminiTokenWrite(_ token: String, to pipeURL: URL) {
        let path = pipeURL.path
        let task = Task.detached(priority: .userInitiated) {
            let deadline = ContinuousClock.now.advanced(by: .seconds(300))
            let data = Data("\(token)\n".utf8)
            while !Task.isCancelled, ContinuousClock.now < deadline {
                let descriptor = path.withCString { Darwin.open($0, O_WRONLY | O_NONBLOCK) }
                if descriptor >= 0 {
                    let written = data.withUnsafeBytes { bytes in
                        Darwin.write(descriptor, bytes.baseAddress, data.count)
                    }
                    Darwin.close(descriptor)
                    if written != data.count { Darwin.unlink(path) }
                    return
                }
                do {
                    try await Task.sleep(for: .milliseconds(100))
                } catch {
                    return
                }
            }
            Darwin.unlink(path)
        }
        pendingGeminiTokenWrites[token] = (pipeURL, task)
    }
}
