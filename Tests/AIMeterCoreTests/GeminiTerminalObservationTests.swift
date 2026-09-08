import Foundation
import Testing
@testable import AIMeterCore

@Suite("Gemini complete frame observation")
struct GeminiTerminalObservationTests {
    @Test(arguments: [0, 1, 2])
    func conflictingFramesAreRejectedRegardlessOfReadChunking(_ chunking: Int) {
        let first = frame(25)
        let second = frame(30)
        let bytes = Data((first + second).utf8)
        var observation = GeminiTerminalObservation()
        switch chunking {
        case 0: observation.receive(bytes)
        case 1: observation.receive(Data(first.utf8)); observation.receive(Data(second.utf8))
        default: for byte in bytes { observation.receive(Data([byte])) }
        }
        #expect(observation.error == .unrecognizedOutput)
    }
    @Test(arguments: [0, 1])
    func repeatedFramesAndExitClearRemainValid(_ chunking: Int) {
        let bytes = Data((frame(25) + frame(25) + "\u{1b}[2J\u{1b}[HGoodbye").utf8)
        var observation = GeminiTerminalObservation()
        if chunking == 0 { observation.receive(bytes) }
        else { for byte in bytes { observation.receive(Data([byte])) } }
        #expect(observation.error == nil)
    }
    @Test func trailingAuthenticationCannotBeHiddenByAnotherClear() {
        var observation = GeminiTerminalObservation()
        observation.receive(Data((frame(25) + "\u{1b}[2J\u{1b}[HEnter the authorization code:\n\u{1b}[2J\u{1b}[HGoodbye").utf8))
        #expect(observation.error == .authenticationRequired)
    }
    private func frame(_ value: Int) -> String {
        "\u{1b}[2J\u{1b}[H╭──────╮\r\nSelect Model\r\nModel usage\r\nPro \(value)%\r\nFlash 60%\r\n(Press Esc to close)\r\n╰──────╯\r\n"
    }
}
