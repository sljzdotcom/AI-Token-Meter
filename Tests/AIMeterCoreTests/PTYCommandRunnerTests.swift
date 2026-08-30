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
        let runner = PTYCommandRunner()
        let startedAt = Date()

        let result = try await runner.run(CommandRequest(
            executableURL: parentExitExecutable,
            inputLines: [],
            timeout: 2
        ))

        #expect(result.exitCode == 0)
        #expect(result.output.contains("parent-exited"))
        #expect(Date().timeIntervalSince(startedAt) < 3)
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
}
