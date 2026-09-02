import AIMeterCore
import SwiftUI

extension UsageSemantic {
    var color: Color {
        switch self {
        case .normal: Color(red: 0.20, green: 0.90, blue: 0.62)
        case .warning: Color(red: 1.0, green: 0.78, blue: 0.18)
        case .critical: Color(red: 1.0, green: 0.28, blue: 0.26)
        case .stale: Color(red: 0.98, green: 0.55, blue: 0.22)
        case .unavailable: Color.secondary
        }
    }
}

extension UsageProvider {
    var symbolName: String {
        switch self {
        case .claude: "sparkles"
        case .codex: "terminal"
        case .deepSeek: "wave.3.right.circle"
        }
    }
}
