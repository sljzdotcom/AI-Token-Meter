import Foundation
import Testing
@testable import AIMeterCore

/// Explicit characterization against the already-installed, isolated upstream fixture.
/// This never discovers a user's executable, account, or home directory.
@Suite("Gemini official CLI through production PTY", .serialized,
       .enabled(if: ProcessInfo.processInfo.environment["AI_METER_GEMINI_OFFICIAL_PTY"] == "1"))
struct GeminiOfficialPTYTests {
    @Test func readsQuotaInExplicitlyUntrustedWorkspace() async throws {
        let root = URL(fileURLWithPath: "/private/tmp/req012-gemini-startup/authenticated-untrusted-allowlist")
        let data = try Data(contentsOf: root.appendingPathComponent("options.json"))
        let options = try JSONDecoder().decode(Options.self, from: data)
        #expect(options.env["HOME"] == root.appendingPathComponent("home").path)
        #expect(options.env["GEMINI_CLI_TRUST_WORKSPACE"] == "false")
        #expect(options.argv.contains("--permission"))
        #expect(options.argv.contains("--require"))
        // The Node permission guard and Nock synthetic auth remain on this test-only boundary.
        var environment = options.env
        environment["PROBE_EVENTS"] = root.appendingPathComponent("swift-pty-events.jsonl").path
        try Data().write(to: URL(fileURLWithPath: environment["PROBE_EVENTS"]!))
        let result = try await PTYCommandRunner().run(CommandRequest(
            executableURL: URL(fileURLWithPath: options.argv[0]), arguments: Array(options.argv.dropFirst()),
            inputLines: [], timeout: 25, currentDirectoryURL: URL(fileURLWithPath: options.cwd),
            environment: environment, geminiQuotaInteraction: true))
        #expect(result.exitCode == 0)
        let snapshot = try GeminiUsageParser().parse(result.output)
        #expect(snapshot.geminiQuotaMetrics?.map(\.current) == [25, 60])
        #expect(snapshot.geminiQuotaMetrics?.map(\.label) == ["Pro", "Flash"])
        #expect(!FileManager.default.fileExists(atPath: root.appendingPathComponent("home/.gemini/trustedFolders.json").path))
    }
    @Test(arguments: ["missing-auth-allowlist", "invalid-auth-allowlist", "quota-failure-allowlist"])
    func handlesActualAuthenticationAndMissingQuota(_ scenario: String) async throws {
        let root = URL(fileURLWithPath: "/private/tmp/req012-gemini-startup").appendingPathComponent(scenario)
        let options = try JSONDecoder().decode(Options.self, from: Data(contentsOf: root.appendingPathComponent("options.json")))
        var environment = options.env
        environment["PROBE_EVENTS"] = root.appendingPathComponent("swift-pty-events.jsonl").path
        try Data().write(to: URL(fileURLWithPath: environment["PROBE_EVENTS"]!))
        let request = CommandRequest(executableURL: URL(fileURLWithPath: options.argv[0]), arguments: Array(options.argv.dropFirst()),
            inputLines: [], timeout: 25, currentDirectoryURL: URL(fileURLWithPath: options.cwd),
            environment: environment, geminiQuotaInteraction: true)
        if scenario == "quota-failure-allowlist" {
            let result = try await PTYCommandRunner().run(request)
            #expect(result.exitCode == 0)
            #expect(throws: UsageCollectionError.geminiUnavailable("Gemini CLI did not provide quota")) { try GeminiUsageParser().parse(result.output) }
        } else {
            await #expect(throws: UsageCollectionError.authenticationRequired) { try await PTYCommandRunner().run(request) }
        }
        let events = try String(contentsOf: URL(fileURLWithPath: environment["PROBE_EVENTS"]!), encoding: .utf8)
        let types = try events.split(separator: "\n").map {
            (try JSONSerialization.jsonObject(with: Data($0.utf8)) as? [String: Any])?["type"] as? String
        }
        #expect(!types.contains("network-denied"))
        #expect(!types.contains("trust-write-attempt"))
    }
    private struct Options: Decodable {
        let argv: [String]
        let cwd: String
        let env: [String: String]
    }
}
