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
        let startedAt = Date()
        let task = Task {
            try await BoundedCommandRunner().run(CommandRequest(
                executableURL: URL(fileURLWithPath: "/bin/sleep"),
                arguments: ["2"],
                inputLines: [],
                timeout: 5
            ))
        }
        try? await Task.sleep(for: .milliseconds(50))
        task.cancel()
        await #expect(throws: CancellationError.self) { try await task.value }
        #expect(Date().timeIntervalSince(startedAt) < 1)
    }
}
