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
