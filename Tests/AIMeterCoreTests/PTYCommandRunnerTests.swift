import Foundation
import Testing
@testable import AIMeterCore

@Suite("PTY command runner", .serialized)
struct PTYCommandRunnerTests {
    @Test("Parent-exit diagnostics retain only recognized numeric fixture metadata")
    func parentExitDiagnosticsSanitizeMetadata() {
        let trace = """
        shell_started|123
        parent_exit_requested|123
        private-command-and-account-output|123
        child_spawned|not-a-pid
        """
        let phases = ParentExitDiagnostics.sanitizedPhases(trace)
        #expect(phases == ["shell_started pid=123", "parent_exit_requested pid=123"])
        #expect(!phases.joined().contains("private-command-and-account-output"))
        #expect(ParentExitDiagnostics.sanitizedPhases("").isEmpty)
    }

    @Test("Fallback process waits use user initiated quality of service")
    func fallbackProcessWaitQoS() {
        #expect(ProcessTerminationWaiter.fallbackWaitQoSClass == .userInitiated)
    }

    @Test("A terminal close before process exit keeps the reader alive for late output")
    func terminalCloseBeforeExitKeepsReaderAlive() {
        var drain = PTYReadDrainState()

        #expect(drain.observe(.terminalClosed, stopRequested: false, now: 10) == .wait)
        #expect(drain.observe(.terminalClosed, stopRequested: true, now: 10.01) == .wait)
        #expect(drain.observe(.bytes(18), stopRequested: true, now: 10.02) == .keepReading)
        #expect(drain.observe(.terminalClosed, stopRequested: true, now: 10.03) == .wait)
        #expect(drain.observe(.terminalClosed, stopRequested: true, now: 10.14) == .finish)
    }

    @Test("The stop drain deadline also bounds a continuous byte stream")
    func stopDrainDeadlineBoundsBytes() {
        var drain = PTYReadDrainState()

        #expect(drain.observe(.noData, stopRequested: true, now: 20) == .wait)
        #expect(drain.observe(.bytes(1), stopRequested: true, now: 20.74) == .keepReading)
        #expect(drain.observe(.bytes(1), stopRequested: true, now: 20.76) == .finish)
    }

    @Test("Process exit starts PTY draining before exit waiters resume")
    func processExitStartsPTYDrainBeforeWaitersResume() async throws {
        let exitActionStarted = DispatchSemaphore(value: 0)
        let allowExitAction = DispatchSemaphore(value: 0)
        let valueReturned = LockedFlag()
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/true")
        let waiter = ProcessTerminationWaiter {
            exitActionStarted.signal()
            allowExitAction.wait()
        }
        waiter.attach(to: process)

        try process.run()
        waiter.beginFallbackWait(for: process)
        let didStartExitAction = await waitForSignal(exitActionStarted, timeout: 2)
        #expect(didStartExitAction)
        guard didStartExitAction else {
            allowExitAction.signal()
            return
        }

        let waitTask = Task {
            _ = await waiter.value()
            valueReturned.set()
        }
        try await Task.sleep(for: .milliseconds(50))
        #expect(!valueReturned.value)

        allowExitAction.signal()
        _ = await waitTask.value
        #expect(valueReturned.value)
    }

    @Test("A delayed reader preserves output from a child that already exited")
    func delayedReaderPreservesFastOutput() async throws {
        let runner = PTYCommandRunner {
            usleep(100_000)
        }
        let result = try await runner.run(CommandRequest(
            executableURL: URL(fileURLWithPath: "/bin/echo"),
            arguments: ["fast-output"],
            inputLines: [],
            timeout: 2
        ))

        #expect(result.exitCode == 0)
        #expect(result.output.contains("fast-output"))
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
            traceURL: directory.appendingPathComponent("phases"))
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
            #expect(phases.contains { $0.hasPrefix("shell_started ") })
            #expect(phases.contains { $0.hasPrefix("child_spawned ") })
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

    @Test("Captures output after the child closes and reopens its standard streams")
    func capturesOutputAfterChildStreamsReopen() async throws {
        let result = try await PTYCommandRunner().run(CommandRequest(
            executableURL: reopenedTailExecutable,
            inputLines: [],
            timeout: 2
        ))

        #expect(result.exitCode == 0)
        #expect(result.output.contains("reopened-tail-output"))
    }

    @Test("Concurrent terminal commands do not lose their output")
    func concurrentCommandsPreserveOutput() async throws {
        let results = try await withThrowingTaskGroup(of: (Int, CommandResult).self) { group in
            for index in 0..<32 {
                group.addTask {
                    let result = try await PTYCommandRunner().run(CommandRequest(
                        executableURL: fixtureExecutable,
                        inputLines: ["concurrent"],
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
        let missingOutput = results
            .filter { !$0.1.output.contains("concurrent-output") }
            .sorted { $0.0 < $1.0 }
        if !missingOutput.isEmpty {
            let diagnostic = missingOutput.map {
                "index=\($0.0) exit=\($0.1.exitCode) bytes=\($0.1.output.utf8.count)"
            }.joined(separator: "; ")
            print("PTY concurrent output diagnostics: \(diagnostic)")
        }
        #expect(missingOutput.isEmpty)
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

    private var reopenedTailExecutable: URL {
        Bundle.module.url(forResource: "fake-reopened-tail", withExtension: "sh")!
    }
}

private final class LockedFlag: @unchecked Sendable {
    private let lock = NSLock()
    private var isSet = false

    var value: Bool { lock.withLock { isSet } }

    func set() {
        lock.withLock { isSet = true }
    }
}

private func waitForSignal(_ semaphore: DispatchSemaphore, timeout: TimeInterval) async -> Bool {
    await withCheckedContinuation { continuation in
        DispatchQueue.global().async {
            continuation.resume(returning: semaphore.wait(timeout: .now() + timeout) == .success)
        }
    }
}

/// Test-only metadata: no command text, environment, paths, or captured CLI output.
private final class ParentExitDiagnostics: @unchecked Sendable {
    let traceURL: URL
    private let monotonicStart = ProcessInfo.processInfo.systemUptime
    private let lock = NSLock()
    private var registrationElapsed: TimeInterval?

    init(traceURL: URL) {
        self.traceURL = traceURL
    }

    func recordRegistration() {
        lock.withLock { registrationElapsed = ProcessInfo.processInfo.systemUptime - monotonicStart }
    }

    func phases() -> [String] {
        let trace = (try? String(contentsOf: traceURL, encoding: .utf8)) ?? ""
        return Self.sanitizedPhases(trace)
    }

    func summary(outcome: String) -> String {
        let registration = lock.withLock { registrationElapsed.map { String(format: "%.3fs", $0) } ?? "not_observed" }
        let elapsed = String(format: "%.3fs", ProcessInfo.processInfo.systemUptime - monotonicStart)
        return "PTY parent-exit diagnostics: deadline=2.000s outcome=\(outcome) "
            + "runner_elapsed=\(elapsed) before_registration=\(registration) "
            + "fixture_phases=[\(phases().joined(separator: "; "))]"
    }

    static func sanitizedPhases(_ trace: String) -> [String] {
        let allowed = Set(["shell_started", "parent_output_flushed", "child_spawned",
                           "parent_exit_requested"])
        return trace.split(separator: "\n").prefix(32).compactMap { line in
            let fields = line.split(separator: "|", omittingEmptySubsequences: false)
            guard fields.count == 2, allowed.contains(String(fields[0])),
                  let pid = Int32(fields[1]), pid > 0 else { return nil }
            return "\(fields[0]) pid=\(pid)"
        }
    }
}
