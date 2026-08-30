import Foundation
import Testing
@testable import AIMeterCore

@Suite("Codex usage parser")
struct CodexUsageParserTests {
    @Test("Converts remaining percentages into used fractions")
    func parsesRemainingAsUsedFraction() throws {
        let snapshot = try CodexUsageParser().parse(fixture("codex-status-en"))

        #expect(snapshot.provider == .codex)
        #expect(snapshot.primaryMetric?.label == "5h limit")
        #expect(snapshot.primaryMetric?.usedFraction == 0.73)
        #expect(snapshot.primaryMetric?.resetDescription == "Resets 10:30 PM")
        #expect(snapshot.secondaryMetric?.label == "Weekly limit")
        #expect(snapshot.secondaryMetric?.usedFraction == 0.08)
    }

    @Test("Accepts left as a synonym for remaining")
    func parsesLeftAsRemaining() throws {
        let snapshot = try CodexUsageParser().parse("5h limit: 21% left (resets 11:00 PM)")

        #expect(snapshot.primaryMetric?.usedFraction == 0.79)
        #expect(snapshot.primaryMetric?.resetDescription == "resets 11:00 PM")
    }

    @Test("Rejects a status screen without quota data")
    func rejectsUnrecognizedOutput() {
        #expect(throws: UsageCollectionError.unrecognizedOutput) {
            try CodexUsageParser().parse("Model: gpt-5\nDirectory: /tmp")
        }
    }

    private func fixture(_ name: String) throws -> String {
        let url = try #require(Bundle.module.url(forResource: name, withExtension: "txt"))
        return try String(contentsOf: url, encoding: .utf8)
    }
}
