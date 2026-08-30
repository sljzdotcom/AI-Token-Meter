import Foundation
import Testing
@testable import AIMeterCore

@Suite("Claude usage parser")
struct ClaudeUsageParserTests {
    @Test("Parses two English used metrics and their reset descriptions")
    func parsesEnglishUsage() throws {
        let snapshot = try ClaudeUsageParser().parse(fixture("claude-usage-en"))

        #expect(snapshot.provider == .claude)
        #expect(snapshot.primaryMetric?.label == "Current session")
        #expect(snapshot.primaryMetric?.usedFraction == 0.73)
        #expect(snapshot.primaryMetric?.resetDescription == "Resets in 51 min")
        #expect(snapshot.secondaryMetric?.label == "All models")
        #expect(snapshot.secondaryMetric?.usedFraction == 0.07)
        #expect(snapshot.secondaryMetric?.resetDescription == "Resets Thu 12:00 AM")
    }

    @Test("Parses Chinese used and remaining percentages")
    func parsesChineseUsage() throws {
        let snapshot = try ClaudeUsageParser().parse(fixture("claude-usage-zh"))

        #expect(snapshot.primaryMetric?.label == "当前会话")
        #expect(snapshot.primaryMetric?.usedFraction == 0.65)
        #expect(snapshot.secondaryMetric?.label == "所有模型")
        #expect(snapshot.secondaryMetric?.usedFraction == 0.80)
    }

    @Test("Ignores Claude promotional percentages")
    func ignoresPromotionalPercentages() throws {
        let snapshot = try ClaudeUsageParser().parse(fixture("claude-usage-promo-en"))

        #expect(snapshot.primaryMetric?.usedFraction == 0)
        #expect(snapshot.secondaryMetric?.usedFraction == 0)
    }

    @Test("Rejects output without a real usage metric")
    func rejectsUnrecognizedOutput() {
        #expect(throws: UsageCollectionError.unrecognizedOutput) {
            try ClaudeUsageParser().parse("Claude Code is ready")
        }
    }

    private func fixture(_ name: String) throws -> String {
        let url = try #require(Bundle.module.url(forResource: name, withExtension: "txt"))
        return try String(contentsOf: url, encoding: .utf8)
    }
}
