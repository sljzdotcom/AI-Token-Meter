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
