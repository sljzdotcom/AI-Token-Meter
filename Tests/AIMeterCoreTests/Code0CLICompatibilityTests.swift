import Foundation
import Testing
@testable import AIMeterCore

@Suite("Code0 never enters the existing CLI paths")
struct Code0CLICompatibilityTests {
    @Test func code0IsNotAnAutomaticRefreshOrDefaultWidgetProvider() {
        #expect(UsageProvider.refreshProviders == [.claude, .codex, .deepSeek, .gemini])
        #expect(!UsageProvider.refreshProviders.contains(.code0))
        #expect(WidgetProvider.displayedProviders.map(\.rawValue) == ["claude", "codex", "deepSeek"])
    }

    @Test func loginAndInstallationBuildersRejectCode0WithoutExecutingAnything() {
        #expect(throws: CLIAuthenticationScriptError.unsupportedProvider) {
            try CLIAuthenticationScriptBuilder().build(provider: .code0, executableURL: URL(fileURLWithPath: "/synthetic/code0"))
        }
        #expect(throws: CLIAuthenticationScriptError.unsupportedProvider) {
            try CLIInstallationScriptBuilder().build(provider: .code0)
        }
    }

    @Test func siteCreditsNeverBecomeBudgetProgressOrThresholdAlerts() {
        let snapshot = UsageSnapshot(
            provider: .code0,
            primaryMetric: UsageMetric(label: "Balance", current: 95, limit: 100, unit: .siteCredits, kind: .balance),
            fetchedAt: Date(), collectionStatus: .fresh
        )
        #expect(snapshot.primaryMetric?.usedFraction == nil)
        #expect(ProviderPresentation(snapshot: snapshot).fraction == nil)
        var evaluator = ThresholdEvaluator()
        #expect(evaluator.evaluate(snapshot).isEmpty)
    }
}
