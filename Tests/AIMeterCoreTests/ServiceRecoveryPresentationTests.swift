import Testing
@testable import AIMeterCore

@Suite("Four-provider recovery presentation")
struct ServiceRecoveryPresentationTests {
    @Test("Every provider and collection state has the intended detail recovery")
    func recoveryMatrix() {
        let passive: [CollectionStatus] = [.fresh, .cached, .refreshing]
        let services: [CollectionStatus] = [
            .authenticationRequired,
            .setupRequired,
            .notInstalled,
            .unavailable,
            .unrecognizedOutput,
        ]

        for provider in [UsageProvider.claude, .codex, .deepSeek] {
            for status in passive {
                #expect(ServiceRecoveryPresentation.detailAction(provider: provider, status: status) == nil)
            }
            for status in services {
                let expected: ServiceRecoveryAction = provider == .claude && status == .setupRequired
                    ? .openClaudeWorkspace
                    : .openServicesSettings
                #expect(ServiceRecoveryPresentation.detailAction(provider: provider, status: status) == expected)
            }
        }

        for status in passive + services {
            #expect(
                ServiceRecoveryPresentation.detailAction(provider: .gemini, status: status)
                    == .geminiControls
            )
        }

        for provider in [UsageProvider.claude, .codex, .deepSeek] {
            #expect(
                ServiceRecoveryPresentation.detailAction(
                    provider: provider,
                    status: .cached,
                    statusMessage: "Sign in required"
                ) == .openServicesSettings
            )
        }
        #expect(
            ServiceRecoveryPresentation.detailAction(
                provider: .claude,
                status: .cached,
                statusMessage: "Approve the private usage workspace once"
            ) == .openClaudeWorkspace
        )
        #expect(
            ServiceRecoveryPresentation.detailAction(
                provider: .codex,
                status: .cached,
                statusMessage: "Request timed out"
            ) == nil
        )
    }
}
