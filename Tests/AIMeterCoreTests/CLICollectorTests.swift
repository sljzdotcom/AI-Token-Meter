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
            locator: FixedLocator(url: fixtureExecutable)
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
            locator: FixedLocator(url: nil)
        )

        await #expect(throws: UsageCollectionError.notInstalled) {
            try await collector.collect()
        }
    }

    @Test("Claude authentication preflight reports a logged-out account")
    func claudeLoggedOutIsReportedBeforeInteractiveUsage() async {
        let collector = ClaudeCollector(
            runner: PTYCommandRunner(),
            locator: FixedLocator(url: loggedOutClaudeExecutable)
        )

        await #expect(throws: UsageCollectionError.authenticationRequired) {
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
