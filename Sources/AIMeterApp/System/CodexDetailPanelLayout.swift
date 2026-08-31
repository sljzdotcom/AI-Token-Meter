import Foundation

enum CodexDetailPanelLayout {
    static func height(creditCount: Int, availableHeight: CGFloat) -> CGFloat {
        let count = max(creditCount, 0)
        let contentHeight = count == 0
            ? 470
            : 520 + CGFloat(max(count - 1, 0)) * 92
        return min(contentHeight, max(availableHeight - 16, 0))
    }
}
