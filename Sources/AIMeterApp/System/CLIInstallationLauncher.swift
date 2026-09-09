import AIMeterCore
import AppKit
import Foundation

enum CLIInstallationLaunchError: Error { case unavailable }

@MainActor
final class CLIInstallationLauncher {
    private let directory: URL
    private let locator: any ExecutableLocating
    private let openURL: (URL) -> Bool

    init(directory: URL? = nil, locator: any ExecutableLocating = ExecutableLocator(), openURL: @escaping (URL) -> Bool = { NSWorkspace.shared.open($0) }) {
        self.directory = directory ?? FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appendingPathComponent("AI Meter/Installation", isDirectory: true)
        self.locator = locator
        self.openURL = openURL
    }

    /// False means discovery found an existing executable; no installer was opened.
    func open(provider: UsageProvider) throws -> Bool {
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
        guard openURL(url) else { throw CLIAuthenticationLaunchError.couldNotOpenTerminal }
        return true
    }
}

struct CLIServiceAction {
    let title: String
    let needsAttention: Bool
    let isEnabled: Bool
    let showsSeparateStatusCheck: Bool

    init(state: ServiceAccountConnectionState, busy: Bool) {
        title = busy ? "Waiting for Terminal…" : state == .notInstalled ? "Install CLI" : state == .connected ? "Sign in again" : state == .unavailable ? "Check Status" : "Sign in"
        needsAttention = !busy && [.notInstalled, .signInRequired].contains(state)
        isEnabled = !busy && state != .checking
        showsSeparateStatusCheck = state != .unavailable
    }
}
