import Foundation
import Testing
@testable import AIMeterCore

@Suite("PTY command runner", .serialized)
struct PTYCommandRunnerTests {
    @Test("Parent-exit diagnostics retain only recognized numeric fixture metadata")
    func parentExitDiagnosticsSanitizeMetadata() {
        let trace = """
        python_started|100.25|123
        parent_exit_requested|100.5|123
        private-command-and-account-output|100.6|123
        child_detached|nan|456
        child_detached|100.7|not-a-pid
        """
        let phases = ParentExitDiagnostics.sanitizedPhases(trace, startedAt: 100)
        #expect(phases == ["python_started at=0.250s pid=123", "parent_exit_requested at=0.500s pid=123"])
        #expect(!phases.joined().contains("private-command-and-account-output"))
        #expect(ParentExitDiagnostics.sanitizedPhases("", startedAt: 100).isEmpty)
    }

    @Test("Fallback process waits use user initiated quality of service")
    func fallbackProcessWaitQoS() {
        #expect(ProcessTerminationWaiter.fallbackWaitQoSClass == .userInitiated)
    }

    @Test("Sends fixed input and preserves the child exit status")
    func sendsInputAndPreservesExitStatus() async throws {
        let runner = PTYCommandRunner()
        let result = try await runner.run(CommandRequest(
            executableURL: fixtureExecutable,
            arguments: [],
            inputLines: ["fail"],
            timeout: 2
        ))

        #expect(result.output.contains("received:fail"))
        #expect(result.exitCode == 7)
    }

    @Test("Preserves the current user identity for credential lookup")
    func preservesUserIdentity() async throws {
        let result = try await PTYCommandRunner().run(CommandRequest(
            executableURL: fixtureExecutable,
            inputLines: ["identity"],
            timeout: 2
        ))

        #expect(result.output.contains("user:\(NSUserName())"))
    }

    @Test("Runs the child in the requested working directory")
    func usesRequestedWorkingDirectory() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-cwd-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let result = try await PTYCommandRunner().run(CommandRequest(
            executableURL: fixtureExecutable,
            inputLines: ["pwd"],
            timeout: 2,
            currentDirectoryURL: directory
        ))

        #expect(ANSITextSanitizer.sanitize(result.output).contains(directory.path))
    }

    @Test("Returns captured output when a configured stop phrase appears")
    func stopsOnConfiguredOutput() async throws {
        let startedAt = Date()
        let result = try await PTYCommandRunner().run(CommandRequest(
            executableURL: fixtureExecutable,
            inputLines: ["trust"],
            timeout: 2,
            stopAfterOutputContains: ["Permission Required: Accessing workspace"]
        ))

        #expect(result.output.contains("Permission Required: Accessing workspace"))
        #expect(Date().timeIntervalSince(startedAt) < 1.5)
    }

    @Test("Terminates an interactive command after its deadline")
    func terminatesAfterDeadline() async {
        let runner = PTYCommandRunner()

        await #expect(throws: UsageCollectionError.timedOut) {
            try await runner.run(CommandRequest(
                executableURL: fixtureExecutable,
                arguments: [],
                inputLines: ["hang"],
                timeout: 0.1
            ))
        }
    }

    @Test("A timeout requested before process registration is replayed")
    func replaysEarlyTimeoutAfterRegistration() async {
        let registrationGate = DispatchSemaphore(value: 0)
        let runner = PTYCommandRunner {
            registrationGate.wait()
        }
        let startedAt = Date()
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.2) {
            registrationGate.signal()
        }

        await #expect(throws: UsageCollectionError.timedOut) {
            try await runner.run(CommandRequest(
                executableURL: fixtureExecutable,
                arguments: [],
                inputLines: ["hang"],
                timeout: 0.05
            ))
        }

        #expect(Date().timeIntervalSince(startedAt) < 1)
    }

    @Test("Provides a usable terminal window size to TUI applications")
    func providesTerminalWindowSize() async throws {
        let runner = PTYCommandRunner()
        let result = try await runner.run(CommandRequest(
            executableURL: terminalSizeExecutable,
            inputLines: [],
            timeout: 2
        ))

        #expect(ANSITextSanitizer.sanitize(result.output).contains("40 120"))
    }

    @Test("Timeout terminates descendants that keep the PTY open")
    func timeoutTerminatesDescendants() async {
        let runner = PTYCommandRunner()
        let startedAt = Date()

        await #expect(throws: UsageCollectionError.timedOut) {
            try await runner.run(CommandRequest(
                executableURL: descendantHangExecutable,
                inputLines: [],
                timeout: 0.1
            ))
        }
        #expect(Date().timeIntervalSince(startedAt) < 1.5)
    }

    @Test("A parent exit cannot leave the runner waiting on a descendant PTY")
    func parentExitClosesReader() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-parent-exit-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let startedAt = Date()
        let diagnostics = ParentExitDiagnostics(
            traceURL: directory.appendingPathComponent("phases"), startedAt: startedAt)
        let runner = PTYCommandRunner { diagnostics.recordRegistration() }

        do {
            let result = try await runner.run(CommandRequest(
                executableURL: parentExitExecutable,
                arguments: [diagnostics.traceURL.path],
                inputLines: [],
                timeout: 2
            ))

            let elapsed = Date().timeIntervalSince(startedAt)
            let phases = diagnostics.phases()
            if result.exitCode != 0 || !result.output.contains("parent-exited") || elapsed >= 3 {
                print(diagnostics.summary(outcome: "returned exit=\(result.exitCode) output_bytes=\(result.output.utf8.count)"))
            }
            #expect(result.exitCode == 0)
            #expect(result.output.contains("parent-exited"))
            #expect(elapsed < 3)
            // Verify the diagnostic path runs, without making child scheduling a new deadline.
            #expect(phases.contains { $0.hasPrefix("python_started ") })
            #expect(phases.contains { $0.hasPrefix("parent_exit_requested ") })
        } catch {
            let outcome = error as? UsageCollectionError == .timedOut ? "timedOut" : "other_error"
            print(diagnostics.summary(outcome: outcome))
            throw error
        }
    }

    @Test("Captures terminal output that arrives shortly after the parent exits")
    func capturesDelayedTailOutput() async throws {
        let result = try await PTYCommandRunner().run(CommandRequest(
            executableURL: delayedTailExecutable,
            inputLines: [],
            timeout: 2
        ))

        #expect(result.exitCode == 0)
        #expect(result.output.contains("delayed-tail-output"))
    }

    @Test("Concurrent terminal commands do not lose their output")
    func concurrentCommandsPreserveOutput() async throws {
        let results = try await withThrowingTaskGroup(of: (Int, CommandResult).self) { group in
            for index in 0..<32 {
                group.addTask {
                    let result = try await PTYCommandRunner().run(CommandRequest(
                        executableURL: fixtureExecutable,
                        inputLines: ["identity"],
                        timeout: 3
                    ))
                    return (index, result)
                }
            }

            var collected: [(Int, CommandResult)] = []
            for try await result in group {
                collected.append(result)
            }
            return collected
        }

        #expect(results.count == 32)
        #expect(results.allSatisfy { $0.1.exitCode == 0 })
        #expect(results.allSatisfy { $0.1.output.contains("user:\(NSUserName())") })
    }

    private var fixtureExecutable: URL {
        Bundle.module.url(forResource: "fake-interactive-cli", withExtension: "sh")!
    }

    private var terminalSizeExecutable: URL {
        Bundle.module.url(forResource: "fake-terminal-size", withExtension: "sh")!
    }

    private var descendantHangExecutable: URL {
        Bundle.module.url(forResource: "fake-descendant-hang", withExtension: "sh")!
    }

    private var parentExitExecutable: URL {
        Bundle.module.url(forResource: "fake-parent-exits", withExtension: "sh")!
    }

    private var delayedTailExecutable: URL {
        Bundle.module.url(forResource: "fake-delayed-tail", withExtension: "sh")!
    }
}

/// Test-only metadata: no command text, environment, paths, or captured CLI output.
private final class ParentExitDiagnostics: @unchecked Sendable {
    let traceURL: URL
    private let startedAt: Date
    private let monotonicStart = ProcessInfo.processInfo.systemUptime
    private let lock = NSLock()
    private var registrationElapsed: TimeInterval?

    init(traceURL: URL, startedAt: Date) {
        self.traceURL = traceURL
        self.startedAt = startedAt
    }

    func recordRegistration() {
        lock.withLock { registrationElapsed = ProcessInfo.processInfo.systemUptime - monotonicStart }
    }

    func phases() -> [String] {
        let trace = (try? String(contentsOf: traceURL, encoding: .utf8)) ?? ""
        return Self.sanitizedPhases(trace, startedAt: startedAt.timeIntervalSince1970)
    }

    func summary(outcome: String) -> String {
        let registration = lock.withLock { registrationElapsed.map { String(format: "%.3fs", $0) } ?? "not_observed" }
        let elapsed = String(format: "%.3fs", ProcessInfo.processInfo.systemUptime - monotonicStart)
        // Fixture phase offsets use wall time for cross-process comparison; the runner
        // duration and registration offset use a monotonic clock to expose clock jumps.
        return "PTY parent-exit diagnostics: deadline=2.000s outcome=\(outcome) "
            + "runner_elapsed=\(elapsed) before_registration=\(registration) "
            + "fixture_wall_phases=[\(phases().joined(separator: "; "))]"
    }

    static func sanitizedPhases(_ trace: String, startedAt: TimeInterval) -> [String] {
        let allowed = Set(["python_started", "parent_output_flushed", "parent_exit_requested",
                           "child_started", "child_detached"])
        return trace.split(separator: "\n").prefix(32).compactMap { line in
            let fields = line.split(separator: "|", omittingEmptySubsequences: false)
            guard fields.count == 3, allowed.contains(String(fields[0])),
                  let timestamp = Double(fields[1]), timestamp.isFinite,
                  let pid = Int32(fields[2]), pid > 0 else { return nil }
            let elapsed = timestamp - startedAt
            guard elapsed.isFinite else { return nil }
            return "\(fields[0]) at=\(String(format: "%.3f", elapsed))s pid=\(pid)"
        }
    }
}
