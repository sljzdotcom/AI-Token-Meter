import AIMeterCore
import AppKit
import Foundation

enum CLIInstallationLaunchError: Error {
    case unavailable
    case couldNotOpenTerminal
    case externalActionsDisabled
}

@MainActor
final class CLIInstallationLauncher {
    private let directory: URL
    private let locator: any ExecutableLocating
    private let openURL: (URL) -> Bool
    private let systemActionPolicy: SystemActionPolicy
    private let usesSystemOpener: Bool

    init(
        directory: URL? = nil,
        locator: any ExecutableLocating = ExecutableLocator(),
        openURL: ((URL) -> Bool)? = nil,
        systemActionPolicy: SystemActionPolicy = .current
    ) {
        let applicationSupportDirectory = FileManager.default.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        )[0]
        self.directory = directory ?? systemActionPolicy.scriptDirectory(
            name: "Installation",
            applicationSupportDirectory: applicationSupportDirectory,
            temporaryDirectory: FileManager.default.temporaryDirectory
        )
        self.locator = locator
        self.systemActionPolicy = systemActionPolicy
        self.usesSystemOpener = openURL == nil
        self.openURL = openURL ?? { NSWorkspace.shared.open($0) }
    }

    /// False means discovery found an existing executable; no installer was opened.
    func open(provider: UsageProvider) throws -> Bool {
        guard !usesSystemOpener || systemActionPolicy.allowsExternalOpen else {
            throw CLIInstallationLaunchError.externalActionsDisabled
        }
        let script = try CLIInstallationScriptBuilder().build(provider: provider)
        let name = provider == .claude ? "claude" : "codex"
        switch locator.discover(named: name) {
        case .found: return false
        case .unavailable: throw CLIInstallationLaunchError.unavailable
        case .missing: break
        }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        let url = directory.appendingPathComponent("Install \(name).command")
        try Data(script.utf8).write(to: url, options: .atomic)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: url.path)
        guard openURL(url) else { throw CLIInstallationLaunchError.couldNotOpenTerminal }
        return true
    }
}

struct CLIServiceAction {
    let title: String
    let needsAttention: Bool
    let isEnabled: Bool
    let showsSeparateStatusCheck: Bool

    init(state: ServiceAccountConnectionState, busy: Bool) {
        title = busy ? "Waiting for Terminal…" : state == .notInstalled ? "Install CLI" : state == .connected ? "Sign in again" : [.unavailable, .lastKnown].contains(state) ? "Check Status" : state == .checking ? "Checking account…" : "Sign in"
        needsAttention = !busy && [.notInstalled, .signInRequired].contains(state)
        isEnabled = !busy && state != .checking
        showsSeparateStatusCheck = state != .unavailable
    }
}
