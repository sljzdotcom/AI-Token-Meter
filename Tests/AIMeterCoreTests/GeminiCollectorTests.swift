import Foundation
import Testing
@testable import AIMeterCore

@Suite("Gemini collector safety")
struct GeminiCollectorTests {
    @Test func collectsOnlyPinnedVersionAndPreservesQuotaAndPrivateEnvironment() async throws {
        let context = try context(); defer { try? FileManager.default.removeItem(at: context.root) }
        let runner = RecordingGeminiRunner(version: "0.58.0")
        let collector = GeminiCollector(runner: runner, locator: GeminiTestLocator(), environment: context.environment)
        let snapshot = try await collector.collect()
        #expect(snapshot.geminiQuotaMetrics?.map(\.current) == [25, 60])
        let requests = await runner.requests
        #expect(requests.count == 2)
        #expect(requests[0].arguments.contains("--version"))
        #expect(requests[1].geminiQuotaInteraction)
        #expect(requests.allSatisfy { !$0.arguments.contains("-p") && $0.inputLines.isEmpty })
        #expect(requests[1].arguments.prefix(2) == ["-e", "none"])
        #expect(requests[1].environment?["HOME"] == context.root.path)
        #expect(requests[1].environment?["NO_BROWSER"] == "true")
        #expect(requests[1].environment?["GEMINI_CLI_TRUST_WORKSPACE"] == "false")
        #expect(requests[1].environment?["NODE_OPTIONS"] == nil)
        #expect(requests[0].currentDirectoryURL == requests[1].currentDirectoryURL)
        #expect(!FileManager.default.fileExists(atPath: requests[1].currentDirectoryURL!.path))
        #expect(await runner.isolationObserved)
    }
    @Test func versionMismatchNeverStartsQuota() async throws {
        let context = try context(); defer { try? FileManager.default.removeItem(at: context.root) }
        let runner = RecordingGeminiRunner(version: "0.57.0")
        await #expect(throws: UsageCollectionError.geminiUnavailable("Gemini CLI version is not supported (requires 0.58.0)")) {
            try await GeminiCollector(runner: runner, locator: GeminiTestLocator(), environment: context.environment).collect()
        }
        #expect(await runner.requests.count == 1)
    }
    @Test func missingAndInaccessibleExecutablesAreDifferent() async throws {
        let context = try context(); defer { try? FileManager.default.removeItem(at: context.root) }
        await #expect(throws: UsageCollectionError.notInstalled) {
            try await GeminiCollector(locator: GeminiTestLocator(discovery: .missing), environment: context.environment).collect()
        }
        await #expect(throws: UsageCollectionError.geminiUnavailable("Gemini CLI is not executable")) {
            try await GeminiCollector(locator: GeminiTestLocator(discovery: .unavailable), environment: context.environment).collect()
        }
    }
    @Test(arguments: [#"{"security":{"auth":{"selectedType":"gemini-api-key"}}}"#,
                      #"{"security":{"auth":{"selectedType":"oauth-personal"}},"tools":{"discoveryCommand":"touch sentinel"}}"#,
                      #"{"security":{"auth":{"selectedType":"oauth-personal"}},"tools":{"sandbox":true}}"#,
                      #"{"security":{"auth":{"selectedType":"oauth-personal"}},"advanced":{"ignoreLocalEnv":true}}"#])
    func unsupportedSettingsNeverStartAnyProcess(_ settings: String) async throws {
        let context = try context(settings: settings); defer { try? FileManager.default.removeItem(at: context.root) }
        let runner = RecordingGeminiRunner(version: "0.58.0")
        await #expect(throws: (any Error).self) { try await GeminiCollector(runner: runner, locator: GeminiTestLocator(), environment: context.environment).collect() }
        #expect(await runner.requests.isEmpty)
    }
    @Test(arguments: ["NODE_OPTIONS", "GEMINI_FORCE_ENCRYPTED_FILE_STORAGE", "GOOGLE_APPLICATION_CREDENTIALS", "GEMINI_CLI_SYSTEM_SETTINGS_PATH", "GEMINI_SANDBOX"])
    func injectedEnvironmentNeverStartsAnyProcess(_ key: String) async throws {
        let context = try context(extraEnvironment: [key: "unverified"]); defer { try? FileManager.default.removeItem(at: context.root) }
        let runner = RecordingGeminiRunner(version: "0.58.0")
        await #expect(throws: (any Error).self) { try await GeminiCollector(runner: runner, locator: GeminiTestLocator(), environment: context.environment).collect() }
        #expect(await runner.requests.isEmpty)
    }
    @Test(arguments: ["settings.json", "system-defaults.json"])
    func enterpriseSettingsAreNeverReplaced(_ filename: String) async throws {
        let context = try context(); defer { try? FileManager.default.removeItem(at: context.root) }
        try Data("{}".utf8).write(to: context.root.appendingPathComponent("system/\(filename)"))
        let runner = RecordingGeminiRunner(version: "0.58.0")
        await #expect(throws: (any Error).self) { try await GeminiCollector(runner: runner, locator: GeminiTestLocator(), environment: context.environment).collect() }
        #expect(await runner.requests.isEmpty)
    }
    @Test(arguments: [#"{"security":{"auth":{"selectedType":"oauth-personal","useExternal":true}}}"#,
                      #"{"security":{"auth":{"selectedType":"oauth-personal","enforcedType":"vertex-ai"}}}"#,
                      #"{"security":{"auth":{"selectedType":"oauth-personal"},"toolSandboxing":true}}"#])
    func refusesExternalAuthenticationAndToolSandbox(_ settings: String) async throws {
        let context = try context(settings: settings); defer { try? FileManager.default.removeItem(at: context.root) }
        let runner = RecordingGeminiRunner(version: "0.58.0")
        await #expect(throws: UsageCollectionError.geminiUnavailable("Gemini CLI security mode is not supported")) {
            try await GeminiCollector(runner: runner, locator: GeminiTestLocator(), environment: context.environment).collect()
        }
        #expect(await runner.requests.isEmpty)
    }

    @Test(arguments: [#"{"security":{"auth":[]}}"#, #"{"security":{"auth":{"selectedType":42}}}"#])
    func malformedAuthenticationIsUnavailableInsteadOfSignInRequired(_ settings: String) async throws {
        let context = try context(settings: settings); defer { try? FileManager.default.removeItem(at: context.root) }
        let runner = RecordingGeminiRunner(version: "0.58.0")
        await #expect(throws: UsageCollectionError.geminiUnavailable("Gemini CLI settings are not supported")) {
            try await GeminiCollector(runner: runner, locator: GeminiTestLocator(), environment: context.environment).collect()
        }
        #expect(await runner.requests.isEmpty)
    }

    private func context(settings: String = #"{"security":{"auth":{"selectedType":"oauth-personal"}}}"#, extraEnvironment: [String: String] = [:]) throws -> (root: URL, environment: GeminiCLIEnvironment) {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("gemini-home-\(UUID())")
        try FileManager.default.createDirectory(at: root.appendingPathComponent(".gemini"), withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: root.appendingPathComponent("system"), withIntermediateDirectories: true)
        try Data(settings.utf8).write(to: root.appendingPathComponent(".gemini/settings.json"))
        return (root, GeminiCLIEnvironment(home: root, source: extraEnvironment, systemDirectory: root.appendingPathComponent("system")))
    }
}
private struct GeminiTestLocator: ExecutableLocating {
    var discovery: ExecutableDiscovery = .found(URL(fileURLWithPath: "/synthetic/gemini"))
    func discover(named name: String) -> ExecutableDiscovery { discovery }
    func locate(named name: String) -> URL? { if case .found(let url) = discovery { url } else { nil } }
}
private actor RecordingGeminiRunner: CommandRunning {
    let version: String
    var requests: [CommandRequest] = []
    var isolationObserved = false
    init(version: String) { self.version = version }
    func run(_ request: CommandRequest) async throws -> CommandResult {
        requests.append(request)
        if let cwd = request.currentDirectoryURL, let env = request.environment,
           let settingsPath = env["GEMINI_CLI_SYSTEM_SETTINGS_PATH"],
           let settings = try JSONSerialization.jsonObject(with: Data(contentsOf: URL(fileURLWithPath: settingsPath))) as? [String: Any] {
            isolationObserved = (try String(contentsOf: cwd.appendingPathComponent(".env"), encoding: .utf8)).isEmpty
                && (settings["hooksConfig"] as? [String: Bool])?["enabled"] == false
                && (settings["privacy"] as? [String: Bool])?["usageStatisticsEnabled"] == false
        }
        return CommandResult(output: request.arguments.contains("--version") ? version : GeminiUsageParserTests.frame("Pro 25%\nFlash 60%"), exitCode: 0, duration: 0.1)
    }
}
