import AppKit

@MainActor
final class ActiveSpaceChangeObserver {
    private let center: NotificationCenter
    private var token: NSObjectProtocol?

    init(
        center: NotificationCenter = NSWorkspace.shared.notificationCenter,
        name: Notification.Name = NSWorkspace.activeSpaceDidChangeNotification,
        onChange: @escaping @MainActor () -> Void
    ) {
        self.center = center
        token = center.addObserver(forName: name, object: nil, queue: .main) { _ in
            MainActor.assumeIsolated {
                onChange()
            }
        }
    }

    func invalidate() {
        guard let token else { return }
        center.removeObserver(token)
        self.token = nil
    }

    isolated deinit {
        if let token {
            center.removeObserver(token)
        }
    }
}
