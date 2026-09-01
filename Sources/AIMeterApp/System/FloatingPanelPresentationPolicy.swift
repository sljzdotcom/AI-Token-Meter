import AppKit

enum FloatingPanelPresentationPolicy {
    static let level = NSWindow.Level(
        rawValue: Int(CGWindowLevelForKey(.desktopIconWindow)) + 1
    )

    static let collectionBehavior: NSWindow.CollectionBehavior = [
        .canJoinAllSpaces,
        .stationary,
        .ignoresCycle,
    ]

    @MainActor
    static func apply(to panel: NSPanel) {
        panel.level = level
        panel.collectionBehavior = collectionBehavior
    }
}
