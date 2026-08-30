import Testing
@testable import AIMeterCore

@Suite("Floating detail interaction policy")
struct FloatingDetailInteractionPolicyTests {
    @Test("DeepSeek detail activates and focuses its web content")
    func deepSeekIsInteractive() {
        let policy = FloatingDetailInteractionPolicy(provider: .deepSeek)

        #expect(policy.activatesApplication)
        #expect(policy.requestsWebFirstResponder)
    }

    @Test(
        "Read-only provider details stay passive",
        arguments: [UsageProvider.claude, .codex]
    )
    func readOnlyProvidersStayPassive(_ provider: UsageProvider) {
        let policy = FloatingDetailInteractionPolicy(provider: provider)

        #expect(!policy.activatesApplication)
        #expect(!policy.requestsWebFirstResponder)
    }
}
