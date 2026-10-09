import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("AppModel external action defaults")
@MainActor
struct AppModelExternalActionPolicyTests {
    @Test("omitted Claude, Codex, and Gemini login injections are refused in tests")
    func omittedAuthenticationOperationsFailClosed() async throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-app-model-policy-\(UUID())", isDirectory: true)
        defer { try? FileManager.default.removeItem(at: root) }
        var openedURLs: [URL] = []
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root.appendingPathComponent("Authentication", isDirectory: true),
            executableLocator: PolicyTestExecutableLocator(),
            openURL: { openedURLs.append($0); return true },
            systemActionPolicy: SystemActionPolicy(environment: [:], isXCTestBundle: false)
        )
        let policy = SystemActionPolicy(
            environment: ["XCTestConfigurationFilePath": "/tmp/fake-tests.xctest"],
            isXCTestBundle: false
        )

        for provider in [UsageProvider.claude, .codex, .gemini] {
            let suite = "AppModelExternalActionPolicy.\(provider.rawValue).\(UUID())"
            let defaults = try #require(UserDefaults(suiteName: suite))
            defer { defaults.removePersistentDomain(forName: suite) }
            let model = AppModel(
                defaults: defaults,
                authenticationLauncher: launcher,
                systemActionPolicy: policy,
                widgetSnapshotPublisher: nil,
                isDemoMode: false
            )

            #expect(model.beginSignIn(provider) == nil)
            if provider == .gemini {
                await model.beginGeminiOneTimeRecovery()
            }
            #expect(!model.isAuthenticating)
            #expect(!model.isGeminiSignInPending)
        }

        #expect(openedURLs.isEmpty)
    }
}

private struct PolicyTestExecutableLocator: ExecutableLocating {
    func locate(named name: String) -> URL? {
        URL(fileURLWithPath: "/tmp/fake-\(name)")
    }
}
