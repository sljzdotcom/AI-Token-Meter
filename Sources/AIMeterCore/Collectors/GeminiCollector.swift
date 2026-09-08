import CoreFoundation
import Darwin
import Foundation

public struct GeminiCollector: UsageCollector {
    public let provider = UsageProvider.gemini
    private let runner: any CommandRunning
    private let locator: any ExecutableLocating
    private let environment: GeminiCLIEnvironment
    public init(runner: any CommandRunning = PTYCommandRunner(), locator: any ExecutableLocating = ExecutableLocator(), environment: GeminiCLIEnvironment = GeminiCLIEnvironment()) {
        self.runner = runner; self.locator = locator; self.environment = environment
    }
    public func collect() async throws -> UsageSnapshot {
        let executable: URL
        switch locator.discover(named: "gemini") {
        case .found(let url): executable = url
        case .missing: throw UsageCollectionError.notInstalled
        case .unavailable: throw UsageCollectionError.geminiUnavailable("Gemini CLI is not executable")
        }
        let context = try environment.prepare(executable: executable)
        defer { try? FileManager.default.removeItem(at: context.directory) }
        let arguments = ["-e", "none", "--allowed-mcp-server-names", "ai-meter-\(UUID().uuidString)"]
        let version = try await runner.run(CommandRequest(executableURL: executable, arguments: ["--version"] + arguments,
            inputLines: [], timeout: 8, currentDirectoryURL: context.directory, environment: context.environment))
        try Task.checkCancellation()
        guard version.exitCode == 0 else { throw UsageCollectionError.transportFailure }
        let lines = ANSITextSanitizer.sanitize(version.output).components(separatedBy: .newlines).map { $0.trimmingCharacters(in: .whitespaces) }
        guard lines.filter({ $0.range(of: #"^\d+\.\d+\.\d+(?:[-+].*)?$"#, options: .regularExpression) != nil }) == ["0.58.0"] else {
            throw UsageCollectionError.geminiUnavailable("Gemini CLI version is not supported (requires 0.58.0)")
        }
        let result = try await runner.run(CommandRequest(executableURL: executable, arguments: arguments,
            inputLines: [], timeout: 25, currentDirectoryURL: context.directory, environment: context.environment, geminiQuotaInteraction: true))
        try Task.checkCancellation()
        if let error = GeminiTerminalProtocol.blockingError(result.output) { throw error }
        guard result.exitCode == 0 else { throw UsageCollectionError.transportFailure }
        return try GeminiUsageParser().parse(result.output)
    }
}

/// Reads non-secret configuration only. Authentication files are exclusively the CLI's concern.
public struct GeminiCLIEnvironment: Sendable {
    private let home: URL
    private let source: [String: String]
    private let systemDirectory: URL
    public init() {
        self.init(home: FileManager.default.homeDirectoryForCurrentUser, source: ProcessInfo.processInfo.environment,
                  systemDirectory: URL(fileURLWithPath: "/Library/Application Support/GeminiCli"))
    }
    init(home: URL, source: [String: String], systemDirectory: URL) {
        self.home = home; self.source = source; self.systemDirectory = systemDirectory
    }
    func prepare(executable: URL) throws -> (directory: URL, environment: [String: String]) {
        for (key, value) in source where !value.isEmpty {
            if ["GEMINI_", "GOOGLE_", "GCLOUD_", "CLOUDSDK_", "NODE_"].contains(where: key.hasPrefix)
                || ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy", "LD_PRELOAD", "DYLD_INSERT_LIBRARIES", "SANDBOX", "BUILD_SANDBOX", "SEATBELT_PROFILE", "SANDBOX_FLAGS", "SANDBOX_MOUNTS", "SANDBOX_ENV", "CLOUD_SHELL", "BASH_ENV", "ENV", "ZDOTDIR"].contains(key) {
                throw UsageCollectionError.geminiUnavailable("Gemini CLI environment uses an unsupported override")
            }
        }
        for filename in ["settings.json", "system-defaults.json"] {
            var info = stat()
            let result = lstat(systemDirectory.appendingPathComponent(filename).path, &info)
            guard result != 0 && (errno == ENOENT || errno == ENOTDIR) else {
                throw UsageCollectionError.geminiUnavailable("Gemini CLI system settings cannot be safely preserved")
            }
        }
        let settingsURL = home.appendingPathComponent(".gemini/settings.json")
        let settings: [String: Any]
        do {
            let data = try Data(contentsOf: settingsURL)
            guard data.count <= 1_048_576, let decoded = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
                throw UsageCollectionError.geminiUnavailable("Gemini CLI settings are not supported")
            }
            settings = decoded
        } catch let error as CocoaError where error.code == .fileReadNoSuchFile {
            throw UsageCollectionError.authenticationRequired
        } catch let error as UsageCollectionError { throw error }
        catch { throw UsageCollectionError.geminiUnavailable("Gemini CLI settings cannot be read") }
        for key in ["security", "tools", "advanced"] where settings[key] != nil {
            guard settings[key] is [String: Any] else { throw UsageCollectionError.geminiUnavailable("Gemini CLI settings are not supported") }
        }
        let security = settings["security"] as? [String: Any]
        let auth = security?["auth"] as? [String: Any]
        guard let mode = auth?["selectedType"] as? String else { throw UsageCollectionError.authenticationRequired }
        guard mode == "oauth-personal" else { throw UsageCollectionError.geminiUnavailable("Gemini CLI authentication mode is not supported") }
        if !Self.disabledOrAbsent(auth?["useExternal"]) || !Self.disabledOrAbsent(security?["toolSandboxing"])
            || (auth?["enforcedType"] != nil && auth?["enforcedType"] as? String != "oauth-personal") {
            throw UsageCollectionError.geminiUnavailable("Gemini CLI security mode is not supported")
        }
        let tools = settings["tools"] as? [String: Any] ?? [:]
        let advanced = settings["advanced"] as? [String: Any] ?? [:]
        for key in ["discoveryCommand", "callCommand"] {
            if let value = tools[key], !(value is NSNull), String(describing: value) != "" {
                throw UsageCollectionError.geminiUnavailable("Gemini CLI custom tools are not supported")
            }
        }
        if !Self.disabledOrAbsent(tools["sandbox"]) {
            throw UsageCollectionError.geminiUnavailable("Gemini CLI sandbox mode is not supported")
        }
        if !Self.disabledOrAbsent(advanced["ignoreLocalEnv"]) {
            throw UsageCollectionError.geminiUnavailable("Gemini CLI local environment isolation is disabled")
        }
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("ai-meter-gemini-\(UUID())", isDirectory: true)
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            try Data().write(to: directory.appendingPathComponent(".env"))
            let overrides: [String: Any] = [
                "hooksConfig": ["enabled": false],
                "context": ["memoryBoundaryMarkers": [], "includeDirectories": [], "fileName": []],
                "privacy": ["usageStatisticsEnabled": false], "telemetry": ["enabled": false], "ide": ["enabled": false],
                "general": ["enableAutoUpdate": false], "advanced": ["autoConfigureMemory": false]
            ]
            let settingsPath = directory.appendingPathComponent("system.json")
            let defaultsPath = directory.appendingPathComponent("system-defaults.json")
            try JSONSerialization.data(withJSONObject: overrides).write(to: settingsPath)
            try Data("{}".utf8).write(to: defaultsPath)
            for file in [settingsPath, defaultsPath, directory.appendingPathComponent(".env")] {
                try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: file.path)
            }
            return (directory, ["HOME": home.path, "USERPROFILE": home.path,
                "PATH": executable.deletingLastPathComponent().path + ":/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
                "TERM": "xterm-256color", "LANG": "en_US.UTF-8", "SHELL": "/bin/sh", "NO_BROWSER": "true",
                "GEMINI_CLI_NO_RELAUNCH": "true", "GEMINI_CLI_TRUST_WORKSPACE": "false",
                "GEMINI_CLI_SYSTEM_SETTINGS_PATH": settingsPath.path, "GEMINI_CLI_SYSTEM_DEFAULTS_PATH": defaultsPath.path,
                "TMPDIR": directory.path])
        } catch {
            try? FileManager.default.removeItem(at: directory)
            throw UsageCollectionError.transportFailure
        }
    }
    private static func disabledOrAbsent(_ value: Any?) -> Bool {
        guard let value else { return true }
        guard let number = value as? NSNumber, CFGetTypeID(number) == CFBooleanGetTypeID() else { return false }
        return !number.boolValue
    }

}
