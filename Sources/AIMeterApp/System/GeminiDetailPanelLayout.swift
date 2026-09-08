import Foundation

enum GeminiDetailPanelLayout {
    static func height(tierCount: Int, availableHeight: CGFloat) -> CGFloat {
        min(280 + CGFloat(min(max(tierCount, 0), 3)) * 64, max(availableHeight - 16, 0))
    }
}
