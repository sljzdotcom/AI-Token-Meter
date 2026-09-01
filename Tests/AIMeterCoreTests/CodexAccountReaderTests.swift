import Foundation
import Testing
@testable import AIMeterCore

@Suite("Codex account reader", .serialized)
struct CodexAccountReaderTests {
    @Test("Codex app-server decodes the documented account response")
    func appServerResponse() async throws {
        let result = try await CodexAppServerClient().readAccount(executableURL: fixtureExecutable)

        guard case .chatGPT(let email, let planType) = result.account else {
            Issue.record("Expected a ChatGPT account")
            return
        }
        #expect(email == "codex@example.com")
        #expect(planType == "pro")
    }

    @Test("Codex account read exposes ChatGPT email and plan")
    func chatGPTAccount() async {
        let status = await reader().read()

        #expect(status.provider == .codex)
        #expect(status.connectionState == .connected)
        #expect(status.accountLabel == "codex@example.com")
        #expect(status.accountDetail == "ChatGPT · Pro")
    }

    @Test("An API Key account is identified without exposing the key")
    func apiKeyAccount() async {
        let status = await reader(accountKind: "api-key").read()

        #expect(status.connectionState == .connected)
        #expect(status.accountLabel == "API Key account")
        #expect(status.accountDetail == nil)
    }

    @Test("An empty Codex account requires sign in")
    func signedOutAccount() async {
        let status = await reader(accountKind: "signed-out").read()

        #expect(status.connectionState == .signInRequired)
        #expect(status.accountLabel == nil)
    }

    @Test("An invalid Codex response is unavailable, not signed out")
    func invalidResponse() async {
        let status = await reader(accountKind: "invalid").read()

        #expect(status.connectionState == .unavailable)
    }

    @Test("A missing Codex executable is reported without starting app-server")
    func notInstalled() async {
        let status = await CodexAccountReader(
            locator: CodexAccountFixedLocator(url: nil)
        ).read()

        #expect(status.connectionState == .notInstalled)
    }

    private func reader(accountKind: String = "chatgpt") -> CodexAccountReader {
        CodexAccountReader(
            locator: CodexAccountFixedLocator(url: fixtureExecutable),
            client: CodexAppServerClient(environmentOverrides: [
                "AI_METER_TEST_ACCOUNT_KIND": accountKind,
            ])
        )
    }

    private var fixtureExecutable: URL {
        Bundle.module.url(forResource: "fake-interactive-cli", withExtension: "sh")!
    }
}

private struct CodexAccountFixedLocator: ExecutableLocating {
    let url: URL?
    func locate(named name: String) -> URL? { url }
}
