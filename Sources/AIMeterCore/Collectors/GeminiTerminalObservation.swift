import Foundation

/// Observes wire order independently of PTY read boundaries and input scheduling.
/// In particular, exit draining must not discard a complete conflicting dialog.
struct GeminiTerminalObservation {
    private var screen = GeminiTerminalScreen()
    private var pendingUTF8: [UInt8] = []
    private var expectedUTF8Length = 0
    private var receivedByteCount = 0
    private var quotaSignature: [String]?
    private(set) var error: UsageCollectionError?
    var text: String { screen.text }

    mutating func receive(_ data: Data) {
        receivedByteCount += data.count
        guard receivedByteCount < 2 * 1_024 * 1_024 else { error = .unrecognizedOutput; return }
        for byte in data {
            if pendingUTF8.isEmpty {
                if byte < 0x80 { consume(String(UnicodeScalar(byte))); continue }
                switch byte {
                case 0xc2...0xdf: expectedUTF8Length = 2
                case 0xe0...0xef: expectedUTF8Length = 3
                case 0xf0...0xf4: expectedUTF8Length = 4
                default: error = error ?? .unrecognizedOutput; continue
                }
            }
            pendingUTF8.append(byte)
            if pendingUTF8.count == expectedUTF8Length {
                if let value = String(bytes: pendingUTF8, encoding: .utf8) { consume(value) }
                else { error = error ?? .unrecognizedOutput }
                pendingUTF8.removeAll(keepingCapacity: true)
            }
        }
        observeBlockingState()
    }

    mutating func finish() {
        if !pendingUTF8.isEmpty { error = error ?? .unrecognizedOutput }
        observeBlockingState()
    }

    private mutating func consume(_ scalar: String) {
        screen.feed(scalar)
        // Ink renders the complete model box through its closing bottom-right cell.
        // Checking this write, rather than only the final screen of a read(), preserves
        // both dialogs even if clear+dialog+clear+dialog arrives in one read.
        if scalar == "╯" {
            observeBlockingState()
            let text = screen.text
            guard text.contains("Select Model"), text.contains("(Press Esc to close)") else { return }
            do {
                let metrics = try GeminiUsageParser().parse(text).geminiQuotaMetrics ?? []
                let signature = metrics.map { "\($0.label)|\($0.current)|\($0.resetDescription ?? "")" }
                if let previous = quotaSignature, signature != previous { error = error ?? .unrecognizedOutput }
                quotaSignature = signature
            } catch let failure as UsageCollectionError {
                if case .geminiUnavailable = failure, quotaSignature == nil { return }
                error = error ?? failure
            } catch { self.error = self.error ?? .unrecognizedOutput }
        } else if scalar == "\n" || scalar == ":" || scalar == "?" {
            // Authentication may be printed and cleared again inside the same read.
            observeBlockingState()
        }
    }

    private mutating func observeBlockingState() {
        if error == nil { error = GeminiTerminalProtocol.blockingError(screen.text) }
    }
}
