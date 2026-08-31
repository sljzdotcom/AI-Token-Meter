import AIMeterCore
import Foundation

enum ProviderLogoStyle {
    static func opticalScale(for provider: UsageProvider) -> CGFloat {
        switch provider {
        case .claude: 1.28
        case .codex: 1.0
        case .deepSeek: 0.92
        }
    }
}
