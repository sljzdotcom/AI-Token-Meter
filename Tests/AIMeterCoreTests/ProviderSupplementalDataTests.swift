import Foundation
import Testing
@testable import AIMeterCore

@Suite("Provider supplemental data")
struct ProviderSupplementalDataTests {
    @Test("Codex reset credits retain display data without redeem identifiers")
    func codexCreditDisplayModel() throws {
        let expiry = Date(timeIntervalSince1970: 1_900_000_000)
        let summary = CodexResetCreditsSummary(
            availableCount: 2,
            credits: [CodexResetCreditDisplay(title: "Usage reset", expiresAt: expiry)],
            hasCompleteDetails: false
        )

        let data = try JSONEncoder().encode(summary)
        let json = try #require(String(data: data, encoding: .utf8))

        #expect(!json.contains("creditId"))
        #expect(try JSONDecoder().decode(CodexResetCreditsSummary.self, from: data) == summary)
    }

    @Test("Old snapshots decode without supplemental provider data")
    func oldSnapshotCompatibility() throws {
        let legacy = try #require(
            #"{"provider":"codex","primaryMetric":null,"secondaryMetric":null,"availability":"available","fetchedAt":0,"staleAfter":300,"sourceVersion":null,"collectionStatus":"fresh","statusMessage":null}"#
                .data(using: .utf8)
        )

        let snapshot = try JSONDecoder().decode(UsageSnapshot.self, from: legacy)

        #expect(snapshot.codexResetCredits == nil)
        #expect(snapshot.deepSeekUsageHistory == nil)
    }

    @Test("DeepSeek history exposes independently derived totals")
    func deepSeekHistoryTotals() {
        let history = DeepSeekUsageHistory(
            days: [
                DeepSeekDailyUsage(
                    date: Date(timeIntervalSince1970: 1_900_000_000),
                    costCNY: 1.25,
                    requestCount: 4,
                    tokenCount: 900
                ),
                DeepSeekDailyUsage(
                    date: Date(timeIntervalSince1970: 1_900_086_400),
                    costCNY: 2.75,
                    requestCount: 6,
                    tokenCount: 1_100
                ),
            ],
            updatedAt: Date(timeIntervalSince1970: 1_900_086_400),
            statusMessage: nil
        )

        #expect(history.totalCostCNY == 4)
        #expect(history.totalRequests == 10)
        #expect(history.totalTokens == 2_000)
    }
}
