import Foundation
import Testing
@testable import AIMeterCore

@Suite("Antigravity quota parser")
struct GeminiUsageParserTests {
    @Test func parsesAllOfficialWindowsAndConvertsRemainingToUsed() throws {
        let snapshot = try GeminiUsageParser().parse(Self.fixture)

        #expect(snapshot.provider == .gemini)
        #expect(snapshot.sourceVersion == "1.1.28")
        #expect(snapshot.geminiQuotaMetrics?.map(\.label) == [
            "Gemini · Five hour", "Gemini · Weekly",
            "Claude/GPT · Five hour", "Claude/GPT · Weekly",
        ])
        #expect(snapshot.geminiQuotaMetrics?.map(\.current) == [60, 25, 80, 20])
        #expect(snapshot.primaryMetric?.label == "Claude/GPT · Five hour")
        #expect(snapshot.primaryMetric?.current == 80)
        #expect(snapshot.secondaryMetric?.label == "Gemini · Five hour")
        #expect(snapshot.geminiQuotaMetrics?.allSatisfy {
            $0.limit == 100 && $0.unit == .percent && $0.kind == .officialLimit && $0.resetAt != nil
        } == true)
    }

    @Test func rowOrderDoesNotChangePresentationOrder() throws {
        let reversed = Self.fixture.components(separatedBy: .newlines).reversed().joined(separator: "\n")
        let snapshot = try GeminiUsageParser().parse(reversed)
        #expect(snapshot.geminiQuotaMetrics?.map(\.current) == [60, 25, 80, 20])
    }

    @Test func zeroAndFullRemainingAreValid() throws {
        let snapshot = try GeminiUsageParser().parse(Self.table(
            geminiWeekly: "0%", geminiFiveHour: "100%",
            otherWeekly: "100%", otherFiveHour: "0%"
        ))
        #expect(snapshot.geminiQuotaMetrics?.map(\.current) == [0, 100, 100, 0])
    }

    @Test(arguments: [
        "",
        "Gemini Models\tWeekly Limit Remaining\t75%\t2026-09-17T10:00:00Z",
        Self.table(geminiWeekly: "101%"),
        Self.table(geminiWeekly: "-1%"),
        Self.table(geminiWeekly: "seventy%"),
        Self.table(geminiWeekly: "75%", reset: "tomorrow"),
        Self.table(group: "Unknown models"),
        Self.table(window: "Daily Limit Remaining"),
        Self.fixture + "\nGemini Models\tWeekly Limit Remaining\t75%\t2026-09-17T10:00:00Z",
        "Select Model\nModel usage\nPro 25%",
    ])
    func refusesIncompleteAmbiguousAndLegacyOutput(_ text: String) {
        #expect(throws: UsageCollectionError.unrecognizedOutput) {
            try GeminiUsageParser().parse(text)
        }
    }

    static let fixture: String = {
        let root = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        return try! String(
            contentsOf: root.appendingPathComponent("contracts/antigravity-cli/1.1.28/usage.txt"),
            encoding: .utf8
        )
    }()

    static func table(
        geminiWeekly: String = "75%",
        geminiFiveHour: String = "40%",
        otherWeekly: String = "80%",
        otherFiveHour: String = "20%",
        reset: String = "2026-09-17T10:00:00Z",
        group: String = "Gemini Models",
        window: String = "Weekly Limit Remaining"
    ) -> String {
        [
            "\(group)\t\(window)\t\(geminiWeekly)\t\(reset)",
            "Gemini Models\tFive Hour Limit Remaining\t\(geminiFiveHour)\t2026-09-10T10:00:00Z",
            "Claude and GPT models\tWeekly Limit Remaining\t\(otherWeekly)\t2026-09-18T11:00:00Z",
            "Claude and GPT models\tFive Hour Limit Remaining\t\(otherFiveHour)\t2026-09-10T11:00:00Z",
        ].joined(separator: "\n")
    }
}
