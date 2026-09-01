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
}
