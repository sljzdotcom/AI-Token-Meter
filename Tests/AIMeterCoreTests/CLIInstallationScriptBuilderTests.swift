import Foundation
import Testing
@testable import AIMeterCore

@Suite("Official CLI installation scripts")
struct CLIInstallationScriptBuilderTests {
    @Test("Failed downloads cannot execute partial installer; success preserves exit status", arguments: [UsageProvider.claude, .codex])
    func downloadBoundary(provider: UsageProvider) throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let script = try CLIInstallationScriptBuilder().build(provider: provider)
        let scriptURL = root.appendingPathComponent("install.command")
        try script.write(to: scriptURL, atomically: true, encoding: .utf8)
        let curl = root.appendingPathComponent("curl")
        let fixture = """
        #!/bin/sh
        while [ "$#" -gt 0 ]; do
          if [ "$1" = "--output" ]; then shift; target="$1"; fi
          case "$1" in https:*) printf '%s' "$1" > "$FIXTURE_ROOT/url";; esac
          shift
        done
        printf 'touch "$FIXTURE_ROOT/executed"; exit 7\\n' > "$target"
        exit "$DOWNLOAD_EXIT"
        """
        try fixture.write(to: curl, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: curl.path)
        for exitCode in ["22", "0"] {
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/bin/sh")
            process.arguments = [scriptURL.path]
            process.environment = ["PATH": root.path + ":/usr/bin:/bin", "FIXTURE_ROOT": root.path, "DOWNLOAD_EXIT": exitCode, "TMPDIR": root.path]
            process.standardOutput = FileHandle.nullDevice
            process.standardError = FileHandle.nullDevice
            try process.run()
            process.waitUntilExit()
            #expect(FileManager.default.fileExists(atPath: root.appendingPathComponent("executed").path) == (exitCode == "0"))
            #expect(process.terminationStatus == (exitCode == "0" ? 7 : 22))
            #expect(try String(contentsOf: root.appendingPathComponent("url"), encoding: .utf8) == (provider == .claude ? "https://claude.ai/install.sh" : "https://chatgpt.com/codex/install.sh"))
            #expect(try FileManager.default.contentsOfDirectory(atPath: root.path).filter { $0.hasPrefix("ai-meter-install.") }.isEmpty)
        }
    }
}
