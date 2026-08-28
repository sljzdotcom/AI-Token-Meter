import Foundation
import Testing
@testable import AIMeterCore

@Suite("PTY command runner", .serialized)
struct PTYCommandRunnerTests {
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

    private var fixtureExecutable: URL {
        Bundle.module.url(forResource: "fake-interactive-cli", withExtension: "sh")!
    }

    private var terminalSizeExecutable: URL {
        Bundle.module.url(forResource: "fake-terminal-size", withExtension: "sh")!
    }

    private var descendantHangExecutable: URL {
        Bundle.module.url(forResource: "fake-descendant-hang", withExtension: "sh")!
    }
}
