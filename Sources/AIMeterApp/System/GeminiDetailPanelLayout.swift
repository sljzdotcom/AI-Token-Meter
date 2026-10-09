import Foundation

enum GeminiDetailPanelLayout {
    static func height(tierCount: Int, sharedQuotaCount: Int = 0, hasCLIInfo: Bool, availableHeight: CGFloat) -> CGFloat {
        let quotaHeight = CGFloat(min(max(tierCount, 0), 2)) * 64
        let sharedQuotaHeight = CGFloat(min(max(sharedQuotaCount, 0), 2)) * 64
        let cliInfoHeight: CGFloat = hasCLIInfo ? 140 : 0
        return min(280 + quotaHeight + sharedQuotaHeight + cliInfoHeight, max(availableHeight - 16, 0))
    }
}
