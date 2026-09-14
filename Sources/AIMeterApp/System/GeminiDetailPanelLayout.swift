import Foundation

enum GeminiDetailPanelLayout {
    static func height(tierCount: Int, hasCLIInfo: Bool, availableHeight: CGFloat) -> CGFloat {
        let quotaHeight = CGFloat(min(max(tierCount, 0), 2)) * 64
        let cliInfoHeight: CGFloat = hasCLIInfo ? 140 : 0
        return min(280 + quotaHeight + cliInfoHeight, max(availableHeight - 16, 0))
    }
}
