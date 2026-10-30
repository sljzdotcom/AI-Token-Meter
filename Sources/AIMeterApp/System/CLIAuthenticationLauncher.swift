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
    private let geminiStopGateLockTimeout: Duration
    private let geminiStopAcknowledgementTimeout: Duration
    private let systemActionPolicy: SystemActionPolicy
    private let usesSystemOpener: Bool
    private var pendingGeminiTokenWrites: [String: (pipeURL: URL, statusURL: URL, execStatusURL: URL, cancelRequestURL: URL, scriptURL: URL, task: Task<Void, Never>)] = [:]
    private var pendingGeminiStops: [String: (id: UUID, statusURL: URL, task: Task<Bool, Never>)] = [:]
    private let geminiTerminalFailureStatuses: Set<String> = [
        "missing_pipe", "pipe_open_failed", "handoff_timeout", "invalid_handoff",
        "callback_failed", "watchdog_start_failed", "terminal_foreground_failed",
        "terminal_foreground_restore_failed", "guardian_identity_mismatch",
        "guardian_identity_unavailable", "helper_stop_unconfirmed", "process_state_unavailable",
        "exec_failed", "launch_indeterminate"
    ]
    // These failures can finish without a callback. Their status alone never
    // proves cleanup; the helper must also publish its per-attempt final ACK.
    private let geminiCompletedFailureStatuses: Set<String> = [
        "missing_pipe", "pipe_open_failed", "handoff_timeout", "invalid_handoff",
        "callback_failed", "watchdog_start_failed", "terminal_foreground_failed"
    ]
    private let geminiCleanupUncertainStatuses: Set<String> = [
        "guardian_identity_mismatch", "guardian_identity_unavailable", "helper_stop_unconfirmed",
        "process_state_unavailable", "launch_indeterminate"
    ]

    init(
        authenticationDirectoryURL: URL? = nil,
        executableLocator: any ExecutableLocating = ExecutableLocator(),
        scriptBuilder: CLIAuthenticationScriptBuilder = CLIAuthenticationScriptBuilder(),
        openURL: ((URL) -> Bool)? = nil,
        completionOpenExecutableURL: URL? = nil,
        watchdogExecutableURL: URL = URL(fileURLWithPath: "/usr/bin/perl"),
        geminiStopGateLockTimeout: Duration = .seconds(2),
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
        self.geminiStopGateLockTimeout = geminiStopGateLockTimeout
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
            // LaunchServices may consume an earlier open request after a retry.
            // Give each attempt its own immutable script path so an old request
            // can never observe a later attempt's coordinator paths.
            scriptName = "Open Antigravity Login \(UUID().uuidString).command"
        case .deepSeek, .code0:
            throw CLIAuthenticationLaunchError.unsupportedProvider
        }

        guard let executableURL = executableLocator.locate(named: executableName) else {
            throw CLIAuthenticationLaunchError.notInstalled(provider)
        }
        let completionTokenPipeURL: URL?
        let completionStatusFileURL: URL?
        let completionExecStatusFileURL: URL?
        let cancellationRequestFileURL: URL?
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
            let execStatusFile = statusFile.appendingPathExtension("exec")
            let launchGateFile = statusFile.appendingPathExtension("lock")
            let cancelRequestFile = statusFile.appendingPathExtension("cancel")
            let created = tokenPipe.path.withCString {
                Darwin.mkfifo($0, mode_t(S_IRUSR | S_IWUSR))
            }
            guard created == 0 else { throw CLIAuthenticationLaunchError.couldNotOpenTerminal }
            do {
                try Data("ready\n".utf8).write(to: statusFile, options: .withoutOverwriting)
                try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: statusFile.path)
                try Data().write(to: launchGateFile, options: .withoutOverwriting)
                try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: launchGateFile.path)
                try Data("pending\n".utf8).write(to: cancelRequestFile, options: .withoutOverwriting)
                try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: cancelRequestFile.path)
                try Data("pending\n".utf8).write(to: execStatusFile, options: .withoutOverwriting)
                try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: execStatusFile.path)
            } catch {
                try? FileManager.default.removeItem(at: tokenPipe)
                try? FileManager.default.removeItem(at: statusFile)
                try? FileManager.default.removeItem(at: launchGateFile)
                try? FileManager.default.removeItem(at: execStatusFile)
                try? FileManager.default.removeItem(at: cancelRequestFile)
                throw CLIAuthenticationLaunchError.couldNotOpenTerminal
            }
            completionTokenPipeURL = tokenPipe
            completionStatusFileURL = statusFile
            completionExecStatusFileURL = execStatusFile
            cancellationRequestFileURL = cancelRequestFile
        } else {
            completionTokenPipeURL = nil
            completionStatusFileURL = nil
            completionExecStatusFileURL = nil
            cancellationRequestFileURL = nil
        }
        let script: String
        do {
            script = try scriptBuilder.build(
                provider: provider,
                executableURL: executableURL,
                completionTokenPipeURL: completionTokenPipeURL,
                completionStatusFileURL: completionStatusFileURL,
                completionExecStatusFileURL: completionExecStatusFileURL,
                cancellationRequestFileURL: cancellationRequestFileURL,
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
            if let completionExecStatusFileURL { try? FileManager.default.removeItem(at: completionExecStatusFileURL) }
            if let cancellationRequestFileURL { try? FileManager.default.removeItem(at: cancellationRequestFileURL) }
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
            if let completionExecStatusFileURL { try? FileManager.default.removeItem(at: completionExecStatusFileURL) }
            if let cancellationRequestFileURL { try? FileManager.default.removeItem(at: cancellationRequestFileURL) }
            throw error
        }
        if let completionToken, let completionTokenPipeURL, let completionStatusFileURL,
           let completionExecStatusFileURL, let cancellationRequestFileURL {
            startGeminiTokenWrite(
                completionToken,
                to: completionTokenPipeURL,
                statusURL: completionStatusFileURL,
                execStatusURL: completionExecStatusFileURL,
                cancelRequestURL: cancellationRequestFileURL,
                scriptURL: scriptURL
            )
        }
        return scriptURL
    }

    func cancelPendingGeminiLogin(token: String) {
        guard let pending = pendingGeminiTokenWrites.removeValue(forKey: token) else { return }
        pending.task.cancel()
        try? FileManager.default.removeItem(at: pending.pipeURL)
        try? FileManager.default.removeItem(at: pending.statusURL)
        try? FileManager.default.removeItem(at: pending.statusURL.appendingPathExtension("lock"))
        try? FileManager.default.removeItem(at: pending.execStatusURL)
        try? FileManager.default.removeItem(at: pending.cancelRequestURL)
        try? FileManager.default.removeItem(at: pending.statusURL.appendingPathExtension("stopped"))
        try? FileManager.default.removeItem(at: pending.scriptURL)
    }

    @discardableResult
    func requestGeminiLoginStop(token: String) -> Task<Bool, Never> {
        guard let pending = pendingGeminiTokenWrites[token] else { return Task { false } }
        if let inFlight = pendingGeminiStops[token], inFlight.statusURL == pending.statusURL {
            return inFlight.task
        }
        pending.task.cancel()
        let statusURL = pending.statusURL
        let gateURL = statusURL.appendingPathExtension("lock")
        let requestURL = pending.cancelRequestURL
        let execStatusURL = pending.execStatusURL
        let pipeURL = pending.pipeURL
        let scriptURL = pending.scriptURL
        let gateLockTimeout = geminiStopGateLockTimeout
        let acknowledgementTimeout = geminiStopAcknowledgementTimeout
        let uncertainStatuses = geminiCleanupUncertainStatuses
        let completedFailureStatuses = geminiCompletedFailureStatuses
        let stoppedURL = statusURL.appendingPathExtension("stopped")
        let stopID = UUID()
        let stopTask = Task.detached(priority: .utility) {
            // The helper exclusively writes responses. A cancellation request
            // can never replace a late acknowledgement or uncertainty proof.
            func terminalResult() -> Bool? {
                let status = (try? String(contentsOf: statusURL, encoding: .utf8))?
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                if status == "helper_cancelled" {
                    for url in [pipeURL, statusURL, gateURL, execStatusURL, requestURL, scriptURL, stoppedURL] {
                        Darwin.unlink(url.path)
                    }
                    return true
                }
                let execStatus = (try? String(contentsOf: execStatusURL, encoding: .utf8))?
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                if [status, execStatus].compactMap({ $0 }).contains(where: uncertainStatuses.contains) {
                    return false
                }
                if let status, completedFailureStatuses.contains(status),
                   let acknowledgement = try? String(contentsOf: stoppedURL, encoding: .utf8),
                   acknowledgement == "helper_stopped_v1:\(statusURL.lastPathComponent):\(status)\n" {
                    // The helper has reaped its owners, completed EXIT cleanup,
                    // and disabled later writers before atomically publishing.
                    for url in [pipeURL, statusURL, gateURL, execStatusURL, requestURL, scriptURL, stoppedURL] {
                        Darwin.unlink(url.path)
                    }
                    return true
                }
                return nil
            }
            if let result = terminalResult() { return result }
            do {
                try Data("cancelled\n".utf8).write(to: requestURL, options: .atomic)
                try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: requestURL.path)
            } catch {
                // Failure to publish a request is not evidence of process exit.
                return false
            }
            Darwin.unlink(pipeURL.path)
            let lockDeadline = ContinuousClock.now.advanced(by: gateLockTimeout)
            while ContinuousClock.now < lockDeadline {
                if let result = terminalResult() { return result }
                let descriptor = gateURL.path.withCString { Darwin.open($0, O_RDWR) }
                if descriptor >= 0 {
                    if Darwin.lockf(descriptor, F_TLOCK, 0) == 0 {
                        _ = Darwin.lockf(descriptor, F_ULOCK, 0)
                        Darwin.close(descriptor)
                        break
                    }
                    let lockError = errno
                    Darwin.close(descriptor)
                    guard lockError == EINTR || lockError == EACCES || lockError == EAGAIN else { break }
                } else if errno == ENOENT { break }
                try? await Task.sleep(for: .milliseconds(100))
            }
            // If the gate remains busy, the already-published request is the
            // bounded fallback. The helper and watchdog both observe it.
            let deadline = ContinuousClock.now.advanced(by: acknowledgementTimeout)
            repeat {
                if let result = terminalResult() { return result }
                guard ContinuousClock.now < deadline else { break }
                try? await Task.sleep(for: .milliseconds(100))
            } while true
            NSLog("AI Token Meter: Antigravity helper did not acknowledge stop; preserving recovery state")
            return false
        }
        let sharedTask = Task { @MainActor [weak self] in
            let confirmed = await stopTask.value
            if self?.pendingGeminiStops[token]?.id == stopID {
                self?.pendingGeminiStops.removeValue(forKey: token)
            }
            if confirmed, self?.pendingGeminiTokenWrites[token]?.statusURL == statusURL {
                self?.pendingGeminiTokenWrites.removeValue(forKey: token)
            }
            return confirmed
        }
        pendingGeminiStops[token] = (stopID, statusURL, sharedTask)
        return sharedTask
    }

    func hasGeminiTerminalFailure(token: String) -> Bool {
        guard let pending = pendingGeminiTokenWrites[token] else { return false }
        let status = (try? String(contentsOf: pending.statusURL, encoding: .utf8))?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        let execStatus = (try? String(contentsOf: pending.execStatusURL, encoding: .utf8))?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return [status, execStatus].compactMap { $0 }.contains { geminiTerminalFailureStatuses.contains($0) }
    }

    func hasGeminiCleanupUncertainty(token: String) -> Bool {
        guard let pending = pendingGeminiTokenWrites[token] else { return false }
        let status = (try? String(contentsOf: pending.statusURL, encoding: .utf8))?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        let execStatus = (try? String(contentsOf: pending.execStatusURL, encoding: .utf8))?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return [status, execStatus].compactMap { $0 }.contains {
            geminiCleanupUncertainStatuses.contains($0)
        }
    }

    func hasGeminiCLIExecConfirmed(token: String) -> Bool {
        guard let pending = pendingGeminiTokenWrites[token],
              let status = try? String(contentsOf: pending.execStatusURL, encoding: .utf8) else { return false }
        return status.trimmingCharacters(in: .whitespacesAndNewlines) == "exec_confirmed"
    }

    private func startGeminiTokenWrite(
        _ token: String,
        to pipeURL: URL,
        statusURL: URL,
        execStatusURL: URL,
        cancelRequestURL: URL,
        scriptURL: URL
    ) {
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
        pendingGeminiTokenWrites[token] = (pipeURL, statusURL, execStatusURL, cancelRequestURL, scriptURL, task)
    }
}
