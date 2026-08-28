import Foundation
import Testing
@testable import AIMeterCore

private let cliSmokeChecksEnabled = ProcessInfo.processInfo.environment["AI_METER_RUN_CLI_SMOKE"] == "1"

@Suite("Installed CLI integration smoke checks", .serialized)
struct CLIIntegrationSmokeTests {
    @Test("Installed Claude auth status is machine-readable", .enabled(if: cliSmokeChecksEnabled))
    func installedClaudeAuthStatusIsReadable() async throws {
        let executable = try #require(ExecutableLocator().locate(named: "claude"))
        let result = try await PTYCommandRunner().run(CommandRequest(
            executableURL: executable,
            arguments: ["auth", "status"],
            inputLines: [],
            timeout: 5
        ))
        let output = ANSITextSanitizer.sanitize(result.output)
        #expect(output.range(
            of: #"\"loggedIn\"\s*:\s*(?:false|true)"#,
            options: .regularExpression
        ) != nil)
    }

    @Test("Installed Claude CLI returns a recognized account state", .enabled(if: cliSmokeChecksEnabled))
    func installedClaudeReturnsRecognizedState() async throws {
        do {
            let snapshot = try await ClaudeCollector().collect()
            #expect(snapshot.provider == .claude)
            #expect(snapshot.primaryMetric != nil)
        } catch UsageCollectionError.authenticationRequired {
            // A logged-out account is a valid, actionable collector state.
        }
    }

    @Test("Installed Codex CLI returns a usage snapshot", .enabled(if: cliSmokeChecksEnabled))
    func installedCodexReturnsSnapshot() async throws {
        let snapshot = try await CodexCollector().collect()
        #expect(snapshot.provider == .codex)
        #expect(snapshot.primaryMetric != nil)
    }
}
