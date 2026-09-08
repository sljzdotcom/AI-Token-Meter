import Foundation
import Testing
@testable import AIMeterCore

@Suite("Gemini quota parser")
struct GeminiUsageParserTests {
    // Dropping a tier, reversing used/remaining, or parsing the footer must fail these literals.
    @Test func preservesAllVisibleTiersAndRanksUsedPercentage() throws {
        let snapshot = try GeminiUsageParser().parse(Self.frame("Pro ▬ 25% Resets: 5:47 PM (1h)\nFlash ▬ 60% Resets: 5:47 PM (1h)\nFlash Lite ▬ 60%"))
        #expect(snapshot.primaryMetric?.label == "Flash")
        #expect(snapshot.primaryMetric?.current == 60)
        #expect(snapshot.secondaryMetric?.label == "Flash Lite")
        #expect(snapshot.geminiQuotaMetrics?.map(\.label) == ["Pro", "Flash", "Flash Lite"])
        #expect(snapshot.geminiQuotaMetrics?.map(\.current) == [25, 60, 60])
        #expect(snapshot.geminiQuotaMetrics?.first?.resetDescription == "Resets: 5:47 PM (1h)")
        #expect(snapshot.geminiQuotaMetrics?.allSatisfy { $0.resetAt == nil && $0.limit == 100 && $0.kind == .officialLimit } == true)
        #expect(snapshot.sourceVersion == "0.58.0")
    }
    @Test(arguments: ["", "43% used", "Select Model\nModel usage\nPro 25%", frame("Pro 101%"), frame("Pro -1%"), frame("Pro 2.5%"), frame("Unknown 25%"), frame("Pro 20%\nPro 25%"), frame("Pro remaining 25%")])
    func refusesUnknownIncompleteAndContradictoryOutput(_ text: String) {
        #expect(throws: UsageCollectionError.unrecognizedOutput) { try GeminiUsageParser().parse(text) }
    }
    @Test func emptyQuotaIsUnavailable() {
        #expect(throws: UsageCollectionError.geminiUnavailable("Gemini CLI did not provide quota")) {
            try GeminiUsageParser().parse("╭────╮\nSelect Model\n(Press Esc to close)\n╰────╯")
        }
    }
    @Test func zeroAndFullUsageAreValidWithoutReset() throws {
        let snapshot = try GeminiUsageParser().parse(Self.frame("Pro 0%\nFlash 100%"))
        #expect(snapshot.primaryMetric?.current == 100)
        #expect(snapshot.geminiQuotaMetrics?.first?.current == 0)
        #expect(snapshot.primaryMetric?.resetDescription == nil)
    }
    static func frame(_ rows: String) -> String {
        "43% used\n╭────────────────────────╮\n│ Select Model │\n│ Model usage │\n\(rows)\n│ (Press Esc to close) │\n╰────────────────────────╯"
    }
}
