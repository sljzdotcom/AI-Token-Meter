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
            runner: PTYCommandRunner(),
            locator: FixedLocator(url: fixtureExecutable)
        )

        let snapshot = try await collector.collect()

        #expect(snapshot.provider == .codex)
        #expect(snapshot.primaryMetric?.usedFraction == 0.73)
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

    private var fixtureExecutable: URL {
        Bundle.module.url(forResource: "fake-interactive-cli", withExtension: "sh")!
    }
}

private struct FixedLocator: ExecutableLocating {
    let url: URL?

    func locate(named name: String) -> URL? {
        url
    }
}

