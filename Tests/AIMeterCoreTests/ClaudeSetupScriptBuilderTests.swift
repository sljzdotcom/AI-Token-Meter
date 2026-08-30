import Foundation
import Testing
@testable import AIMeterCore

@Suite("Claude setup script builder")
struct ClaudeSetupScriptBuilderTests {
    @Test("Quotes workspace and Claude executable paths")
    func quotesPaths() {
        let script = ClaudeSetupScriptBuilder().build(
            workspaceURL: URL(fileURLWithPath: "/tmp/AI Meter/Claude's Workspace"),
            executableURL: URL(fileURLWithPath: "/tmp/Claude Tools/claude")
        )

        #expect(script.contains("cd -- '/tmp/AI Meter/Claude'\\''s Workspace'"))
        #expect(script.contains("exec '/tmp/Claude Tools/claude' --ax-screen-reader --safe-mode"))
    }
}
