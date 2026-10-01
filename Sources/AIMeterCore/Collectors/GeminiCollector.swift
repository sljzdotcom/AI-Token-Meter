import Foundation

public struct GeminiCollector: UsageCollector {
    public let provider = UsageProvider.gemini
    private let runner: any CommandRunning
    private let locator: any ExecutableLocating
    private let environment: GeminiCLIEnvironment
    private let diagnostics: GeminiDiagnosticStore?

    public init(
        runner: any CommandRunning = ProcessGroupCommandRunner(),
        locator: any ExecutableLocating = ExecutableLocator(),
        environment: GeminiCLIEnvironment = GeminiCLIEnvironment(),
        diagnostics: GeminiDiagnosticStore? = nil
    ) {
        self.runner = runner
        self.locator = locator
        self.environment = environment
        self.diagnostics = diagnostics
    }

    public func collect() async throws -> UsageSnapshot {
        let discoveryStartedAt = Date()
        let executable: URL
        switch locator.discover(named: "agy") {
        case .found(let url):
            executable = url
            await record(.discovery, .succeeded, since: discoveryStartedAt)
        case .missing:
            await record(.discovery, .notInstalled, since: discoveryStartedAt)
            throw UsageCollectionError.notInstalled
        case .unavailable:
            await record(.discovery, .notInstalled, since: discoveryStartedAt)
            throw UsageCollectionError.executableUnavailable
        }

        let environmentStartedAt = Date()
        let context: (directory: URL, environment: [String: String])
        do {
            context = try environment.prepare(executable: executable)
            await record(.environment, .succeeded, since: environmentStartedAt)
        } catch let error as UsageCollectionError {
            await record(.environment, Self.category(for: error), since: environmentStartedAt)
            throw error
        }
        defer { try? FileManager.default.removeItem(at: context.directory) }
        let logPath = context.directory.appendingPathComponent("agy.log").path
        let shared = ["--log-file", logPath]
        let versionStartedAt = Date()
        let versionResult = try await run(CommandRequest(
            executableURL: executable,
            arguments: ["--version"] + shared,
            inputLines: [],
            timeout: 8,
            currentDirectoryURL: context.directory,
            environment: context.environment,
            maxOutputBytes: 64 * 1_024
        ), stage: .version, startedAt: versionStartedAt)
        guard versionResult.exitCode == 0 else {
            await record(.version, .transportFailure, since: versionStartedAt)
            throw UsageCollectionError.transportFailure
        }
        let version = ANSITextSanitizer.sanitize(versionResult.output)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard Self.supports(version: version) else {
            await record(.version, .unsupportedVersion, since: versionStartedAt)
            throw UsageCollectionError.unsupportedVersion
        }
        await record(.version, .succeeded, since: versionStartedAt)

        let usageStartedAt = Date()
        let usageResult = try await run(CommandRequest(
            executableURL: executable,
            arguments: ["-p", "/usage", "--print-timeout", "20s"] + shared,
            inputLines: [],
            timeout: 30,
            currentDirectoryURL: context.directory,
            environment: context.environment,
            maxOutputBytes: 64 * 1_024
        ), stage: .usage, startedAt: usageStartedAt)
        guard usageResult.exitCode == 0 else {
            let message = ANSITextSanitizer.sanitize(usageResult.output).lowercased()
            if [
                "authentication required", "sign in required", "please sign in",
                "not authenticated", "login required",
            ].contains(where: message.contains) {
                await record(.usage, .authenticationRequired, since: usageStartedAt)
                throw UsageCollectionError.authenticationRequired
            }
            await record(.usage, .transportFailure, since: usageStartedAt)
            throw UsageCollectionError.transportFailure
        }
        let snapshot: UsageSnapshot
        do {
            snapshot = try GeminiUsageParser().parse(usageResult.output, sourceVersion: version)
            await record(.usage, .succeeded, since: usageStartedAt)
        } catch let error as UsageCollectionError {
            await record(.usage, Self.category(for: error), since: usageStartedAt)
            throw error
        } catch {
            await record(.usage, .invalidResponse, since: usageStartedAt)
            throw UsageCollectionError.invalidResponse
        }
        let modelStartedAt = Date()
        let currentModelOutput = try await optionalOutput(CommandRequest(
            executableURL: executable,
            arguments: ["-p", "/model", "--print-timeout", "10s"] + shared,
            inputLines: [],
            timeout: 15,
            currentDirectoryURL: context.directory,
            environment: context.environment,
            maxOutputBytes: 64 * 1_024
        ), stage: .model, startedAt: modelStartedAt)
        let catalogStartedAt = Date()
        let modelsOutput = try await optionalOutput(CommandRequest(
            executableURL: executable,
            arguments: ["models"] + shared,
            inputLines: [],
            timeout: 15,
            currentDirectoryURL: context.directory,
            environment: context.environment,
            maxOutputBytes: 64 * 1_024
        ), stage: .catalog, startedAt: catalogStartedAt)
        let currentModel = currentModelOutput.flatMap {
            AntigravityCLIInfoParser.currentGeminiModel(from: $0)
        }
        let catalog = modelsOutput.flatMap {
            AntigravityCLIInfoParser.geminiCatalog(from: $0)
        }
        if currentModelOutput != nil, currentModel == nil {
            await record(.model, .unrecognizedOutput, since: modelStartedAt)
        }
        if modelsOutput != nil, catalog == nil {
            await record(.catalog, .unrecognizedOutput, since: catalogStartedAt)
        }
        guard currentModel != nil || catalog != nil else { return snapshot }
        return snapshot.withAntigravityCLIInfo(AntigravityCLIInfo(
            currentModel: currentModel,
            availableModelCount: catalog?.modelCount,
            modelFamilies: catalog?.families ?? []
        ))
    }

    private func run(_ request: CommandRequest, stage: GeminiDiagnosticStage, startedAt: Date) async throws -> CommandResult {
        do {
            let result = try await runner.run(request)
            try Task.checkCancellation()
            guard !result.outputTruncated else { throw UsageCollectionError.outputLimitExceeded }
            return result
        } catch is CancellationError {
            await record(stage, .cancelled, since: startedAt)
            throw CancellationError()
        } catch let error as UsageCollectionError {
            await record(stage, Self.category(for: error), since: startedAt)
            throw error
        } catch {
            await record(stage, .transportFailure, since: startedAt)
            throw UsageCollectionError.transportFailure
        }
    }

    private func optionalOutput(
        _ request: CommandRequest,
        stage: GeminiDiagnosticStage,
        startedAt: Date
    ) async throws -> String? {
        do {
            let result = try await run(request, stage: stage, startedAt: startedAt)
            guard result.exitCode == 0 else {
                await record(stage, .transportFailure, since: startedAt)
                return nil
            }
            await record(stage, .succeeded, since: startedAt)
            return result.output
        } catch is CancellationError {
            throw CancellationError()
        } catch is UsageCollectionError {
            try Task.checkCancellation()
            return nil
        }
    }

    private func record(
        _ stage: GeminiDiagnosticStage,
        _ category: GeminiDiagnosticCategory,
        since startedAt: Date,
        outputTruncated: Bool = false
    ) async {
        guard let diagnostics else { return }
        let milliseconds = Int(max(0, Date().timeIntervalSince(startedAt) * 1_000).rounded())
        await diagnostics.record(GeminiDiagnosticRecord(
            recordedAt: Date(),
            stage: stage,
            category: category,
            durationMilliseconds: milliseconds,
            outputTruncated: outputTruncated || category == .outputTruncated
        ))
    }

    private static func category(for error: UsageCollectionError) -> GeminiDiagnosticCategory {
        switch error {
        case .authenticationRequired: .authenticationRequired
        case .timedOut: .timedOut
        case .rateLimited, .rateLimitedRetryAfter: .rateLimited
        case .outputLimitExceeded: .outputTruncated
        case .invalidResponse: .invalidResponse
        case .unrecognizedOutput: .unrecognizedOutput
        case .transportFailure: .transportFailure
        case .notInstalled, .executableUnavailable: .notInstalled
        case .unsupportedVersion: .unsupportedVersion
        case .environmentRejected: .environmentRejected
        case .setupRequired, .geminiUnavailable: .unknown
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
                throw UsageCollectionError.environmentRejected
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
                "TMPDIR": directory.path,
            ])
        } catch {
            try? FileManager.default.removeItem(at: directory)
            throw UsageCollectionError.transportFailure
        }
    }
}
