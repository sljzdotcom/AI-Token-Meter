import Foundation
import Testing
@testable import AIMeterCore

@Suite("Service account status")
struct ServiceAccountStatusTests {
    @Test("A checking status never exposes an account identity")
    func checkingStatusHasNoIdentity() {
        let status = ServiceAccountStatus.checking(provider: .claude)

        #expect(status.provider == .claude)
        #expect(status.connectionState == .checking)
        #expect(status.accountLabel == nil)
        #expect(status.accountDetail == nil)
        #expect(status.checkedAt == nil)
    }

    @Test("The coordinator reads all three providers without persisting identity")
    func coordinatorReadsAllProviders() async {
        let coordinator = ServiceAccountCoordinator(
            claudeReader: FixedServiceAccountReader(status: .init(
                provider: .claude,
                connectionState: .connected,
                accountLabel: "claude@example.com"
            )),
            codexReader: FixedServiceAccountReader(status: .init(
                provider: .codex,
                connectionState: .signInRequired
            )),
            deepSeekReader: FixedServiceAccountReader(status: .init(
                provider: .deepSeek,
                connectionState: .connected,
                accountLabel: "API Key ••••ABCD"
            ))
        )

        let statuses = await coordinator.readAll()

        #expect(statuses.map(\.provider) == [.claude, .codex, .deepSeek])
        #expect(await coordinator.read(.codex).connectionState == .signInRequired)
    }
}

private struct FixedServiceAccountReader: ServiceAccountReading {
    let status: ServiceAccountStatus
    var provider: UsageProvider { status.provider }
    func read() async -> ServiceAccountStatus { status }
}
