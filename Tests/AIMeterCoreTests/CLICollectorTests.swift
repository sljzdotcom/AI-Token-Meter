import Foundation
import Testing
@testable import AIMeterCore

@Suite("Claude and Codex CLI collectors", .serialized)
struct CLICollectorTests {
    @Test("Claude collector runs usage and parses the resulting snapshot")
    func claudeCollectorReturnsSnapshot() async throws {
        let collector = ClaudeCollector(
            runner: PTYCommandRunner(),
            locator: FixedLocator(url: fixtureExecutable)
        )

        let snapshot = try await collector.collect()

        #expect(snapshot.provider == .claude)
        #expect(snapshot.primaryMetric?.usedFraction == 0.73)
    }

    @Test("Codex collector runs status and parses remaining quota")
    func codexCollectorReturnsSnapshot() async throws {
        let collector = CodexCollector(
            locator: FixedLocator(url: fixtureExecutable)
        )

        let snapshot = try await collector.collect()

        #expect(snapshot.provider == .codex)
        #expect(snapshot.primaryMetric?.usedFraction == 0.27)
        #expect(snapshot.secondaryMetric?.usedFraction == 0.08)
        #expect(snapshot.sourceVersion == "codex-app-server")
    }

    @Test("A missing executable is reported without starting a process")
    func missingExecutableIsReported() async {
        let collector = ClaudeCollector(
            runner: PTYCommandRunner(),
            locator: FixedLocator(url: nil)
        )

        await #expect(throws: UsageCollectionError.notInstalled) {
            try await collector.collect()
        }
    }

    @Test("Claude authentication preflight reports a logged-out account")
    func claudeLoggedOutIsReportedBeforeInteractiveUsage() async {
        let collector = ClaudeCollector(
            runner: PTYCommandRunner(),
            locator: FixedLocator(url: loggedOutClaudeExecutable)
        )

        await #expect(throws: UsageCollectionError.authenticationRequired) {
            try await collector.collect()
        }
    }

    @Test("Codex app server timeout kills a process that ignores termination")
    func codexTimeoutIsBounded() async {
        let startedAt = Date()

        await #expect(throws: UsageCollectionError.timedOut) {
            try await CodexAppServerClient().readRateLimits(
                executableURL: ignoredTerminationCodexExecutable,
                timeout: 0.1
            )
        }

        #expect(Date().timeIntervalSince(startedAt) < 1.5)
    }

    private var fixtureExecutable: URL {
        Bundle.module.url(forResource: "fake-interactive-cli", withExtension: "sh")!
    }

    private var loggedOutClaudeExecutable: URL {
        Bundle.module.url(forResource: "fake-logged-out-claude", withExtension: "sh")!
    }

    private var ignoredTerminationCodexExecutable: URL {
        Bundle.module.url(forResource: "fake-codex-ignore-term", withExtension: "sh")!
    }
}

private struct FixedLocator: ExecutableLocating {
    let url: URL?

    func locate(named name: String) -> URL? {
        url
    }
}
