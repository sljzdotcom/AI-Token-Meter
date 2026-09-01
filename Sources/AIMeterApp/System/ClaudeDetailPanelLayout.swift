import Foundation

enum ClaudeDetailPanelLayout {
    static let preferredSize = CGSize(width: 390, height: 560)
    static let screenInset: CGFloat = 24

    static func size(availableHeight: CGFloat) -> CGSize {
        CGSize(
            width: preferredSize.width,
            height: min(preferredSize.height, max(availableHeight - screenInset, 0))
        )
    }
}
