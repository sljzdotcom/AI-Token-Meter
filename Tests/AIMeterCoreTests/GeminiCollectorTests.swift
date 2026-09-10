import Foundation
import Testing
@testable import AIMeterCore

@Suite("Antigravity collector safety")
struct GeminiCollectorTests {
    @Test func defaultRunnerUsesTheNoninteractiveHeadlessPath() async throws {
        let context = try context(); defer { try? FileManager.default.removeItem(at: context.root) }
        let executable = context.root.appendingPathComponent("agy")
        let script = """
        #!/bin/sh
        if [ -t 1 ]; then exit 9; fi
        if [ "$1" = "--version" ]; then
          printf '1.1.28\\n'
        else
          cat <<'OUTPUT'
        \(GeminiUsageParserTests.fixture)
        OUTPUT
        fi
        """
        try Data(script.utf8).write(to: executable)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: executable.path)

        let snapshot = try await GeminiCollector(
            locator: AntigravityTestLocator(discovery: .found(executable)),
            environment: context.environment
        ).collect()
        #expect(snapshot.geminiQuotaMetrics?.count == 4)
    }

    @Test func usesOnlyOfficialHeadlessUsageCommandInPrivateDirectory() async throws {
        let context = try context(); defer { try? FileManager.default.removeItem(at: context.root) }
        let runner = RecordingAntigravityRunner(version: "1.1.28")
        let collector = GeminiCollector(
            runner: runner,
            locator: AntigravityTestLocator(),
            environment: context.environment
        )

        let snapshot = try await collector.collect()
        #expect(snapshot.geminiQuotaMetrics?.map(\.current) == [60, 25, 80, 20])
        let requests = await runner.requests
        #expect(requests.count == 2)
        #expect(requests[0].arguments.contains("--version"))
        #expect(requests[1].arguments.prefix(2) == ["-p", "/usage"])
        #expect(requests[1].arguments.contains("--print-timeout"))
        #expect(requests.allSatisfy { $0.inputLines.isEmpty })
        #expect(requests.allSatisfy { $0.maxOutputBytes == 64 * 1024 })
        #expect(requests[1].environment?["HOME"] == context.root.path)
        #expect(requests[1].environment?["NO_BROWSER"] == "true")
        #expect(requests[1].environment?["GEMINI_API_KEY"] == nil)
        #expect(requests[0].currentDirectoryURL == requests[1].currentDirectoryURL)
        #expect(!FileManager.default.fileExists(atPath: requests[1].currentDirectoryURL!.path))
    }

    @Test(arguments: ["1.1.28", "1.1.29", "1.2.0", "1.99.1"])
    func acceptsSupportedMajorWhenStrictUsageOutputMatches(_ version: String) async throws {
        let context = try context(); defer { try? FileManager.default.removeItem(at: context.root) }
        let snapshot = try await GeminiCollector(
            runner: RecordingAntigravityRunner(version: version),
            locator: AntigravityTestLocator(),
            environment: context.environment
        ).collect()
        #expect(snapshot.sourceVersion == version)
    }

    @Test(arguments: ["1.1.27", "0.58.0", "2.0.0", "1.1", "1.1.28-beta", "1.1.28+build", "not-a-version"])
    func unsupportedVersionNeverStartsUsage(_ version: String) async throws {
        let context = try context(); defer { try? FileManager.default.removeItem(at: context.root) }
        let runner = RecordingAntigravityRunner(version: version)
        await #expect(throws: UsageCollectionError.geminiUnavailable("Antigravity CLI version is not supported (requires 1.1.28 or later in major version 1)")) {
            try await GeminiCollector(
                runner: runner,
                locator: AntigravityTestLocator(),
                environment: context.environment
            ).collect()
        }
        #expect(await runner.requests.count == 1)
    }

    @Test(arguments: ["authentication required", "sign in required", "please sign in", "not authenticated", "login required"])
    func authenticationFailureIsDistinctFromTransportFailure(_ message: String) async throws {
        let context = try context(); defer { try? FileManager.default.removeItem(at: context.root) }
        let runner = RecordingAntigravityRunner(version: "1.1.28", usageExitCode: 2, usageOutput: message)
        await #expect(throws: UsageCollectionError.authenticationRequired) {
            try await GeminiCollector(runner: runner, locator: AntigravityTestLocator(), environment: context.environment).collect()
        }
    }

    @Test func missingAndInaccessibleExecutablesAreDifferentAndOnlyAgyIsDiscovered() async throws {
        let context = try context(); defer { try? FileManager.default.removeItem(at: context.root) }
        await #expect(throws: UsageCollectionError.notInstalled) {
            try await GeminiCollector(locator: AntigravityTestLocator(discovery: .missing), environment: context.environment).collect()
        }
        await #expect(throws: UsageCollectionError.geminiUnavailable("Antigravity CLI is not executable")) {
            try await GeminiCollector(locator: AntigravityTestLocator(discovery: .unavailable), environment: context.environment).collect()
        }
    }

    @Test(arguments: ["GEMINI_API_KEY", "GOOGLE_API_KEY", "AGY_DEBUG", "HTTP_PROXY", "SSL_CERT_FILE", "DYLD_INSERT_LIBRARIES", "BASH_ENV"])
    func injectedEnvironmentNeverStartsAnyProcess(_ key: String) async throws {
        let context = try context(extraEnvironment: [key: "unverified"])
        defer { try? FileManager.default.removeItem(at: context.root) }
        let runner = RecordingAntigravityRunner(version: "1.1.28")
        await #expect(throws: UsageCollectionError.geminiUnavailable("Antigravity CLI environment uses an unsupported override")) {
            try await GeminiCollector(runner: runner, locator: AntigravityTestLocator(), environment: context.environment).collect()
        }
        #expect(await runner.requests.isEmpty)
    }

    private func context(extraEnvironment: [String: String] = [:]) throws -> (root: URL, environment: GeminiCLIEnvironment) {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("antigravity-home-\(UUID())")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        return (root, GeminiCLIEnvironment(home: root, source: extraEnvironment))
    }
}

private struct AntigravityTestLocator: ExecutableLocating {
    var discovery: ExecutableDiscovery = .found(URL(fileURLWithPath: "/synthetic/agy"))
    func discover(named name: String) -> ExecutableDiscovery { name == "agy" ? discovery : .missing }
    func locate(named name: String) -> URL? {
        guard name == "agy", case .found(let url) = discovery else { return nil }
        return url
    }
}

private actor RecordingAntigravityRunner: CommandRunning {
    let version: String
    let usageExitCode: Int32
    let usageOutput: String
    var requests: [CommandRequest] = []

    init(version: String, usageExitCode: Int32 = 0, usageOutput: String = GeminiUsageParserTests.fixture) {
        self.version = version
        self.usageExitCode = usageExitCode
        self.usageOutput = usageOutput
    }

    func run(_ request: CommandRequest) async throws -> CommandResult {
        requests.append(request)
        return CommandResult(
            output: request.arguments.contains("--version") ? version : usageOutput,
            exitCode: request.arguments.contains("--version") ? 0 : usageExitCode,
            duration: 0.1
        )
    }
}
