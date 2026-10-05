import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("System action policy")
struct SystemActionPolicyTests {
    @Test("current XCTest process refuses external opens")
    func currentTestProcessIsRecognized() {
        #expect(!SystemActionPolicy.current.allowsExternalOpen)
    }

    @Test("XCTest process refuses default system opens without invoking the opener")
    func testProcessRefusesSystemOpen() {
        let policy = SystemActionPolicy(
            environment: ["XCTestConfigurationFilePath": "/tmp/fake-tests.xctest"],
            isXCTestBundle: false
        )
        var openCount = 0

        let opened = policy.open(URL(string: "https://example.invalid")!) { _ in
            openCount += 1
            return true
        }

        #expect(!opened)
        #expect(openCount == 0)
    }

    @Test("XCTest script directories resolve under temporary storage")
    func testScriptDirectoryUsesTemporaryStorage() {
        let policy = SystemActionPolicy(
            environment: ["XCTestBundlePath": "/tmp/fake-tests.xctest"],
            isXCTestBundle: false
        )
        let support = URL(fileURLWithPath: "/synthetic/application-support", isDirectory: true)
        let temporary = URL(fileURLWithPath: "/synthetic/temporary", isDirectory: true)

        let directory = policy.scriptDirectory(
            name: "Authentication",
            applicationSupportDirectory: support,
            temporaryDirectory: temporary,
            uniqueSuffix: "test-case"
        )

        #expect(directory.path.hasPrefix(temporary.path + "/"))
        #expect(!directory.path.contains(support.path))
    }

    @MainActor
    @Test("default guide and brand link openers refuse external opens in tests")
    func defaultExternalOpenersAreFailClosed() {
        let policy = SystemActionPolicy(
            environment: ["XCTestConfigurationFilePath": "/tmp/fake-tests.xctest"],
            isXCTestBundle: false
        )
        #expect(!CodexInstallationGuideLauncher(systemActionPolicy: policy).open())
        #expect(!BrandLinkOpenAction(systemActionPolicy: policy).open(AppBrand.authorLinks[0]))
    }

    @MainActor
    @Test("Claude workspace setup refuses before resolving or writing a script")
    func claudeWorkspaceSetupRefusesExternalAction() {
        let policy = SystemActionPolicy(
            environment: ["XCTestConfigurationFilePath": "/tmp/fake-tests.xctest"],
            isXCTestBundle: false
        )
        let launcher = ClaudeWorkspaceSetupLauncher(systemActionPolicy: policy)
        var didRefuse = false

        do {
            try launcher.open()
        } catch {
            didRefuse = true
        }

        #expect(didRefuse)
    }

    @Test("normal process permits the injected system opener")
    func normalProcessCallsInjectedOpener() {
        let policy = SystemActionPolicy(environment: [:], isXCTestBundle: false)
        var openedURL: URL?
        let url = URL(string: "https://example.invalid")!

        let opened = policy.open(url) { openedURL = $0; return true }

        #expect(opened)
        #expect(openedURL == url)
    }
}
