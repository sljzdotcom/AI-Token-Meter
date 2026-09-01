import Foundation

public struct ClaudeCollector: UsageCollector {
    public let provider = UsageProvider.claude

    private let runner: any CommandRunning
    private let locator: any ExecutableLocating
    private let parser: ClaudeUsageParser
    private let accountStatusParser: ClaudeAccountStatusParser
    private let workspaceResolver: any ClaudeUsageWorkspaceResolving
    private let localActivityReader: TimedOptionalClaudeLocalActivityReader

    public init(
        runner: any CommandRunning = PTYCommandRunner(),
        locator: any ExecutableLocating = ExecutableLocator(),
        parser: ClaudeUsageParser = ClaudeUsageParser(),
        accountStatusParser: ClaudeAccountStatusParser = ClaudeAccountStatusParser(),
        workspaceResolver: any ClaudeUsageWorkspaceResolving = ClaudeUsageWorkspaceResolver()
    ) {
        self.runner = runner
        self.locator = locator
        self.parser = parser
        self.accountStatusParser = accountStatusParser
        self.workspaceResolver = workspaceResolver
        self.localActivityReader = TimedOptionalClaudeLocalActivityReader(
            reader: ClaudeLocalActivityReader(),
            timeout: .seconds(2)
        )
    }

    init(
        runner: any CommandRunning,
        locator: any ExecutableLocating,
        parser: ClaudeUsageParser = ClaudeUsageParser(),
        accountStatusParser: ClaudeAccountStatusParser = ClaudeAccountStatusParser(),
        workspaceResolver: any ClaudeUsageWorkspaceResolving,
        localActivityReader: any ClaudeLocalActivityReading,
        localActivityTimeout: Duration = .seconds(2)
    ) {
        self.runner = runner
        self.locator = locator
        self.parser = parser
        self.accountStatusParser = accountStatusParser
        self.workspaceResolver = workspaceResolver
        self.localActivityReader = TimedOptionalClaudeLocalActivityReader(
            reader: localActivityReader,
            timeout: localActivityTimeout
        )
    }

    public func collect() async throws -> UsageSnapshot {
        let localActivity = Task(priority: .utility) {
            await localActivityReader.read(now: Date(), dayCount: 30)
        }
        let official = try await collectOfficialUsage()
        return official.withClaudeLocalActivity(await localActivity.value)
    }

    private func collectOfficialUsage() async throws -> UsageSnapshot {
        guard let executableURL = locator.locate(named: "claude") else {
            throw UsageCollectionError.notInstalled
        }
        let workspaceURL: URL
        do {
            workspaceURL = try workspaceResolver.resolve()
        } catch {
            throw UsageCollectionError.transportFailure
        }

        try await verifyAuthentication(
            executableURL: executableURL,
            workspaceURL: workspaceURL
        )

        let result = try await runner.run(CommandRequest(
            executableURL: executableURL,
            arguments: ["--ax-screen-reader", "--safe-mode"],
            inputLines: ["/usage"],
            inputLineTerminator: "\r",
            inputDelay: 3,
            timeout: 20,
            currentDirectoryURL: workspaceURL,
            stopAfterOutputContains: [
                "Resets",
                "Permission Required: Accessing workspace",
            ]
        ))
        try detectWorkspaceTrustRequirement(in: result.output)
        try detectAuthenticationFailure(in: result.output)

        do {
            return try parser.parse(result.output)
        } catch UsageCollectionError.unrecognizedOutput where result.exitCode != 0 {
            throw UsageCollectionError.transportFailure
        }
    }

    private func verifyAuthentication(
        executableURL: URL,
        workspaceURL: URL
    ) async throws {
        let result = try await runner.run(CommandRequest(
            executableURL: executableURL,
            arguments: ["auth", "status"],
            inputLines: [],
            timeout: 5,
            currentDirectoryURL: workspaceURL
        ))
        if let authStatus = try? accountStatusParser.parse(result.output) {
            if authStatus.connectionState == .signInRequired {
                throw UsageCollectionError.authenticationRequired
            }
            return
        }

        if result.exitCode != 0 {
            throw UsageCollectionError.transportFailure
        }
    }

    private func detectWorkspaceTrustRequirement(in output: String) throws {
        let normalized = ANSITextSanitizer.sanitize(output).lowercased()
        if normalized.contains("permission required: accessing workspace") {
            throw UsageCollectionError.setupRequired
        }
    }

    private func detectAuthenticationFailure(in output: String) throws {
        let normalized = ANSITextSanitizer.sanitize(output).lowercased()
        if normalized.contains("not logged in")
            || normalized.contains("please log in")
            || normalized.contains("login required")
            || normalized.contains("需要登录") {
            throw UsageCollectionError.authenticationRequired
        }
    }
}

private enum ClaudeLocalActivityReadOutcome: Sendable {
    case value(ClaudeLocalActivitySummary?)
    case timedOut
}

private actor TimedOptionalClaudeLocalActivityReader {
    private let reader: any ClaudeLocalActivityReading
    private let timeout: Duration
    private var inFlightToken: UUID?

    init(reader: any ClaudeLocalActivityReading, timeout: Duration) {
        self.reader = reader
        self.timeout = timeout
    }

    func read(now: Date, dayCount: Int) async -> ClaudeLocalActivitySummary? {
        guard inFlightToken == nil else { return nil }

        let token = UUID()
        let reader = reader
        let task = Task(priority: .utility) {
            ClaudeLocalActivityReadOutcome.value(
                try? await reader.read(now: now, dayCount: dayCount)
            )
        }
        inFlightToken = token

        Task { [weak self] in
            _ = await task.value
            await self?.finish(token: token)
        }

        switch await Self.firstResult(from: task, timeout: timeout) {
        case .value(let summary): return summary
        case .timedOut: return nil
        }
    }

    private func finish(token: UUID) {
        guard inFlightToken == token else { return }
        inFlightToken = nil
    }

    nonisolated private static func firstResult(
        from task: Task<ClaudeLocalActivityReadOutcome, Never>,
        timeout: Duration
    ) async -> ClaudeLocalActivityReadOutcome {
        await withCheckedContinuation { continuation in
            let gate = ClaudeActivityOneShotContinuation(continuation)
            Task.detached { gate.resume(returning: await task.value) }
            Task.detached {
                try? await Task.sleep(for: timeout)
                gate.resume(returning: .timedOut)
            }
        }
    }
}

private final class ClaudeActivityOneShotContinuation<Value: Sendable>: @unchecked Sendable {
    private let lock = NSLock()
    private var continuation: CheckedContinuation<Value, Never>?

    init(_ continuation: CheckedContinuation<Value, Never>) {
        self.continuation = continuation
    }

    func resume(returning value: Value) {
        let continuation = lock.withLock {
            defer { self.continuation = nil }
            return self.continuation
        }
        continuation?.resume(returning: value)
    }
}
