import Foundation
import Testing
@testable import AIMeterCore

@Suite("CLI authentication script builder")
struct CLIAuthenticationScriptBuilderTests {
    @Test("Claude and Codex scripts contain only approved login commands")
    func approvedCommands() throws {
        let builder = CLIAuthenticationScriptBuilder()
        let claude = try builder.build(
            provider: .claude,
            executableURL: URL(fileURLWithPath: "/tmp/Claude CLI/claude")
        )
        let codex = try builder.build(
            provider: .codex,
            executableURL: URL(fileURLWithPath: "/tmp/Codex CLI/codex")
        )

        #expect(claude.contains("exec '/tmp/Claude CLI/claude' auth login"))
        #expect(codex.contains("exec '/tmp/Codex CLI/codex' login"))
        #expect(codex.contains("export PATH='/tmp/Codex CLI':\"${PATH:-/usr/bin:/bin:/usr/sbin:/sbin}\""))
        #expect(!claude.localizedCaseInsensitiveContains("token"))
        #expect(!codex.localizedCaseInsensitiveContains("api key"))
    }

    @Test("Executable paths use safe shell quoting")
    func quotesPaths() throws {
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .claude,
            executableURL: URL(fileURLWithPath: "/tmp/Miller's Tools/claude")
        )

        #expect(script.contains("'/tmp/Miller'\\''s Tools/claude'"))
    }

    @Test("Antigravity login is interactive and sends completion only after it exits")
    func geminiInteractiveCompletionSignal() throws {
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: URL(fileURLWithPath: "/tmp/Antigravity CLI/agy"),
            completionToken: "12345678-1234-1234-1234-123456789abc"
        )

        #expect(script.contains("'/tmp/Antigravity CLI/agy'"))
        #expect(script.contains("aitokenmeter://antigravity-login-complete?token=12345678-1234-1234-1234-123456789abc"))
        #expect(script.range(of: "aitokenmeter://antigravity-login-complete")!.lowerBound > script.range(of: "'/tmp/Antigravity CLI/agy'")!.lowerBound)
        #expect(script.contains("status=$?"))
    }

    @Test("Antigravity scripts reject missing or malformed one-time completion tokens")
    func requiresCompletionToken() {
        let builder = CLIAuthenticationScriptBuilder()
        #expect(throws: CLIAuthenticationScriptError.invalidCompletionToken) {
            try builder.build(provider: .gemini, executableURL: URL(fileURLWithPath: "/tmp/agy"))
        }
        #expect(throws: CLIAuthenticationScriptError.invalidCompletionToken) {
            try builder.build(provider: .gemini, executableURL: URL(fileURLWithPath: "/tmp/agy"), completionToken: "not-a-token")
        }
    }

    @Test("DeepSeek can never be routed to a CLI authentication script")
    func rejectsDeepSeek() {
        #expect(throws: CLIAuthenticationScriptError.unsupportedProvider) {
            try CLIAuthenticationScriptBuilder().build(
                provider: .deepSeek,
                executableURL: URL(fileURLWithPath: "/tmp/deepseek")
            )
        }
    }
}
