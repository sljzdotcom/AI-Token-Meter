import Foundation
import Testing

@Suite("Public release safety gate", .serialized)
struct PublicReleaseSafetyScriptTests {
    @Test("Accepts a repository that contains no credential patterns")
    func acceptsSafeRepository() throws {
        let fixture = try PublicReleaseFixture()
        defer { fixture.remove() }
        try fixture.commit(path: "README.md", contents: "# Safe public project\n")

        let result = try fixture.runSafetyCheck()

        #expect(result.status == 0)
        #expect(result.output.contains("Public release safety checks passed"))
    }

    @Test("Rejects a credential in the current worktree without echoing it")
    func rejectsCurrentSecret() throws {
        let fixture = try PublicReleaseFixture()
        defer { fixture.remove() }
        try fixture.commit(path: "README.md", contents: "# Safe public project\n")
        try fixture.write(path: "local.txt", contents: fixture.telegramToken)

        let result = try fixture.runSafetyCheck()

        #expect(result.status != 0)
        #expect(result.output.contains("current repository content"))
        #expect(!result.output.contains(fixture.telegramToken))
    }

    @Test("Rejects a credential that exists only in Git history")
    func rejectsHistoricalSecret() throws {
        let fixture = try PublicReleaseFixture()
        defer { fixture.remove() }
        try fixture.commit(path: "secret.txt", contents: fixture.telegramToken)
        try fixture.commit(path: "secret.txt", contents: "removed\n")

        let result = try fixture.runSafetyCheck()

        #expect(result.status != 0)
        #expect(result.output.contains("Git history"))
        #expect(!result.output.contains(fixture.telegramToken))
    }

    @Test("Rejects a tracked private key filename")
    func rejectsPrivateKeyFilename() throws {
        let fixture = try PublicReleaseFixture()
        defer { fixture.remove() }
        try fixture.commit(path: "README.md", contents: "# Safe public project\n")
        try fixture.commit(path: "update-signing.key", contents: "fixture only\n")

        let result = try fixture.runSafetyCheck()

        #expect(result.status != 0)
        #expect(result.output.contains("credential-bearing filename"))
        #expect(!result.output.contains("update-signing.key"))
    }

    @Test("Rejects a Sparkle private-key export marker without echoing it")
    func rejectsSparklePrivateKeyExportMarker() throws {
        let fixture = try PublicReleaseFixture()
        defer { fixture.remove() }
        try fixture.commit(path: "README.md", contents: "# Safe public project\n")
        let marker = "[SPARKLE " + "PRIVATE KEY EXPORT]"
        try fixture.write(path: "local.txt", contents: marker)

        let result = try fixture.runSafetyCheck()

        #expect(result.status != 0)
        #expect(result.output.contains("current repository content"))
        #expect(!result.output.contains(marker))
    }

    @Test("Gitleaks scans only files eligible for the public Git worktree")
    func gitleaksUsesFilteredWorktreeSnapshot() throws {
        let fixture = try PublicReleaseFixture()
        defer { fixture.remove() }
        try fixture.commit(path: "README.md", contents: "# Safe public project\n")
        let probePath = try fixture.installGitleaksProbe()

        let result = try fixture.runSafetyCheck(
            environment: [
                "PATH": "\(probePath):\(ProcessInfo.processInfo.environment["PATH"] ?? "")",
                "EXPECTED_REPOSITORY": fixture.root.path,
            ]
        )

        #expect(result.status == 0)
        #expect(result.output.contains("Public release safety checks passed"))
    }
}

private final class PublicReleaseFixture {
    let root: URL
    let telegramToken = "1234567890:" + "AA" + String(repeating: "x", count: 35)

    init() throws {
        root = FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-token-meter-public-release-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        _ = try command("git", ["init", "-q"])
        _ = try command("git", ["config", "user.name", "Release Test"])
        _ = try command("git", ["config", "user.email", "release-test@example.invalid"])
    }

    func write(path: String, contents: String) throws {
        try write(at: root, path: path, contents: contents)
    }

    func write(at repository: URL, path: String, contents: String) throws {
        let url = repository.appendingPathComponent(path)
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try contents.write(to: url, atomically: true, encoding: .utf8)
    }

    func commit(path: String, contents: String) throws {
        try write(path: path, contents: contents)
        _ = try command("git", ["add", path])
        _ = try command("git", ["commit", "-q", "-m", "fixture update"])
    }

    func runSafetyCheck(environment: [String: String] = [:]) throws -> CommandResult {
        try command(
            repositoryScript.path,
            ["--repository", root.path],
            environment: environment
        )
    }

    func installGitleaksProbe() throws -> String {
        let directory = root.appendingPathComponent("test-bin", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let executable = directory.appendingPathComponent("gitleaks")
        try """
        #!/bin/sh
        if [ "$1" = "dir" ] && [ "$2" = "$EXPECTED_REPOSITORY" ]; then
          exit 19
        fi
        exit 0
        """.write(to: executable, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes(
            [.posixPermissions: 0o755],
            ofItemAtPath: executable.path
        )
        return directory.path
    }

    func remove() {
        try? FileManager.default.removeItem(at: root)
    }

    private var repositoryScript: URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("scripts/check-public-release.sh")
    }

    @discardableResult
    private func command(
        _ executable: String,
        _ arguments: [String],
        environment: [String: String] = [:]
    ) throws -> CommandResult {
        let process = Process()
        let output = Pipe()
        if executable.contains("/") {
            process.executableURL = URL(fileURLWithPath: executable)
            process.arguments = arguments
        } else {
            process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
            process.arguments = [executable] + arguments
        }
        process.currentDirectoryURL = root
        process.environment = ProcessInfo.processInfo.environment.merging(environment) { _, override in
            override
        }
        process.standardOutput = output
        process.standardError = output
        do {
            try process.run()
        } catch {
            return CommandResult(status: 127, output: error.localizedDescription)
        }
        process.waitUntilExit()
        let bytes = output.fileHandleForReading.readDataToEndOfFile()
        return CommandResult(
            status: process.terminationStatus,
            output: String(decoding: bytes, as: UTF8.self)
        )
    }
}

private struct CommandResult {
    let status: Int32
    let output: String
}
