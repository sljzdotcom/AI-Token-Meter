import Foundation
import Testing
@testable import AIMeterCore

@Suite("Bounded command runner", .serialized)
struct BoundedCommandRunnerTests {
    @Test func usesPipesAndCapturesStandardOutputAndError() async throws {
        let result = try await BoundedCommandRunner().run(CommandRequest(
            executableURL: URL(fileURLWithPath: "/bin/sh"),
            arguments: ["-c", "if [ -t 1 ]; then echo tty; else echo pipe; fi; echo diagnostic >&2"],
            inputLines: [],
            timeout: 2
        ))

        #expect(result.exitCode == 0)
        #expect(result.output.contains("pipe"))
        #expect(result.output.contains("diagnostic"))
        #expect(!result.output.contains("tty"))
    }

    @Test func rejectsOutputBeyondTheConfiguredLimit() async {
        await #expect(throws: UsageCollectionError.unrecognizedOutput) {
            try await BoundedCommandRunner().run(CommandRequest(
                executableURL: URL(fileURLWithPath: "/usr/bin/yes"),
                inputLines: [],
                timeout: 2,
                maxOutputBytes: 32
            ))
        }
    }

    @Test func terminatesAfterTheConfiguredDeadline() async {
        let startedAt = Date()
        await #expect(throws: UsageCollectionError.timedOut) {
            try await BoundedCommandRunner().run(CommandRequest(
                executableURL: URL(fileURLWithPath: "/bin/sleep"),
                arguments: ["2"],
                inputLines: [],
                timeout: 0.05
            ))
        }
        #expect(Date().timeIntervalSince(startedAt) < 1)
    }

    @Test func cancellationTerminatesTheCommand() async {
        let clock = ContinuousClock()
        let startedSignal = FileManager.default.temporaryDirectory
            .appending(path: "bounded-command-started-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: startedSignal) }
        let task = Task {
            try await BoundedCommandRunner().run(CommandRequest(
                executableURL: URL(fileURLWithPath: "/bin/sh"),
                arguments: [
                    "-c", "printf started > \"$1\"; exec /bin/sleep 30",
                    "bounded-command-cancellation", startedSignal.path,
                ],
                inputLines: [],
                timeout: 10
            ))
        }
        var didStart = false
        for _ in 0..<500 {
            if FileManager.default.fileExists(atPath: startedSignal.path) {
                didStart = true
                break
            }
            try? await Task.sleep(for: .milliseconds(10))
        }
        guard didStart else {
            task.cancel()
            _ = try? await task.value
            Issue.record("The controlled child process did not start")
            return
        }
        // A long pre-cancel delay proves the one-second assertion measures cleanup,
        // while the 30-second child cannot naturally exit inside that threshold.
        try? await Task.sleep(for: .milliseconds(1_100))
        let cancelledAt = clock.now
        task.cancel()
        await #expect(throws: CancellationError.self) { try await task.value }
        #expect(cancelledAt.duration(to: clock.now) < .seconds(1))
    }
}
