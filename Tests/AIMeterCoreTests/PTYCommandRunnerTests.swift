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

    private var fixtureExecutable: URL {
        Bundle.module.url(forResource: "fake-interactive-cli", withExtension: "sh")!
    }
}

