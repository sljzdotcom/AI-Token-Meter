import Foundation
import Testing
@testable import AIMeterApp

@Suite("Antigravity one-time login receipt")
struct GeminiLoginReceiptTests {
    @Test("A valid UUID success receipt requires zero exit code")
    func acceptsSuccessfulReceipt() throws {
        let receipt = try #require(GeminiLoginReceipt(url: URL(string:
            "aitokenmeter://antigravity-login-complete?token=12345678-1234-1234-1234-123456789abc&result=success&exit_code=0"
        )!))

        #expect(receipt.token == "12345678-1234-1234-1234-123456789abc")
        #expect(receipt.result == .success)
    }

    @Test(arguments: [
        ("failure", "7", GeminiLoginResult.failure),
        ("timeout", "124", GeminiLoginResult.timedOut),
        ("cancelled", "130", GeminiLoginResult.cancelled),
    ])
    func classifiesNonSuccessReceipts(_ input: (String, String, GeminiLoginResult)) throws {
        let receipt = try #require(GeminiLoginReceipt(url: URL(string:
            "aitokenmeter://antigravity-login-complete?token=12345678-1234-1234-1234-123456789abc&result=\(input.0)&exit_code=\(input.1)"
        )!))
        #expect(receipt.result == input.2)
    }

    @Test(arguments: [
        "aitokenmeter://wrong-host?token=12345678-1234-1234-1234-123456789abc&result=success&exit_code=0",
        "https://antigravity-login-complete?token=12345678-1234-1234-1234-123456789abc&result=success&exit_code=0",
        "aitokenmeter://antigravity-login-complete?token=bad&result=success&exit_code=0",
        "aitokenmeter://antigravity-login-complete?token=12345678-1234-1234-1234-123456789abc&result=success&exit_code=7",
        "aitokenmeter://antigravity-login-complete?token=12345678-1234-1234-1234-123456789abc&result=failure&exit_code=0",
        "aitokenmeter://antigravity-login-complete?token=12345678-1234-1234-1234-123456789abc&result=other&exit_code=1",
        "aitokenmeter://antigravity-login-complete?token=12345678-1234-1234-1234-123456789abc&token=12345678-1234-1234-1234-123456789abc&result=success&exit_code=0",
    ])
    func rejectsMalformedOrAmbiguousReceipts(_ value: String) {
        #expect(GeminiLoginReceipt(url: URL(string: value)!) == nil)
    }
}
