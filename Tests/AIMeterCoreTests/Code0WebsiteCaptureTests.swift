import Foundation
import Testing
@testable import AIMeterCore

@Suite("Code0 filtered website quota")
struct Code0WebsiteCaptureTests {
    @Test("Filtered quota converts to site-credit balance and cumulative usage without a ratio")
    func filteredQuotaConvertsToSiteCreditsWithoutInventingLimit() throws {
        let capture = try Code0WebsiteCapture.decode(
            filteredPayload: Data(#"{"quota":1250000,"usedQuota":750000,"divisor":500000}"#.utf8)
        )
        #expect(capture.balance == 2.5)
        #expect(capture.cumulativeUsage == 1.5)
        let fetchedAt = Date(timeIntervalSince1970: 1_793_016_000)
        let snapshot = try capture.makeSnapshot(fetchedAt: fetchedAt)
        #expect(snapshot.provider.rawValue == "code0")
        #expect(snapshot.provider.displayName == "Code0")
        #expect(snapshot.collectionStatus == .fresh)
        #expect(snapshot.fetchedAt == fetchedAt)
        let balance = try #require(snapshot.primaryMetric)
        let used = try #require(snapshot.secondaryMetric)
        #expect(balance.label == "Balance")
        #expect(balance.current == 2.5)
        #expect(balance.unit.rawValue == "siteCredits")
        #expect(balance.kind.rawValue == "balance")
        #expect(used.label == "Cumulative usage")
        #expect(used.current == 1.5)
        #expect(used.unit.rawValue == "siteCredits")
        #expect(used.kind.rawValue == "cumulativeUsage")
        for metric in [balance, used] {
            #expect(metric.limit == nil)
            #expect(metric.resetAt == nil)
            #expect(metric.resetDescription == nil)
            #expect(metric.usedFraction == nil)
        }
        let presentation = ProviderPresentation(snapshot: snapshot)
        #expect(presentation.fraction == nil)
        #expect(!presentation.valueText.contains("¥"))
        #expect(!presentation.valueText.contains("$"))
        #expect(!presentation.valueText.contains("%"))
    }

    @Test("Zero is real quota data and a positive fractional divisor is not replaced with a default")
    func zeroAndFractionalDivisorAreValid() throws {
        let zero = try Code0WebsiteCapture.decode(filteredPayload: Data(#"{"quota":0,"usedQuota":0,"divisor":500000}"#.utf8))
        #expect(zero.balance == 0)
        #expect(zero.cumulativeUsage == 0)
        let fractional = try Code0WebsiteCapture.decode(filteredPayload: Data(#"{"quota":1,"usedQuota":2,"divisor":0.5}"#.utf8))
        #expect(fractional.balance == 2)
        #expect(fractional.cumulativeUsage == 4)
        let upperBound = try Code0WebsiteCapture.decode(filteredPayload: Data(#"{"quota":9007199254740991,"usedQuota":9007199254740991,"divisor":9007199254740991}"#.utf8))
        #expect(upperBound.balance == 1)
        #expect(upperBound.cumulativeUsage == 1)
        let scientific = try Code0WebsiteCapture.decode(filteredPayload: Data(#"{"quota":1.25e6,"usedQuota":750000.0,"divisor":5e5}"#.utf8))
        #expect(scientific.balance == 2.5)
        #expect(scientific.cumulativeUsage == 1.5)
    }

    @Test("Filtered payload rejects missing, coerced, unsafe and extra fields", arguments: [
        #"{"quota":1,"usedQuota":2}"#,
        #"{"usedQuota":2,"divisor":1}"#,
        #"{"quota":1,"divisor":1}"#,
        #"{"quota":"1","usedQuota":2,"divisor":1}"#,
        #"{"quota":1,"usedQuota":"2","divisor":1}"#,
        #"{"quota":1,"usedQuota":2,"divisor":"1"}"#,
        #"{"quota":true,"usedQuota":2,"divisor":1}"#,
        #"{"quota":1,"usedQuota":false,"divisor":1}"#,
        #"{"quota":1,"usedQuota":2,"divisor":true}"#,
        #"{"quota":null,"usedQuota":2,"divisor":1}"#,
        #"{"quota":-1,"usedQuota":2,"divisor":1}"#,
        #"{"quota":1,"usedQuota":-2,"divisor":1}"#,
        #"{"quota":1,"usedQuota":2,"divisor":0}"#,
        #"{"quota":1,"usedQuota":2,"divisor":-1}"#,
        #"{"quota":1.5,"usedQuota":2,"divisor":1}"#,
        #"{"quota":1,"usedQuota":2.5,"divisor":1}"#,
        #"{"quota":9007199254740992,"usedQuota":2,"divisor":1}"#,
        #"{"quota":1,"usedQuota":9007199254740992,"divisor":1}"#,
        #"{"quota":1,"usedQuota":2,"divisor":9007199254740992}"#,
        #"{"quota":1e309,"usedQuota":2,"divisor":1}"#,
        #"{"quota":1,"usedQuota":2,"divisor":NaN}"#,
        #"{"quota":1,"usedQuota":2,"divisor":1e-320}"#,
        #"{"quota":1,"usedQuota":2,"divisor":1,"identity":"synthetic-do-not-export"}"#,
        #"{"quota":1,"usedQuota":2,"divisor":1,"data":{}}"#,
        #"{"quota":1,"quota":3,"usedQuota":2,"divisor":1}"#,
        #"{"quota":9007199254740991.1,"usedQuota":0,"divisor":1}"#,
        #"{"quota":0,"usedQuota":9007199254740991.1,"divisor":1}"#,
        #"{"quota":1,"usedQuota":2,"divisor":1} trailing"#,
        #"{"quota":01,"usedQuota":2,"divisor":1}"#,
        #"{"quota ":1,"usedQuota":2,"divisor":1}"#,
        #"{"quota":1,"usedQuota":2,"divisor":9007199254740991.1}"#,
    ])
    func rejectsUnfilteredOrInvalidQuota(_ payload: String) {
        #expect(throws: Code0WebsiteCaptureError.invalidPayload) {
            try Code0WebsiteCapture.decode(filteredPayload: Data(payload.utf8))
        }
    }

    @Test func rejectsOversizedFilteredPayloadAndInvalidNativeTimestamp() throws {
        #expect(throws: Code0WebsiteCaptureError.invalidPayload) {
            try Code0WebsiteCapture.decode(filteredPayload: Data(repeating: 32, count: 2049))
        }
        let capture = try Code0WebsiteCapture.decode(filteredPayload: Data(#"{"quota":1,"usedQuota":2,"divisor":1}"#.utf8))
        #expect(throws: Code0WebsiteCaptureError.invalidPayload) {
            try capture.makeSnapshot(fetchedAt: Date(timeIntervalSince1970: .infinity))
        }
    }

    @Test("Code0 identity and semantic enums decode through existing raw and JSON entry points")
    func code0IdentityAndMetricSemanticsUseStableRawValues() throws {
        let provider = try #require(UsageProvider(rawValue: "code0"))
        #expect(provider.displayName == "Code0")
        #expect(try JSONDecoder().decode(UsageProvider.self, from: Data(#""code0""#.utf8)) == provider)
        #expect(try JSONDecoder().decode(UsageUnit.self, from: Data(#""siteCredits""#.utf8)).rawValue == "siteCredits")
        #expect(try JSONDecoder().decode(UsageMetricKind.self, from: Data(#""cumulativeUsage""#.utf8)).rawValue == "cumulativeUsage")
        #expect(UsageProvider.deepSeek.rawValue == "deepSeek")
        #expect(try JSONDecoder().decode(UsageProvider.self, from: Data(#""deepSeek""#.utf8)) == .deepSeek)
    }

    @Test("The dedicated seventh shared fixture carries only normalized site-credit metrics")
    func dedicatedCode0FixtureHasNoCurrencyRatioOrAccountData() throws {
        let fixtureURL = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("contracts/fixtures/code0-site-credits.json")
        let data = try Data(contentsOf: fixtureURL)
        let fixture = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])
        let providerID = try #require(fixture["providerId"] as? String)
        let provider = try #require(UsageProvider(rawValue: providerID))
        #expect(provider.displayName == "Code0")
        #expect(fixture["usedRatio"] is NSNull)
        let primary = try #require(fixture["primaryMetric"] as? [String: Any])
        let secondary = try #require(fixture["secondaryMetric"] as? [String: Any])
        #expect(primary["current"] as? Double == 2.5)
        #expect(secondary["current"] as? Double == 1.5)
        #expect(primary["unit"] as? String == UsageUnit(rawValue: "siteCredits")?.rawValue)
        #expect(secondary["kind"] as? String == UsageMetricKind(rawValue: "cumulativeUsage")?.rawValue)
        #expect(Set(fixture.keys) == ["schemaVersion", "providerId", "displayName", "status", "usedRatio", "primaryMetric", "secondaryMetric", "fetchedAt", "staleAfterSeconds", "sourceVersion"])
    }
}
