import Testing
@testable import AIMeterCore

@Suite("ANSI terminal text sanitizer")
struct ANSITextSanitizerTests {
    @Test("Removes color, OSC title, carriage return, and backspace sequences")
    func removesTerminalControlSequences() {
        let raw = "\u{001B}]0;Claude\u{0007}\u{001B}[31mCurrent sessX\u{0008}ion\u{001B}[0m\r73% used\r\n"

        let result = ANSITextSanitizer.sanitize(raw)

        #expect(result == "Current session\n73% used\n")
    }
}

