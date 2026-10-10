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
    private let completionOpenExecutableURL: URL
    private let watchdogExecutableURL: URL
    private let geminiStopAcknowledgementTimeout: Duration
    private let systemActionPolicy: SystemActionPolicy
    private let usesSystemOpener: Bool
    private var pendingGeminiTokenWrites: [String: (pipeURL: URL, statusURL: URL, task: Task<Void, Never>)] = [:]
    private let geminiTerminalFailureStatuses: Set<String> = [
        "missing_pipe", "pipe_open_failed", "handoff_timeout", "invalid_handoff",
        "callback_failed", "watchdog_start_failed"
    ]

    init(
        authenticationDirectoryURL: URL? = nil,
        executableLocator: any ExecutableLocating = ExecutableLocator(),
        scriptBuilder: CLIAuthenticationScriptBuilder = CLIAuthenticationScriptBuilder(),
        openURL: ((URL) -> Bool)? = nil,
        completionOpenExecutableURL: URL? = nil,
        watchdogExecutableURL: URL = URL(fileURLWithPath: "/usr/bin/perl"),
        geminiStopAcknowledgementTimeout: Duration = .seconds(60),
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
        self.completionOpenExecutableURL = completionOpenExecutableURL
            ?? URL(fileURLWithPath: systemActionPolicy.allowsExternalOpen ? "/usr/bin/open" : "/usr/bin/false")
        self.watchdogExecutableURL = watchdogExecutableURL
        self.geminiStopAcknowledgementTimeout = geminiStopAcknowledgementTimeout
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
        let completionStatusFileURL: URL?
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
            let statusFile = authenticationDirectoryURL
                .appendingPathComponent("antigravity-\(UUID().uuidString).status")
            let launchGateFile = statusFile.appendingPathExtension("lock")
            let created = tokenPipe.path.withCString {
                Darwin.mkfifo($0, mode_t(S_IRUSR | S_IWUSR))
            }
            guard created == 0 else { throw CLIAuthenticationLaunchError.couldNotOpenTerminal }
            do {
                try Data("ready\n".utf8).write(to: statusFile, options: .withoutOverwriting)
                try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: statusFile.path)
                try Data().write(to: launchGateFile, options: .withoutOverwriting)
                try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: launchGateFile.path)
            } catch {
                try? FileManager.default.removeItem(at: tokenPipe)
                try? FileManager.default.removeItem(at: statusFile)
                try? FileManager.default.removeItem(at: launchGateFile)
                throw CLIAuthenticationLaunchError.couldNotOpenTerminal
            }
            completionTokenPipeURL = tokenPipe
            completionStatusFileURL = statusFile
        } else {
            completionTokenPipeURL = nil
            completionStatusFileURL = nil
        }
        let script: String
        do {
            script = try scriptBuilder.build(
                provider: provider,
                executableURL: executableURL,
                completionTokenPipeURL: completionTokenPipeURL,
                completionStatusFileURL: completionStatusFileURL,
                completionOpenExecutableURL: completionOpenExecutableURL,
                watchdogExecutableURL: watchdogExecutableURL
            )
        } catch {
            if let completionTokenPipeURL {
                try? FileManager.default.removeItem(at: completionTokenPipeURL)
            }
            if let completionStatusFileURL {
                try? FileManager.default.removeItem(at: completionStatusFileURL)
                try? FileManager.default.removeItem(at: completionStatusFileURL.appendingPathExtension("lock"))
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
            if let completionStatusFileURL {
                try? FileManager.default.removeItem(at: completionStatusFileURL)
                try? FileManager.default.removeItem(at: completionStatusFileURL.appendingPathExtension("lock"))
            }
            throw error
        }
        if let completionToken, let completionTokenPipeURL, let completionStatusFileURL {
            startGeminiTokenWrite(completionToken, to: completionTokenPipeURL, statusURL: completionStatusFileURL)
        }
        return scriptURL
    }

    func cancelPendingGeminiLogin(token: String) {
        guard let pending = pendingGeminiTokenWrites.removeValue(forKey: token) else { return }
        pending.task.cancel()
        try? FileManager.default.removeItem(at: pending.pipeURL)
        try? FileManager.default.removeItem(at: pending.statusURL)
        try? FileManager.default.removeItem(at: pending.statusURL.appendingPathExtension("lock"))
    }

    func requestGeminiLoginStop(token: String) {
        guard let pending = pendingGeminiTokenWrites.removeValue(forKey: token) else { return }
        pending.task.cancel()
        try? FileManager.default.removeItem(at: pending.pipeURL)
        let gatePath = pending.statusURL.appendingPathExtension("lock").path
        var gateDescriptor = gatePath.withCString { Darwin.open($0, O_RDWR) }
        while gateDescriptor < 0 {
            Thread.sleep(forTimeInterval: 0.01)
            gateDescriptor = gatePath.withCString { Darwin.open($0, O_RDWR) }
        }
        while Darwin.lockf(gateDescriptor, F_LOCK, 0) != 0 {
            if errno != EINTR { Thread.sleep(forTimeInterval: 0.01) }
        }
        try? Data("cancelled\n".utf8).write(to: pending.statusURL, options: .atomic)
        _ = Darwin.lockf(gateDescriptor, F_ULOCK, 0)
        Darwin.close(gateDescriptor)
        let statusPath = pending.statusURL.path
        let gateURL = pending.statusURL.appendingPathExtension("lock")
        let acknowledgementTimeout = geminiStopAcknowledgementTimeout
        Task.detached(priority: .utility) {
            let deadline = ContinuousClock.now.advanced(by: acknowledgementTimeout)
            while ContinuousClock.now < deadline {
                if let status = try? String(contentsOfFile: statusPath, encoding: .utf8),
                   status.trimmingCharacters(in: .whitespacesAndNewlines) == "helper_cancelled" {
                    Darwin.unlink(statusPath)
                    Darwin.unlink(gateURL.path)
                    return
                }
                try? await Task.sleep(for: .milliseconds(100))
            }
            // A delayed helper requires the status marker and launch gate before
            // exec, so an unacknowledged marker need not accumulate forever.
            Darwin.unlink(statusPath)
            Darwin.unlink(gateURL.path)
        }
    }

    func hasGeminiTerminalFailure(token: String) -> Bool {
        guard let pending = pendingGeminiTokenWrites[token],
              let status = try? String(contentsOf: pending.statusURL, encoding: .utf8),
              geminiTerminalFailureStatuses.contains(status.trimmingCharacters(in: .whitespacesAndNewlines)) else { return false }
        return true
    }

    private func startGeminiTokenWrite(_ token: String, to pipeURL: URL, statusURL: URL) {
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
        pendingGeminiTokenWrites[token] = (pipeURL, statusURL, task)
    }
}
