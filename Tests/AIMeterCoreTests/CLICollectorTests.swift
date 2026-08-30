import Darwin
import Foundation
import Testing
@testable import AIMeterCore

@Suite("Claude and Codex CLI collectors", .serialized)
struct CLICollectorTests {
    @Test("Claude collector runs usage and parses the resulting snapshot")
    func claudeCollectorReturnsSnapshot() async throws {
        let collector = ClaudeCollector(
            runner: PTYCommandRunner(),
            locator: FixedLocator(url: fixtureExecutable),
            workspaceResolver: FixedWorkspaceResolver(url: FileManager.default.temporaryDirectory)
        )

        let snapshot = try await collector.collect()

        #expect(snapshot.provider == .claude)
        #expect(snapshot.primaryMetric?.usedFraction == 0.73)
    }

    @Test("Codex collector runs status and parses remaining quota")
    func codexCollectorReturnsSnapshot() async throws {
        let collector = CodexCollector(
            locator: FixedLocator(url: fixtureExecutable)
        )

        let snapshot = try await collector.collect()

        #expect(snapshot.provider == .codex)
        #expect(snapshot.primaryMetric?.usedFraction == 0.27)
        #expect(snapshot.secondaryMetric?.usedFraction == 0.08)
        #expect(snapshot.sourceVersion == "codex-app-server")
    }

    @Test("A missing executable is reported without starting a process")
    func missingExecutableIsReported() async {
        let collector = ClaudeCollector(
            runner: PTYCommandRunner(),
            locator: FixedLocator(url: nil),
            workspaceResolver: FixedWorkspaceResolver(url: FileManager.default.temporaryDirectory)
        )

        await #expect(throws: UsageCollectionError.notInstalled) {
            try await collector.collect()
        }
    }

    @Test("Claude authentication preflight reports a logged-out account")
    func claudeLoggedOutIsReportedBeforeInteractiveUsage() async {
        let collector = ClaudeCollector(
            runner: PTYCommandRunner(),
            locator: FixedLocator(url: loggedOutClaudeExecutable),
            workspaceResolver: FixedWorkspaceResolver(url: FileManager.default.temporaryDirectory)
        )

        await #expect(throws: UsageCollectionError.authenticationRequired) {
            try await collector.collect()
        }
    }

    @Test("Claude collector runs all commands in the isolated workspace")
    func claudeUsesIsolatedWorkspace() async throws {
        let workspace = FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-claude-\(UUID().uuidString)", isDirectory: true)
        let runner = RecordingClaudeRunner()
        let collector = ClaudeCollector(
            runner: runner,
            locator: FixedLocator(url: fixtureExecutable),
            workspaceResolver: FixedWorkspaceResolver(url: workspace)
        )

        _ = try await collector.collect()

        let requests = await runner.recordedRequests()
        #expect(requests.count == 2)
        #expect(requests.allSatisfy { $0.currentDirectoryURL == workspace })
        #expect(requests.last?.arguments == ["--ax-screen-reader", "--safe-mode"])
        #expect(requests.last?.inputLines == ["/usage"])
        #expect(requests.last?.inputLineTerminator == "\r")
        #expect(requests.last?.inputDelay == 3)
        #expect(requests.last?.timeout == 20)
        #expect(requests.last?.stopAfterOutputContains.contains("Resets") == true)
    }

    @Test("Claude trust screen reports setup required")
    func claudeTrustScreenRequiresSetup() async throws {
        let workspace = FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-untrusted-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: workspace, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: workspace) }
        let collector = ClaudeCollector(
            runner: PTYCommandRunner(),
            locator: FixedLocator(url: untrustedClaudeExecutable),
            workspaceResolver: FixedWorkspaceResolver(url: workspace)
        )

        await #expect(throws: UsageCollectionError.setupRequired) {
            try await collector.collect()
        }
    }

    @Test("Codex app server timeout kills a process that ignores termination")
    func codexTimeoutIsBounded() async throws {
        let pidFile = FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-codex-\(UUID().uuidString).pid")
        defer {
            try? FileManager.default.removeItem(at: pidFile)
        }
        let client = CodexAppServerClient(environmentOverrides: [
            "AI_METER_TEST_PID_FILE": pidFile.path,
        ])
        let startedAt = Date()

        await #expect(throws: UsageCollectionError.timedOut) {
            try await client.readRateLimits(
                executableURL: ignoredTerminationCodexExecutable,
                timeout: 0.1
            )
        }

        #expect(Date().timeIntervalSince(startedAt) < 1.5)
        let pid = try await readPID(from: pidFile)
        #expect(await processExited(pid, within: 2))
    }

    @Test("Codex replays a timeout requested before process registration")
    func codexEarlyTimeoutIsReplayed() async throws {
        let pidFile = FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-codex-early-\(UUID().uuidString).pid")
        defer {
            try? FileManager.default.removeItem(at: pidFile)
        }
        let registrationGate = DispatchSemaphore(value: 0)
        let client = CodexAppServerClient(
            beforeProcessRegistration: { registrationGate.wait() },
            environmentOverrides: ["AI_METER_TEST_PID_FILE": pidFile.path]
        )
        let startedAt = Date()
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.2) {
            registrationGate.signal()
        }

        await #expect(throws: UsageCollectionError.timedOut) {
            try await client.readRateLimits(
                executableURL: ignoredTerminationCodexExecutable,
                timeout: 0.05
            )
        }

        #expect(Date().timeIntervalSince(startedAt) < 1)
        let pid = try await readPID(from: pidFile)
        #expect(await processExited(pid, within: 2))
    }

    private func readPID(from file: URL) async throws -> pid_t {
        for _ in 0..<50 {
            if let contents = try? String(contentsOf: file, encoding: .utf8),
               let pid = pid_t(contents.trimmingCharacters(in: .whitespacesAndNewlines)) {
                return pid
            }
            try await Task.sleep(for: .milliseconds(20))
        }
        throw UsageCollectionError.transportFailure
    }

    private func processExited(_ pid: pid_t, within timeout: TimeInterval) async -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if kill(pid, 0) == -1, errno == ESRCH { return true }
            try? await Task.sleep(for: .milliseconds(20))
        }
        return kill(pid, 0) == -1 && errno == ESRCH
    }

    private var fixtureExecutable: URL {
        Bundle.module.url(forResource: "fake-interactive-cli", withExtension: "sh")!
    }

    private var loggedOutClaudeExecutable: URL {
        Bundle.module.url(forResource: "fake-logged-out-claude", withExtension: "sh")!
    }

    private var untrustedClaudeExecutable: URL {
        Bundle.module.url(forResource: "fake-untrusted-claude", withExtension: "sh")!
    }

    private var ignoredTerminationCodexExecutable: URL {
        Bundle.module.url(forResource: "fake-codex-ignore-term", withExtension: "sh")!
    }
}

private struct FixedLocator: ExecutableLocating {
    let url: URL?

    func locate(named name: String) -> URL? {
        url
    }
}

private struct FixedWorkspaceResolver: ClaudeUsageWorkspaceResolving {
    let url: URL

    func resolve() throws -> URL {
        url
    }
}

private actor RecordingClaudeRunner: CommandRunning {
    private var requests: [CommandRequest] = []

    func run(_ request: CommandRequest) async throws -> CommandResult {
        requests.append(request)
        let output = request.arguments == ["auth", "status"]
            ? #"{"loggedIn":true}"#
            : "Current session\n73% used\nResets in 51 min\nAll models\n7% used\nResets Thu 12:00 AM\n"
        return CommandResult(output: output, exitCode: 0, duration: 0)
    }

    func recordedRequests() -> [CommandRequest] {
        requests
    }
}
