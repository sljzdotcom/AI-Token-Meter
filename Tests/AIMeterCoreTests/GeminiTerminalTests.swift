import Foundation
import Testing
@testable import AIMeterCore

@Suite("Gemini terminal protocol")
struct GeminiTerminalTests {
    @Test(arguments: ["authenticated-model-open.ansi.txt", "authenticated-untrusted-model-open.ansi.txt"])
    func replaysRealQuotaFrameAndClearsItOnExit(_ name: String) throws {
        var screen = GeminiTerminalScreen()
        screen.feed(try fixture(name))
        let snapshot = try GeminiUsageParser().parse(screen.text)
        #expect(snapshot.geminiQuotaMetrics?.map(\.current) == [25, 60])
        var full = GeminiTerminalScreen()
        full.feed(try fixture("authenticated.ansi.txt"))
        #expect(throws: (any Error).self) { try GeminiUsageParser().parse(full.text) }
    }
    @Test func readinessRequiresRealInputNotTitleOrOldFrame() throws {
        var screen = GeminiTerminalScreen()
        screen.feed("\u{1b}]0;Ready (workspace)\u{7}Waiting for authentication")
        #expect(!GeminiTerminalProtocol.isReady(screen.text))
        screen = GeminiTerminalScreen()
        screen.feed(try fixture("ready.ansi.txt"))
        #expect(GeminiTerminalProtocol.isReady(screen.text))
        screen.feed("\u{1b}[2J\u{1b}[HSelect a theme\n> Type your message or @path/to/file")
        #expect(!GeminiTerminalProtocol.isReady(screen.text))
    }
    @Test func erasesOlderQuotaRatherThanConcatenatingHistory() {
        var screen = GeminiTerminalScreen()
        screen.feed("Pro 25%\r\nFlash 60%")
        screen.feed("\u{1b}[1A\r\u{1b}[2KPro 30%\u{1b}[1B\r\u{1b}[2K")
        #expect(screen.text.trimmingCharacters(in: .whitespacesAndNewlines) == "Pro 30%")
    }
    @Test(arguments: ["missing-auth.ansi.txt", "invalid-auth.ansi.txt"])
    func recognizesRealAuthenticationWithoutSendingCommands(_ name: String) throws {
        var screen = GeminiTerminalScreen(); screen.feed(try fixture(name))
        #expect(GeminiTerminalProtocol.blockingError(screen.text) == .authenticationRequired)
        #expect(!GeminiTerminalProtocol.isReady(screen.text))
    }
    func fixture(_ name: String) throws -> String {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        return try String(contentsOf: root.appendingPathComponent("contracts/gemini-cli/0.58.0/\(name)"), encoding: .utf8)
    }
}
