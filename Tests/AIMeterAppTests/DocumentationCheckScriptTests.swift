import Foundation
import Testing

@Suite("Documentation consistency checker", .serialized)
struct DocumentationCheckScriptTests {
    @Test("A complete documentation set passes")
    func acceptsCompleteDocumentation() throws {
        let fixture = try makeFixture()
        defer { try? FileManager.default.removeItem(at: fixture) }

        let result = try runChecker(at: fixture)

        #expect(result.status == 0)
        #expect(result.output.contains("Documentation checks passed"))
    }

    @Test("A broken local Markdown link fails with its source")
    func rejectsBrokenLocalLink() throws {
        let fixture = try makeFixture()
        defer { try? FileManager.default.removeItem(at: fixture) }
        try "[Missing](docs/missing.md)\n".write(
            to: fixture.appendingPathComponent("broken.md"),
            atomically: true,
            encoding: .utf8
        )

        let result = try runChecker(at: fixture)

        #expect(result.status != 0)
        #expect(result.output.contains("broken.md"))
        #expect(result.output.contains("docs/missing.md"))
    }

    @Test("A README and bundle version mismatch fails")
    func rejectsVersionMismatch() throws {
        let fixture = try makeFixture(bundleVersion: "0.2.0")
        defer { try? FileManager.default.removeItem(at: fixture) }

        let result = try runChecker(at: fixture)

        #expect(result.status != 0)
        #expect(result.output.contains("README version 0.1.0 does not match Info.plist 0.2.0"))
    }

    @Test("README download guidance follows the bundle version")
    func rejectsStaleDownloadGuidance() throws {
        let fixture = try makeFixture(bundleVersion: "0.2.0", readmeVersion: "0.2.0", downloadVersion: "0.1.2")
        defer { try? FileManager.default.removeItem(at: fixture) }

        let result = try runChecker(at: fixture)

        #expect(result.status != 0)
        #expect(result.output.contains("v0.2.0 download guidance"))
    }

    @Test("A missing public community file fails")
    func rejectsMissingPublicCommunityFile() throws {
        let fixture = try makeFixture()
        defer { try? FileManager.default.removeItem(at: fixture) }
        try FileManager.default.removeItem(at: fixture.appendingPathComponent("LICENSE"))

        let result = try runChecker(at: fixture)

        #expect(result.status != 0)
        #expect(result.output.contains("LICENSE"))
    }

    @Test("A README missing its public author credit fails")
    func rejectsMissingPublicAuthorCredit() throws {
        let fixture = try makeFixture()
        defer { try? FileManager.default.removeItem(at: fixture) }
        let readmeURL = fixture.appendingPathComponent("README.md")
        let original = try String(contentsOf: readmeURL, encoding: .utf8)
        try original.replacingOccurrences(of: "Author: Miller", with: "")
            .write(to: readmeURL, atomically: true, encoding: .utf8)

        let result = try runChecker(at: fixture)

        #expect(result.status != 0)
        #expect(result.output.contains("public author credit"))
    }

    private func makeFixture(
        bundleVersion: String = "0.1.0",
        readmeVersion: String = "0.1.0",
        downloadVersion: String = "0.1.0"
    ) throws -> URL {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-token-meter-doc-check-\(UUID().uuidString)", isDirectory: true)
        let requiredDirectories = [
            "Sources/AIMeterApp/Resources",
            "docs/architecture",
            "docs/design/specifications",
            "docs/design/implementation-plans",
            "docs/development",
            "docs/assets/screenshots",
            ".github/ISSUE_TEMPLATE",
            ".github/workflows",
        ]
        for directory in requiredDirectories {
            try FileManager.default.createDirectory(
                at: root.appendingPathComponent(directory, isDirectory: true),
                withIntermediateDirectories: true
            )
        }

        try """
        # AI Token Meter

        ![Version \(readmeVersion)](https://img.shields.io/badge/version-\(readmeVersion)-green)
        ![Tests 305](https://img.shields.io/badge/tests-305%20passed-green)

        A native macOS usage meter for Claude Code, OpenAI Codex, and DeepSeek.

        ![Floating strip](docs/assets/screenshots/floating-strip.png)
        ![Provider detail](docs/assets/screenshots/provider-detail.png)
        ![Settings](docs/assets/screenshots/settings.png)

        Download v\(downloadVersion). The public package is ad-hoc signed and not notarized.

        Author: Miller · MIT License

        [Documentation](docs/README.md)
        """.write(to: root.appendingPathComponent("README.md"), atomically: true, encoding: .utf8)

        for filename in ["floating-strip.png", "provider-detail.png", "settings.png"] {
            try Data([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]).write(
                to: root.appendingPathComponent("docs/assets/screenshots/\(filename)")
            )
        }

        try """
        <?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0"><dict>
        <key>CFBundleShortVersionString</key><string>\(bundleVersion)</string>
        </dict></plist>
        """.write(
            to: root.appendingPathComponent("Sources/AIMeterApp/Resources/Info.plist"),
            atomically: true,
            encoding: .utf8
        )

        let documents: [String: String] = [
            "LICENSE": "MIT License\n",
            "SECURITY.md": "# Security\n",
            "CONTRIBUTING.md": "# Contributing\n",
            "CODE_OF_CONDUCT.md": "# Code of Conduct\n",
            "SUPPORT.md": "# Support\n",
            ".github/ISSUE_TEMPLATE/bug_report.yml": "name: Bug report\n",
            ".github/ISSUE_TEMPLATE/feature_request.yml": "name: Feature request\n",
            ".github/ISSUE_TEMPLATE/config.yml": "blank_issues_enabled: false\n",
            ".github/pull_request_template.md": "# Pull request\n",
            ".github/workflows/ci.yml": "name: CI\n",
            "docs/README.md": "# Documentation\n\n[Status](project-status.md)\n",
            "docs/project-status.md": "# Project status\n",
            "docs/requirements-backlog.md": "# Requirements\n",
            "docs/architecture/decisions.md": "# Decisions\n",
            "docs/development/maintenance-playbook.md": "# Maintenance\n",
            "docs/development/2026-09-02-project-retrospective.md": "# Retrospective\n",
            "docs/development/testing.md": "Current baseline: **305 tests**.\n",
        ]
        for (path, contents) in documents {
            try contents.write(
                to: root.appendingPathComponent(path),
                atomically: true,
                encoding: .utf8
            )
        }
        return root
    }

    private func runChecker(at fixture: URL) throws -> (status: Int32, output: String) {
        let process = Process()
        let output = Pipe()
        process.executableURL = repositoryRoot.appendingPathComponent("scripts/check-docs.sh")
        process.arguments = [fixture.path]
        process.standardOutput = output
        process.standardError = output
        try process.run()
        process.waitUntilExit()
        let data = output.fileHandleForReading.readDataToEndOfFile()
        return (process.terminationStatus, String(decoding: data, as: UTF8.self))
    }

    private var repositoryRoot: URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
    }
}
