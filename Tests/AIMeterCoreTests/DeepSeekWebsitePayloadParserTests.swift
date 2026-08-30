import Foundation
import Testing
@testable import AIMeterCore

@Suite("DeepSeek website payload parser")
struct DeepSeekWebsitePayloadParserTests {
    @Test("Extracts daily usage from nested official page JSON")
    func extractsNestedRows() throws {
        let data = #"{"data":{"daily_usage":[{"date":"2026-08-29","cost":"2.50","request_count":10,"input_tokens":700,"output_tokens":300},{"usage_date":"2026-08-30","total_cost":1.25,"requests":"5","total_tokens":"500"}]}}"#.data(using: .utf8)!

        let rows = try DeepSeekWebsitePayloadParser().parse(data)

        #expect(rows.count == 2)
        #expect(rows[0].costCNY == 2.5)
        #expect(rows[0].requestCount == 10)
        #expect(rows[0].tokenCount == 1_000)
        #expect(rows[1].costCNY == 1.25)
        #expect(rows[1].requestCount == 5)
        #expect(rows[1].tokenCount == 500)
    }

    @Test("Aggregates current by-API-key amount buckets without retaining key identity")
    func extractsCurrentAmountBuckets() throws {
        let data = #"{"code":0,"data":{"biz_code":0,"biz_data":{"series":[{"api_key":{"tracking_id":"secret","name":"Primary"},"model":"deepseek-v4-flash","buckets":[{"time":1788105600,"usage":{"PROMPT_CACHE_HIT_TOKEN":"700","PROMPT_CACHE_MISS_TOKEN":"200","RESPONSE_TOKEN":"100","REQUEST":"3"}}]},{"api_key":{"tracking_id":"other"},"model":"deepseek-v4-pro","buckets":[{"time":1788105600,"usage":{"PROMPT_CACHE_HIT_TOKEN":"50","PROMPT_CACHE_MISS_TOKEN":"25","RESPONSE_TOKEN":"25","REQUEST":"2"}}]}]}}}"#.data(using: .utf8)!

        let rows = try DeepSeekWebsitePayloadParser().parse(data)

        #expect(rows.count == 1)
        #expect(rows[0].costCNY == 0)
        #expect(rows[0].requestCount == 5)
        #expect(rows[0].tokenCount == 1_100)
    }

    @Test("Aggregates current by-API-key cost buckets")
    func extractsCurrentCostBuckets() throws {
        let data = #"{"code":0,"data":{"biz_code":0,"biz_data":{"data":[{"currency":"CNY","series":[{"model":"deepseek-v4-flash","buckets":[{"time":1788105600,"cost":"1.25"}]},{"model":"deepseek-v4-pro","buckets":[{"time":1788105600,"cost":"0.75"}]}]}]}}}"#.data(using: .utf8)!

        let rows = try DeepSeekWebsitePayloadParser().parse(data)

        #expect(rows.count == 1)
        #expect(rows[0].costCNY == 2)
        #expect(rows[0].requestCount == 0)
        #expect(rows[0].tokenCount == 0)
    }

    @Test("Merges amount and cost facets only when both are present")
    func mergesResponseFacets() throws {
        let date = Date(timeIntervalSince1970: 1_788_105_600)
        var accumulator = DeepSeekUsageFacetAccumulator()

        #expect(accumulator.replace(
            .amount,
            rows: [DeepSeekDailyUsage(date: date, costCNY: 0, requestCount: 5, tokenCount: 1_100)]
        ) == nil)
        let completed = accumulator.replace(
            .cost,
            rows: [DeepSeekDailyUsage(date: date, costCNY: 2, requestCount: 0, tokenCount: 0)]
        )
        let merged = try #require(completed)

        #expect(merged.count == 1)
        #expect(merged[0].costCNY == 2)
        #expect(merged[0].requestCount == 5)
        #expect(merged[0].tokenCount == 1_100)
    }

    @Test("Rejects unrelated and oversized payloads")
    func rejectsUnsafePayloads() throws {
        #expect(throws: DeepSeekWebsitePayloadParser.Error.self) {
            try DeepSeekWebsitePayloadParser().parse(#"{"user":{"email":"private@example.com"}}"#.data(using: .utf8)!)
        }
        #expect(throws: DeepSeekWebsitePayloadParser.Error.self) {
            try DeepSeekWebsitePayloadParser(maxPayloadBytes: 10).parse(Data(repeating: 0, count: 11))
        }
    }
}
