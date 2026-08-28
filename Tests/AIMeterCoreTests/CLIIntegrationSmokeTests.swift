import Foundation
import Testing
@testable import AIMeterCore

private let cliSmokeChecksEnabled = ProcessInfo.processInfo.environment["AI_METER_RUN_CLI_SMOKE"] == "1"

@Suite("Installed CLI integration smoke checks", .serialized)
struct CLIIntegrationSmokeTests {
    @Test("Installed Claude CLI returns a usage snapshot", .enabled(if: cliSmokeChecksEnabled))
    func installedClaudeReturnsSnapshot() async throws {
        let snapshot = try await ClaudeCollector().collect()
        #expect(snapshot.provider == .claude)
        #expect(snapshot.primaryMetric != nil)
    }

    @Test("Installed Codex CLI returns a usage snapshot", .enabled(if: cliSmokeChecksEnabled))
    func installedCodexReturnsSnapshot() async throws {
        let snapshot = try await CodexCollector().collect()
        #expect(snapshot.provider == .codex)
        #expect(snapshot.primaryMetric != nil)
    }
}
