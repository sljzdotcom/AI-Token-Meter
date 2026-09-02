import Foundation
import Testing
@testable import AIMeterCore

@Suite("Claude account reader")
struct ClaudeAccountReaderTests {
    @Test("Connected Claude status keeps only display identity")
    func connectedIdentity() throws {
        let status = try ClaudeAccountStatusParser().parse(
            #"noise {"loggedIn":true,"email":"m@example.com","authMethod":"oauth","subscriptionType":"pro"} trailing"#,
            checkedAt: Date(timeIntervalSince1970: 123)
        )

        #expect(status.provider == .claude)
        #expect(status.connectionState == .connected)
        #expect(status.accountLabel == "m@example.com")
        #expect(status.accountDetail == "OAuth · Pro")
        #expect(status.checkedAt == Date(timeIntervalSince1970: 123))
    }

    @Test("A connected account without email falls back to the authentication method")
    func connectedWithoutEmail() throws {
        let status = try ClaudeAccountStatusParser().parse(
            #"{"loggedIn":true,"authMethod":"claude.ai"}"#
        )

        #expect(status.connectionState == .connected)
        #expect(status.accountLabel == "Claude Code account")
        #expect(status.accountDetail == nil)
    }

    @Test("Logged-out Claude is distinct from an unavailable probe")
    func loggedOut() throws {
        let status = try ClaudeAccountStatusParser().parse(
            #"{"loggedIn":false,"authMethod":"none"}"#
        )

        #expect(status.connectionState == .signInRequired)
        #expect(status.accountLabel == nil)
        #expect(status.accountDetail == nil)
    }

    @Test("The reader reports a missing Claude executable")
    func notInstalled() async {
        let reader = ClaudeAccountReader(
            runner: StubAccountCommandRunner(result: .success(.init(output: "", exitCode: 0, duration: 0))),
            locator: MissingAccountExecutableLocator()
        )

        let status = await reader.read()

        #expect(status.connectionState == .notInstalled)
    }

    @Test("The reader uses the bounded official auth status command")
    func officialStatusCommand() async {
        let runner = RecordingAccountCommandRunner(output: #"{"loggedIn":true,"authMethod":"oauth"}"#)
        let reader = ClaudeAccountReader(
            runner: runner,
            locator: FixedAccountExecutableLocator(url: URL(fileURLWithPath: "/tmp/claude"))
        )

        let status = await reader.read()
        let requests = await runner.requests

        #expect(status.connectionState == .connected)
        #expect(requests.count == 1)
        #expect(requests.first?.arguments == ["auth", "status", "--json"])
        #expect(requests.first?.timeout == 5)
        #expect(requests.first?.inputLines.isEmpty == true)
    }

    @Test("A failed or malformed probe is unavailable, not signed out")
    func malformedProbe() async {
        let reader = ClaudeAccountReader(
            runner: StubAccountCommandRunner(result: .success(.init(output: "not json", exitCode: 1, duration: 0))),
            locator: FixedAccountExecutableLocator(url: URL(fileURLWithPath: "/tmp/claude"))
        )

        let status = await reader.read()

        #expect(status.connectionState == .unavailable)
        #expect(status.accountLabel == nil)
    }
}

private struct FixedAccountExecutableLocator: ExecutableLocating {
    let url: URL?
    func locate(named name: String) -> URL? { url }
}

private struct MissingAccountExecutableLocator: ExecutableLocating {
    func locate(named name: String) -> URL? { nil }
}

private struct StubAccountCommandRunner: CommandRunning {
    let result: Result<CommandResult, Error>
    func run(_ request: CommandRequest) async throws -> CommandResult { try result.get() }
}

private actor RecordingAccountCommandRunner: CommandRunning {
    let output: String
    private(set) var requests: [CommandRequest] = []

    init(output: String) { self.output = output }

    func run(_ request: CommandRequest) async throws -> CommandResult {
        requests.append(request)
        return CommandResult(output: output, exitCode: 0, duration: 0)
    }
}
