import AppKit

enum FloatingPanelPresentationRole {
    case strip
    case detail
}

enum FloatingPanelPresentationPolicy {
    private static let desktopLevel = NSWindow.Level(
        rawValue: Int(CGWindowLevelForKey(.desktopIconWindow)) + 1
    )

    static let collectionBehavior: NSWindow.CollectionBehavior = [
        .canJoinAllSpaces,
        .stationary,
        .ignoresCycle,
    ]

    static func level(for role: FloatingPanelPresentationRole) -> NSWindow.Level {
        switch role {
        case .strip:
            desktopLevel
        case .detail:
            .floating
        }
    }

    @MainActor
    static func apply(to panel: NSPanel, role: FloatingPanelPresentationRole) {
        panel.level = level(for: role)
        panel.collectionBehavior = collectionBehavior
    }
}
