import Foundation

public struct GeminiCollector: UsageCollector {
    public let provider = UsageProvider.gemini
    private let runner: any CommandRunning
    private let locator: any ExecutableLocating
    private let environment: GeminiCLIEnvironment

    public init(
        runner: any CommandRunning = BoundedCommandRunner(),
        locator: any ExecutableLocating = ExecutableLocator(),
        environment: GeminiCLIEnvironment = GeminiCLIEnvironment()
    ) {
        self.runner = runner
        self.locator = locator
        self.environment = environment
    }

    public func collect() async throws -> UsageSnapshot {
        let executable: URL
        switch locator.discover(named: "agy") {
        case .found(let url): executable = url
        case .missing: throw UsageCollectionError.notInstalled
        case .unavailable:
            throw UsageCollectionError.geminiUnavailable("Antigravity CLI is not executable")
        }

        let context = try environment.prepare(executable: executable)
        defer { try? FileManager.default.removeItem(at: context.directory) }
        let logPath = context.directory.appendingPathComponent("agy.log").path
        let shared = ["--log-file", logPath]
        let versionResult = try await runner.run(CommandRequest(
            executableURL: executable,
            arguments: ["--version"] + shared,
            inputLines: [],
            timeout: 8,
            currentDirectoryURL: context.directory,
            environment: context.environment,
            maxOutputBytes: 64 * 1_024
        ))
        try Task.checkCancellation()
        guard versionResult.exitCode == 0 else { throw UsageCollectionError.transportFailure }
        let version = ANSITextSanitizer.sanitize(versionResult.output)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard Self.supports(version: version) else {
            throw UsageCollectionError.geminiUnavailable(
                "Antigravity CLI version is not supported (requires 1.1.28 or later in major version 1)"
            )
        }

        let usageResult = try await runner.run(CommandRequest(
            executableURL: executable,
            arguments: ["-p", "/usage", "--print-timeout", "20s"] + shared,
            inputLines: [],
            timeout: 30,
            currentDirectoryURL: context.directory,
            environment: context.environment,
            maxOutputBytes: 64 * 1_024
        ))
        try Task.checkCancellation()
        guard usageResult.exitCode == 0 else {
            let message = ANSITextSanitizer.sanitize(usageResult.output).lowercased()
            if [
                "authentication required", "sign in required", "please sign in",
                "not authenticated", "login required",
            ].contains(where: message.contains) {
                throw UsageCollectionError.authenticationRequired
            }
            throw UsageCollectionError.transportFailure
        }
        let snapshot = try GeminiUsageParser().parse(usageResult.output, sourceVersion: version)
        let currentModelOutput = try await optionalOutput(CommandRequest(
            executableURL: executable,
            arguments: ["-p", "/model", "--print-timeout", "10s"] + shared,
            inputLines: [],
            timeout: 15,
            currentDirectoryURL: context.directory,
            environment: context.environment,
            maxOutputBytes: 64 * 1_024
        ))
        let modelsOutput = try await optionalOutput(CommandRequest(
            executableURL: executable,
            arguments: ["models"] + shared,
            inputLines: [],
            timeout: 15,
            currentDirectoryURL: context.directory,
            environment: context.environment,
            maxOutputBytes: 64 * 1_024
        ))
        let currentModel = currentModelOutput.flatMap {
            AntigravityCLIInfoParser.currentGeminiModel(from: $0)
        }
        let catalog = modelsOutput.flatMap {
            AntigravityCLIInfoParser.geminiCatalog(from: $0)
        }
        guard currentModel != nil || catalog != nil else { return snapshot }
        return snapshot.withAntigravityCLIInfo(AntigravityCLIInfo(
            currentModel: currentModel,
            availableModelCount: catalog?.modelCount,
            modelFamilies: catalog?.families ?? []
        ))
    }

    private func optionalOutput(_ request: CommandRequest) async throws -> String? {
        do {
            let result = try await runner.run(request)
            try Task.checkCancellation()
            return result.exitCode == 0 ? result.output : nil
        } catch is CancellationError {
            throw CancellationError()
        } catch {
            try Task.checkCancellation()
            return nil
        }
    }

    private static func supports(version: String) -> Bool {
        let parts = version.split(separator: ".", omittingEmptySubsequences: false)
        guard parts.count == 3,
              let major = Int(parts[0]), let minor = Int(parts[1]), let patch = Int(parts[2]),
              major == 1 else { return false }
        return minor > 1 || (minor == 1 && patch >= 28)
    }
}

/// Creates an empty working directory and a minimal environment while leaving
/// credentials and account configuration exclusively under Antigravity's control.
public struct GeminiCLIEnvironment: Sendable {
    private let home: URL
    private let source: [String: String]

    public init() {
        self.init(
            home: FileManager.default.homeDirectoryForCurrentUser,
            source: ProcessInfo.processInfo.environment
        )
    }

    init(home: URL, source: [String: String]) {
        self.home = home
        self.source = source
    }

    func prepare(executable: URL) throws -> (directory: URL, environment: [String: String]) {
        let forbiddenPrefixes = ["AGY_", "GEMINI_", "GOOGLE_", "GCLOUD_", "CLOUDSDK_", "NODE_"]
        let forbiddenKeys = [
            "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy",
            "SSL_CERT_FILE", "LD_PRELOAD", "DYLD_INSERT_LIBRARIES", "DYLD_LIBRARY_PATH",
            "BASH_ENV", "ENV", "ZDOTDIR",
        ]
        for (key, value) in source where !value.isEmpty {
            if forbiddenPrefixes.contains(where: key.hasPrefix) || forbiddenKeys.contains(key) {
                throw UsageCollectionError.geminiUnavailable(
                    "Antigravity CLI environment uses an unsupported override"
                )
            }
        }

        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-antigravity-\(UUID())", isDirectory: true)
        do {
            try FileManager.default.createDirectory(
                at: directory,
                withIntermediateDirectories: true,
                attributes: [.posixPermissions: 0o700]
            )
            try Data().write(to: directory.appendingPathComponent(".env"))
            return (directory, [
                "HOME": home.path,
                "USERPROFILE": home.path,
                "PATH": executable.deletingLastPathComponent().path
                    + ":/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
                "TERM": "dumb",
                "LANG": "en_US.UTF-8",
                "SHELL": "/bin/sh",
                "NO_BROWSER": "true",
                "TMPDIR": directory.path,
            ])
        } catch {
            try? FileManager.default.removeItem(at: directory)
            throw UsageCollectionError.transportFailure
        }
    }
}
