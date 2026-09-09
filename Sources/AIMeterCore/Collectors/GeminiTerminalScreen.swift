import Foundation

/// Bounded VT screen used by the pinned Ink protocol. Cursor movement and erasure
/// replace cells; stripping ANSI would incorrectly retain old quota frames.
struct GeminiTerminalScreen {
    private var rows = [[UnicodeScalar]](repeating: [], count: 1)
    private var row = 0
    private var column = 0
    private var escape = ""
    private var inOSC = false
    private var oscEscape = false
    var text: String { rows.map { String(String.UnicodeScalarView($0)).trimmingCharacters(in: .whitespaces) }.joined(separator: "\n") }

    mutating func feed(_ text: String) {
        for scalar in text.unicodeScalars {
            if inOSC {
                if scalar.value == 7 || (oscEscape && scalar == "\\") { inOSC = false }
                oscEscape = scalar.value == 27
                continue
            }
            if !escape.isEmpty {
                escape.unicodeScalars.append(scalar)
                if escape == "\u{1b}]" { inOSC = true; escape = ""; continue }
                if escape == "\u{1b}[" { continue }
                if escape.hasPrefix("\u{1b}["), (0x40...0x7e).contains(scalar.value) {
                    applyCSI(String(escape.dropFirst(2).dropLast()), final: scalar)
                    escape = ""
                } else if escape.count > 128 || !escape.hasPrefix("\u{1b}[") { escape = "" }
                continue
            }
            switch scalar.value {
            case 27: escape = "\u{1b}"
            case 13: column = 0
            case 10: row = min(row + 1, 499); column = 0; ensureRow()
            case 8: column = max(0, column - 1)
            case 9: column = min(511, ((column / 8) + 1) * 8)
            case 0..<32: break
            default:
                ensureRow()
                while rows[row].count <= column { rows[row].append(" ") }
                rows[row][column] = scalar
                column = min(511, column + 1)
            }
        }
    }
    private mutating func ensureRow() {
        while rows.count <= row { rows.append([]) }
    }
    private mutating func applyCSI(_ args: String, final: UnicodeScalar) {
        guard !args.contains("?") && !args.contains(">") && !args.contains("<") else { return }
        let values = args.split(separator: ";", omittingEmptySubsequences: false).map { Int($0) ?? 0 }
        let first = values.first ?? 0
        let amount = max(first, 1)
        switch final {
        case "A": row = max(0, row - amount)
        case "B": row = min(499, row + amount)
        case "C": column = min(511, column + amount)
        case "D": column = max(0, column - amount)
        case "G": column = min(511, amount - 1)
        case "H", "f": row = min(499, amount - 1); column = min(511, max(1, values.dropFirst().first ?? 1) - 1)
        case "J":
            if first == 2 || first == 3 { rows = [[]] }
            else if first == 0 {
                ensureRow(); rows[row] = Array(rows[row].prefix(column)); rows = Array(rows.prefix(row + 1))
            }
        case "K":
            ensureRow()
            if first == 2 { rows[row] = [] }
            else if first == 0 { rows[row] = Array(rows[row].prefix(column)) }
            else if first == 1 { for index in 0..<min(column + 1, rows[row].count) { rows[row][index] = " " } }
        default: break // Styling, terminal queries and mode toggles do not alter cells.
        }
        ensureRow()
    }
}

enum GeminiTerminalProtocol {
    static func isReady(_ text: String) -> Bool {
        blockingError(text) == nil && !text.contains("Select Model")
            && text.contains("Type your message or @path/to/file")
            && text.contains("? for shortcuts")
    }
    static func blockingError(_ text: String) -> UsageCollectionError? {
        let lower = text.lowercased()
        if lower.contains("enter the authorization code") || lower.contains("accounts.google.com/o/oauth")
            || lower.contains("sign in with google") || lower.contains("login with google") {
            return .authenticationRequired
        }
        if ["select a theme", "choose a theme", "trust this", "trust the", "do you trust", "select authentication", "how would you like to authenticate", "welcome to gemini", "use gemini api key", "vertex ai"].contains(where: lower.contains) {
            return .geminiUnavailable("Gemini CLI requires interactive setup or uses an unsupported authentication mode")
        }
        return nil
    }
}
